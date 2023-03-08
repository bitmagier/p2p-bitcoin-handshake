use std::net::SocketAddr;

use crate::conversation::{ConversationAction, ConversationTopicHandler};
use crate::error::{PeerError, PeerResult};
use crate::wire_protocol::messages::{PongMessage, ProtocolMessage, VerackMessage, VersionMessage};
use crate::wire_protocol::node::NodeDesc;

/// Handshake:
///
/// NodeA <---> NodeB
///    __version__ message, replied by __verack__ message (both)
///
/// - create TCP connection
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

impl ConversationTopicHandler for HandshakeInitConversationTopic {
    type Outcome = NodeDesc;

    fn initial_action(&mut self) -> ConversationAction {
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
                let reply_msg = ProtocolMessage::Verack(VerackMessage::new(self.me.chain));
                let topic_finished = self.version_msg_sent && self.version_ack_msg_received;
                Ok(ConversationAction {
                    message: Some(reply_msg),
                    topic_finished,
                })
            }
            ProtocolMessage::Verack(_) => {
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
            ProtocolMessage::Ping(_) => {
                Ok(ConversationAction {
                    message: Some(ProtocolMessage::Pong(PongMessage::new(self.me.chain))),
                    topic_finished: false,
                })
            }
            ProtocolMessage::Pong(_) => {
                Ok(ConversationAction::nop())
            }
        }
    }

    fn outcome(self) -> PeerResult<NodeDesc> {
        match self.version_msg_received {
            None => Err(PeerError::from("should have a version message from remote node")),
            Some(msg) => Ok(
                NodeDesc {
                    chain: self.me.chain,
                    protocol_version: msg.protocol_version,
                    services: msg.services.clone(),
                    sub_ver: msg.sub_ver.clone(),
                    start_height: msg.start_height,
                }
            )
        }
    }
}
