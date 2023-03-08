use crate::error::PeerResult;
use crate::wire_protocol::messages::ProtocolMessage;

pub struct ConversationAction {
    pub message: Option<ProtocolMessage>,
    pub topic_finished: bool,
}

impl ConversationAction {
    pub fn nop() -> Self {
        ConversationAction {
            message: None,
            topic_finished: false,
        }
    }
}

pub trait ConversationTopicHandler {
    type Outcome;

    fn initial_action(&mut self) -> ConversationAction;
    fn on_message(&mut self, message: ProtocolMessage) -> PeerResult<ConversationAction>;
    /// the result of this conversation, once it's finished
    fn outcome(self) -> PeerResult<Self::Outcome>;
}



