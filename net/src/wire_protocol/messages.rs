use std::net::SocketAddr;
use std::time::{SystemTime, UNIX_EPOCH};

use rand::{Rng, RngCore, thread_rng};

use crate::error::PeerResult;
use crate::wire_protocol::buffer::{ByteBufferComposer, ByteBufferParser};
use crate::wire_protocol::node::{Chain, NodeDesc, NodeServiceSet};
use crate::wire_protocol::raw_message::{Command, RawMessage};

#[derive(Debug)]
pub enum ProtocolMessage {
    Version(VersionMessage),
    Verack(VerackMessage),
    Ping(PingMessage),
    Pong(PongMessage),
}

impl ProtocolMessage {
    pub fn to_bytes(self) -> Vec<u8> {
        RawMessage::from(self).to_bytes()
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
    pub chain: Chain,
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
            chain: me.chain,
            protocol_version: me.protocol_version,
            services: me.services.clone(),
            timestamp,
            addr_recv,
            sub_ver: me.sub_ver.clone(),
            start_height: me.start_height,
        }
    }

    pub(super) fn from_raw_message(raw: RawMessage) -> PeerResult<Self> {
        let mut parser = ByteBufferParser::new(&raw.payload);

        let protocol_version = parser.read_i32_le()?;
        let services_mask = parser.read_u64_le()?;
        let services = NodeServiceSet::from_bitmask(services_mask);
        let timestamp = parser.read_i64_le()?;
        let (_, addr_recv) = parser.parse_net_addr()?;
        parser.skip_bytes(26)?;
        parser.skip_bytes(8)?;

        Ok(VersionMessage {
            chain: raw.chain,
            protocol_version,
            services,
            timestamp,
            addr_recv,
            sub_ver: "".to_string(), // TODO let sub_ver = parser.read_var_string()?;
            start_height: 1, // TODO let start_height = parser.read_i32_le()?;
        })
    }

    pub(super) fn to_raw_message(self) -> RawMessage {
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

        RawMessage::new(self.chain, Command::Version, composer.result())
    }
}

/// _A "verack" packet shall be sent if the version packet was accepted._
#[derive(Debug)]
pub struct VerackMessage {
    chain: Chain,
}

impl VerackMessage {
    pub fn new(chain: Chain) -> Self {
        VerackMessage { chain }
    }
    pub fn to_raw_message(self) -> RawMessage {
        RawMessage::new(self.chain, Command::Verack, vec![])
    }
}

#[allow(dead_code)]
#[derive(Debug)]
pub struct PingMessage {
    chain: Chain,
}

impl PingMessage {
    pub fn new(chain: Chain) -> Self {
        PingMessage { chain }
    }
    pub fn to_raw_message(self) -> RawMessage {
        unimplemented!() // not needed for handshake
    }
}

#[derive(Debug)]
pub struct PongMessage {
    chain: Chain,
}

impl PongMessage {
    pub fn new(chain: Chain) -> Self {
        PongMessage { chain }
    }
    pub fn to_raw_message(self) -> RawMessage {
        let mut rng = thread_rng();
        RawMessage::new(self.chain, Command::Pong, rng.next_u64().to_le_bytes().to_vec())
    }
}
