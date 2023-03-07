use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter};
use std::net::SocketAddr;

use crate::peer::connection::NodeConnection;
use crate::peer::conversation::HandshakeInitConversationTopic;
use crate::peer::wire_protocol::{NodeServiceSet, VersionMessage};

pub mod wire_protocol;
mod buffer;
mod connection;
mod conversation;

type PeerResult<T> = Result<T, PeerError>;

#[derive(Debug)]
pub struct PeerError {
    pub msg: String,
}

impl Display for PeerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.msg)
    }
}

impl std::error::Error for PeerError {}

impl From<String> for PeerError {
    fn from(msg: String) -> Self {
        PeerError { msg }
    }
}

impl From<&str> for PeerError {
    fn from(msg: &str) -> Self {
        PeerError::from(msg.to_string())
    }
}

impl From<std::io::Error> for PeerError {
    fn from(value: std::io::Error) -> Self {
        Self::from(format!("{}", value))
    }
}

#[derive(Clone)]
pub struct NodeDesc {
    pub protocol_version: i32,
    pub services: NodeServiceSet,
    pub sub_ver: String,
    pub start_height: i32,
}

impl From<VersionMessage> for NodeDesc {
    fn from(msg: VersionMessage) -> Self {
        NodeDesc {
            protocol_version: msg.protocol_version,
            services: msg.services.clone(),
            sub_ver: msg.sub_ver.clone(),
            start_height: msg.start_height,
        }
    }
}


pub struct Node {
    node_desc: NodeDesc,
    remote_nodes: HashMap<SocketAddr, NodeConnection>,
}

impl Node {
    pub fn new(node_desc: NodeDesc) -> Self {
        Node {
            node_desc,
            remote_nodes: HashMap::new(),
        }
    }

    // TODO add timeout
    pub async fn connect_with(&mut self, remote_addr: SocketAddr) -> PeerResult<NodeDesc> {
        let mut connection = NodeConnection::new(remote_addr).await?;

        let result = connection.proceed_conversation(
            HandshakeInitConversationTopic::new(&self.node_desc, remote_addr)
        ).await?;

        self.remote_nodes.insert(remote_addr, connection);

        Ok(result)
    }

    pub fn close_connection(&mut self, remote: SocketAddr) {
        // connection is closed by tokio when socket is dropped
        self.remote_nodes.remove(&remote);
    }
}
