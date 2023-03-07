use std::net::SocketAddr;

use crate::peer::{NodeDesc, PeerError, PeerResult};
use crate::peer::wire_protocol::{ProtocolMessage, VerAckMessage, VersionMessage};

pub struct ConversationAction {
    pub message: Option<ProtocolMessage>,
    pub topic_finished: bool,
}

pub trait ConversationTopicHandler<O> {
    fn initial_message(&mut self) -> ConversationAction;
    fn on_message(&mut self, message: ProtocolMessage) -> PeerResult<ConversationAction>;
    /// the result of this conversation, once it's finished
    fn outcome(self) -> PeerResult<O>;
}


/// Handshake:
/// - create TCP connection on host:port (127.0.0.1:18334)
///
/// NodeA <---> NodeB
///    __version__ message, replied by __verack__ message (both)
///
/// - send __version__ message
/// - expect __verack__ message
/// - expect __version__ message
/// - respond with __verack__ message
/// => connected
pub struct HandshakeInitConversationTopic {
    me: NodeDesc,
    remote_addr: SocketAddr,
    version_msg_sent: bool,
    version_ack_msg_received: bool,
    version_msg_received: Option<VersionMessage>,
}

impl HandshakeInitConversationTopic {
    pub fn new(me: &NodeDesc, remote_addr: SocketAddr) -> Self {
        HandshakeInitConversationTopic {
            me: me.clone(),
            remote_addr,
            version_msg_sent: false,
            version_ack_msg_received: false,
            version_msg_received: None,
        }
    }
}

impl ConversationTopicHandler<NodeDesc> for HandshakeInitConversationTopic {
    fn initial_message(&mut self) -> ConversationAction {
        let message = ProtocolMessage::Version(VersionMessage::new(self.remote_addr, &self.me));
        self.version_msg_sent = true;
        ConversationAction {
            message: Some(message),
            topic_finished: false,
        }
    }

    fn on_message(&mut self, message: ProtocolMessage) -> PeerResult<ConversationAction> {
        match message {
            ProtocolMessage::Version(m) => {
                self.version_msg_received = Some(m);
                let reply_msg = ProtocolMessage::VerAck(VerAckMessage::default());
                let topic_finished = self.version_msg_sent && self.version_ack_msg_received;
                Ok(ConversationAction {
                    message: Some(reply_msg),
                    topic_finished,
                })
            }
            ProtocolMessage::VerAck(_) => {
                self.version_ack_msg_received = true;
                if !self.version_msg_sent {
                    Err(PeerError::from("Protocol error: received a 'verack', but no 'version' was sent yet"))
                } else {
                    let topic_finished = self.version_msg_received.is_some();
                    Ok(ConversationAction {
                        message: None,
                        topic_finished,
                    })
                }
            }
        }
    }

    fn outcome(self) -> PeerResult<NodeDesc> {
        match self.version_msg_received {
            None => Err(PeerError::from("should have a version message from remote node")),
            Some(m) => Ok(NodeDesc::from(m))
        }
    }
}
