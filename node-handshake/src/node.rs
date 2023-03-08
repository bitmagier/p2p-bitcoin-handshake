use std::collections::HashMap;
use std::net::SocketAddr;

use net::error::PeerResult;
use net::wire_protocol::connection::NodeConnection;
use net::wire_protocol::handshake::HandshakeInitConversationTopic;
use net::wire_protocol::node::NodeDesc;

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

    pub async fn connect_with(&mut self, remote_addr: SocketAddr) -> PeerResult<NodeDesc> {
        let mut connection = NodeConnection::new(self.node_desc.chain, remote_addr).await?;

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
