use std::{ascii, io};
use std::net::SocketAddr;
use std::ops::BitAnd;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{Rng, thread_rng};
use sha2::{Digest, Sha256};
use sha2::digest::FixedOutput;
use strum::{EnumIter, IntoEnumIterator};

use crate::peer::{NodeDesc, PeerError, PeerResult};
use crate::peer::buffer::{ByteBufferComposer, ByteBufferParser};

#[derive(Debug)]
pub enum ProtocolMessage {
    Version(VersionMessage),
    VerAck(VerAckMessage),
}

impl ProtocolMessage {
    pub fn to_bytes(self) -> Vec<u8> {
        RawMessage::from(self).to_bytes()
    }
}

impl TryFrom<RawMessage> for ProtocolMessage {
    type Error = PeerError;

    fn try_from(m: RawMessage) -> Result<Self, Self::Error> {
        m.to_protocol_message()
    }
}

/// https://en.bitcoin.it/wiki/Protocol_documentation#version
///
/// size | field        | type     | description
/// ---  | -----        | ----     | ------------
/// 4    | version      | i32      | Identifies protocol version being used by the node
/// 8    | services     | u64      | bitfield of features to be enabled for this connection
/// 8    | timestamp    | i64      | standard UNIX timestamp in seconds
/// 26   | addr_recv    | net_addr | The network address of the node receiving this message
/// 26   | addr_from    | net_addr | Field can be ignored.
/// 8    | nonce        | u64      | Node random nonce
/// ?    | user_agent   | var_str  | User Agent (0x00 if string is 0 bytes long)
/// 4    | start_height | i32      | The last block received by the emitting node
/// 1    | relay        | bool     | Whether the remote peer should announce relayed transactions or not, see BIP 0037
#[derive(Clone, Debug)]
pub struct VersionMessage {
    pub protocol_version: i32,
    pub services: NodeServiceSet,
    pub timestamp: i64,
    pub addr_recv: SocketAddr,
    pub sub_ver: String,
    pub start_height: i32,
}

impl VersionMessage {
    pub fn new(addr_recv: SocketAddr, me: &NodeDesc) -> Self {
        let timestamp = match SystemTime::now().duration_since(UNIX_EPOCH) {
            Ok(v) => v.as_secs() as i64,
            Err(_) => panic!("SystemTime too low")
        };

        VersionMessage {
            protocol_version: me.protocol_version,
            services: me.services.clone(),
            timestamp,
            addr_recv,
            sub_ver: me.sub_ver.clone(),
            start_height: me.start_height,
        }
    }

    fn from_raw_message(raw: RawMessage) -> PeerResult<Self> {
        let mut parser = ByteBufferParser::new(&raw.payload);

        let protocol_version = parser.read_i32_le()?;
        let services_mask = parser.read_u64_le()?;
        let services = NodeServiceSet::from_bitmask(services_mask);
        let timestamp = parser.read_i64_le()?;
        let (_, addr_recv) = parser.parse_net_addr()?;
        parser.skip_bytes(26)?;
        parser.skip_bytes(8)?;
        //let sub_ver = parser.read_var_string()?;
        //let start_height = parser.read_i32_le()?;

        Ok(VersionMessage {
            protocol_version,
            services,
            timestamp,
            addr_recv,
            sub_ver: "".to_string(), // TODO
            start_height: 1, // TODO
        })
    }

    fn to_raw_message(self) -> RawMessage {
        let mut rng = thread_rng();
        let mut composer = ByteBufferComposer::new();

        composer.append(&self.protocol_version.to_le_bytes());
        composer.append(&self.services.as_bitmask().to_le_bytes());
        composer.append(&self.timestamp.to_le_bytes());
        composer.append_net_addr(&self.services, &self.addr_recv);
        composer.append(&[0x0_u8; 26]);
        composer.append(&rng.gen::<u64>().to_le_bytes());
        composer.append(&[0]);  // TODO add own version string in ASCII var_string format
        composer.append(&self.start_height.to_le_bytes());
        composer.append(&[0]);

        RawMessage::new(THIS_NET_MAGIC_VALUE, Command::Version, composer.result())
    }
}

/// _A "verack" packet shall be sent if the version packet was accepted._
#[derive(Default, Debug)]
pub struct VerAckMessage {}

