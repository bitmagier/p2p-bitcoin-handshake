use std::ops::BitAnd;

use strum::{EnumIter, IntoEnumIterator};

use crate::error::PeerError;

#[derive(Clone, Debug)]
pub struct NodeDesc {
    pub chain: Chain,
    pub protocol_version: i32,
    pub services: NodeServiceSet,
    pub sub_ver: String,
    pub start_height: i32,
}

#[derive(Copy, Clone, Debug, PartialEq, EnumIter)]
pub enum Chain {
    Regtest,
    Testnet3,
}

impl Chain {
    pub fn magic_value(&self) -> u32 {
        match self {
            Chain::Regtest => 0xDAB5BFFA,
            Chain::Testnet3 => 0x0709110B
        }
    }
}

impl TryFrom<u32> for Chain {
    type Error = PeerError;

    fn try_from(magic_value: u32) -> Result<Self, Self::Error> {
        for c in Self::iter() {
            if c.magic_value() == magic_value {
                return Ok(c);
            }
        }
        Err(PeerError::from(format!("No chain known having magic value {}", magic_value)))
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

