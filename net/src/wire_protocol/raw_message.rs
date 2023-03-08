use std::ascii;

use sha2::{Digest, Sha256};
use sha2::digest::FixedOutput;
use strum::{EnumIter, IntoEnumIterator};

use crate::error::{PeerError, PeerResult};
use crate::wire_protocol::buffer::{ByteBufferComposer, ByteBufferParser, IOBuffer};
use crate::wire_protocol::messages::{PingMessage, PongMessage, ProtocolMessage, VerackMessage, VersionMessage};
use crate::wire_protocol::node::Chain;

#[derive(Debug, EnumIter)]
pub enum Command {
    Version,
    Verack,
    Ping,
    Pong,
}

impl Command {
    // ASCII string identifying the packet content, NULL padded (non-NULL padding results in packet rejected)
    fn as_bytes(&self) -> &[u8; 12] {
        match self {
            Command::Version => b"version\0\0\0\0\0",
            Command::Verack => b"verack\0\0\0\0\0\0",
            Command::Ping => b"ping\0\0\0\0\0\0\0\0",
            Command::Pong => b"pong\0\0\0\0\0\0\0\0",
        }
    }
}

impl TryFrom<&[u8]> for Command {
    type Error = PeerError;

    fn try_from(value: &[u8]) -> PeerResult<Self> {
        fn format_byte_array_as_string(bytes: &[u8]) -> String {
            let mut result = String::new();
            for &c in bytes {
                result.push_str(std::str::from_utf8(&ascii::escape_default(c).collect::<Vec<u8>>()).unwrap())
            }
            result
        }

        for command in Command::iter() {
            if command.as_bytes() == value {
                return Ok(command);
            }
        }
        let printable = format_byte_array_as_string(value);
        Err(
            PeerError::from(format!("'{}' ({:?}) do not represent a known bitcoin command", printable, value))
        )
    }
}


/// Almost all integers are encoded in little endian. Only IP or port number are encoded big endian.
pub struct RawMessage {
    pub chain: Chain,
    pub command: Command,
    pub payload: Vec<u8>,
}

impl<'a> RawMessage {
    pub fn new(chain: Chain, command: Command, payload: Vec<u8>) -> Self {
        RawMessage {
            chain,
            command,
            payload,
        }
    }

    /// Message structure (see https://en.bitcoin.it/wiki/Protocol_documentation#Message_structure)
    ///
    /// size | field    | type     | description
    /// ---  | -----    | ----     | ------------
    /// 4    | magic    | u32      | Magic value indicating message origin network, and used to seek to next message when stream state is unknown
    /// 12   | command  | [u8; 12] | ASCII string identifying the packet content, NULL padded (non-NULL padding results in packet rejected)
    /// 4    | length   | u32      | Length of payload in number of bytes
    /// 4    | checksum | u32      | First 4 bytes of sha256(sha256(payload))
    /// ?    | payload  | Vec<u8>  | The actual data
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut c = ByteBufferComposer::new();
        c.append(&self.chain.magic_value().to_le_bytes());
        c.append(self.command.as_bytes());
        c.append(&(self.payload.len() as u32).to_le_bytes());
        let checksum = sha256(&sha256(self.payload.as_slice()));
        c.append(&checksum[..4]);
        c.append(&self.payload);
        c.result()
    }

    /// returns the buffer-length of the deserialized message in bytes and the corresponding message object
    pub fn try_consume_message(buffer: &mut IOBuffer, expected_chain: Chain) -> PeerResult<MessageParseOutcome> {
        let mut parser = ByteBufferParser::new(buffer.content());

        const HEADER_LEN: usize = 4 + 12 + 4 + 4;
        if parser.remaining() < HEADER_LEN {
            return Ok(MessageParseOutcome::NoMessage);
        }

        let magic = parser.read_u32_le()?;
        let chain = Chain::try_from(magic)?;
        if chain != expected_chain {
            return Err(PeerError::from(format!("expected network chain {expected_chain:?}, but got a message from {chain:?}")));
        }

        let command_string = parser.read(12).unwrap();
        log::debug!("receiving command {}", String::from_utf8(Vec::from(command_string)).unwrap());
        let payload_len = parser.read_u32_le()? as usize;
        let checksum: [u8; 4] = parser.read(4)?.try_into().unwrap();

        if parser.remaining() < payload_len {
            return Ok(MessageParseOutcome::NoMessage);
        }

        let payload = parser.read(payload_len as usize)?.to_vec();
        Self::verify_checksum(&payload, &checksum)?;

        let command = match Command::try_from(command_string) {
            Ok(command) => command,
            Err(err) => {
                buffer.shift_left(parser.pos());
                log::warn!("{}", err);
                return Ok(MessageParseOutcome::SkippedMessage);
            }
        };

        buffer.shift_left(parser.pos());

        Ok(MessageParseOutcome::Message(
            RawMessage {
                chain,
                command,
                payload,
            }))
    }

    pub fn to_protocol_message(self) -> PeerResult<ProtocolMessage> {
        match self.command {
            Command::Version => Ok(ProtocolMessage::Version(VersionMessage::from_raw_message(self)?)),
            Command::Verack => Ok(ProtocolMessage::Verack(VerackMessage::new(self.chain))),
            Command::Ping => Ok(ProtocolMessage::Ping(PingMessage::new(self.chain))),
            Command::Pong => Ok(ProtocolMessage::Pong(PongMessage::new(self.chain))),
        }
    }

    fn verify_checksum(payload: &[u8], checksum: &[u8]) -> PeerResult<()> {
        if *checksum == sha256(&sha256(payload))[..4] {
            Ok(())
        } else {
            Err(PeerError::from("checksum error"))
        }
    }
}

pub enum MessageParseOutcome {
    Message(RawMessage),
    SkippedMessage,
    NoMessage,
}

impl From<ProtocolMessage> for RawMessage {
    fn from(message: ProtocolMessage) -> Self {
        match message {
            ProtocolMessage::Version(message) => message.to_raw_message(),
            ProtocolMessage::Verack(message) => message.to_raw_message(),
            ProtocolMessage::Ping(message) => message.to_raw_message(),
            ProtocolMessage::Pong(message) => message.to_raw_message(),
        }
    }
}

fn sha256(input: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::default();
    hasher.update(input);
    hasher.finalize_fixed().into()
}

#[cfg(test)]
mod test {
    use hex_literal::hex;
    use rstest::*;

    use crate::peer::wire_protocol::sha256;
    use crate::wire_protocol::messages::sha256;
    use crate::wire_protocol::raw_message::sha256;
    use crate::wire_protocol::sha256;

    #[rstest]
    #[case(b"hello world", & hex ! ("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")[..])]
    #[case(b"What a wonderful day!", & hex ! ("99645b38ff103516a86ade43cffa0116d31f6136a83f99d4fa5b6c19e29c20cf"))]
    fn test_message_sha256(#[case] input: &[u8], #[case] expected_result: &[u8]) {
        assert_eq!(&sha256(input), expected_result);
    }
}