impl VerAckMessage {
    fn to_raw_message(&self) -> RawMessage {
        RawMessage::new(THIS_NET_MAGIC_VALUE, Command::VerAck, vec![])
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NodeServiceSet(pub Vec<NodeService>);

impl NodeServiceSet {
    pub fn as_bitmask(&self) -> u64 {
        let mut bitset = 0x0_u64;
        for bit in self.0.iter() {
            bitset = bitset.bitand(bit.as_u64());
        }
        bitset
    }

    pub fn from_bitmask(mask: u64) -> Self {
        let mut services = vec![];

        for e in NodeService::iter() {
            if mask.bitand(e.as_u64()) != 0 {
                services.push(e);
            }
        }

        NodeServiceSet(services)
    }
}


#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u64)]
#[derive(EnumIter)]
pub enum NodeService {
    NodeNetwork = 0x1, // bit mask value
    // ...
}

impl NodeService {
    pub fn as_u64(self) -> u64 {
        self as u64
    }
}


#[derive(EnumIter)]
pub enum Command {
    Version,
    VerAck,
}

impl Command {
    // ASCII string identifying the packet content, NULL padded (non-NULL padding results in packet rejected)
    fn as_bytes(&self) -> &[u8; 12] {
        match self {
            Command::Version => b"version\0\0\0\0\0",
            Command::VerAck => b"verack\0\0\0\0\0\0",
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
        Err(PeerError::from(format!("'{}' ({:?}) do not represent a known bitcoin command", printable, value)))
    }
}


/// Almost all integers are encoded in little endian. Only IP or port number are encoded big endian.
pub(super) struct RawMessage {
    pub magic: u32,
    pub command: Command,
    pub payload: Vec<u8>,
}

const THIS_NET_MAGIC_VALUE: u32 = MAGIC_VALUE_REGTEST;
// const MAGIC_VALUE_TESTNET3: u32 = 0x0709110B;
const MAGIC_VALUE_REGTEST: u32 = 0xDAB5BFFA;

impl<'a> RawMessage {
    pub fn new(magic: u32, command: Command, payload: Vec<u8>) -> Self {
        RawMessage {
            magic,
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
        c.append(&self.magic.to_le_bytes());
        c.append(self.command.as_bytes());
        c.append(&(self.payload.len() as u32).to_le_bytes());
        let checksum = sha256(&sha256(self.payload.as_slice()));
        c.append(&checksum[..4]);
        c.append(&self.payload);
        c.result()
    }

    /// returns the buffer-length of the deserialized message in bytes and the corresponding message object
    pub fn parse(buffer: &[u8]) -> PeerResult<(usize, RawMessage)> {
        if !Self::contains_a_complete_message(buffer)? {
            return Err(io::Error::from(io::ErrorKind::UnexpectedEof))?;
        }

        let mut parser = ByteBufferParser::new(buffer);

        let magic = parser.read_u32_le()?;
        let command = Command::try_from(parser.read(12)?)?;
        let payload_len = parser.read_u32_le()?;
        let checksum: [u8; 4] = parser.read(4)?.try_into().unwrap();
        let payload = parser.read(payload_len as usize)?.to_vec();

        Self::verify_checksum(&payload, &checksum)?;

        let message = RawMessage {
            magic,
            command,
            payload,
        };

        Ok((parser.pos(), message))
    }

    /// checks if these bytes are describing a complete message (regarding length and size)
    /// No checksum or content check is done here
    pub fn contains_a_complete_message(buffer: &[u8]) -> PeerResult<bool> {
        const HEADER_LEN: usize = 4 + 12 + 4 + 4;

        let mut parser = ByteBufferParser::new(buffer);

        if parser.remaining() < HEADER_LEN {
            return Ok(false);
        }

        let magic = parser.read_u32_le()?;
        if magic != THIS_NET_MAGIC_VALUE {
            return Err(PeerError::from("magic differs (indicates a different network)"));
        }

        parser.skip_bytes(12)?;
        let payload_len = parser.read_u32_le()?;
        let complete = HEADER_LEN + payload_len as usize <= parser.remaining();

        Ok(complete)
    }

    pub fn to_protocol_message(self) -> PeerResult<ProtocolMessage> {
        if self.magic != THIS_NET_MAGIC_VALUE {
            return Err(PeerError::from(format!("unrecognized net magic value: {}", self.magic)));
        }

        let message = match self.command {
            Command::Version => ProtocolMessage::Version(VersionMessage::from_raw_message(self)?),
            Command::VerAck => ProtocolMessage::VerAck(VerAckMessage::default())
        };
        Ok(message)
    }

    fn verify_checksum(payload: &[u8], checksum: &[u8]) -> PeerResult<()> {
        if *checksum == sha256(&sha256(payload))[..4] {
            Ok(())
        } else {
            Err(PeerError::from("checksum error"))
        }
    }
}

impl From<ProtocolMessage> for RawMessage {
    fn from(message: ProtocolMessage) -> Self {
        match message {
            ProtocolMessage::Version(message) => RawMessage::from(message),
            ProtocolMessage::VerAck(message) => RawMessage::from(message)
        }
    }
}

impl From<VersionMessage> for RawMessage {
    fn from(m: VersionMessage) -> Self {
        m.to_raw_message()
    }
}

impl From<VerAckMessage> for RawMessage {
    fn from(m: VerAckMessage) -> Self {
        m.to_raw_message()
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

    #[rstest]
    #[case(b"hello world", & hex ! ("b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9")[..])]
    #[case(b"What a wonderful day!", & hex ! ("99645b38ff103516a86ade43cffa0116d31f6136a83f99d4fa5b6c19e29c20cf"))]
    fn test_message_sha256(#[case] input: &[u8], #[case] expected_result: &[u8]) {
        assert_eq!(&sha256(input), expected_result);
    }
}
