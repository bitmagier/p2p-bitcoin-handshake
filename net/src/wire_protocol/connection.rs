use std::net::SocketAddr;

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

use crate::conversation::ConversationTopicHandler;
use crate::error::{PeerError, PeerResult};
use crate::wire_protocol::buffer::IOBuffer;
use crate::wire_protocol::node::Chain;
use crate::wire_protocol::raw_message::{MessageParseOutcome, RawMessage};

pub struct NodeConnection {
    chain: Chain,
    socket: TcpStream,
}

impl NodeConnection {
    pub async fn new(chain: Chain, addr: SocketAddr) -> io::Result<Self> {
        let socket = TcpStream::connect(addr).await?;
        Ok(NodeConnection { chain, socket })
    }

    pub async fn proceed_conversation<H: ConversationTopicHandler>(&mut self, handler: H) -> PeerResult<H::Outcome> {
        let mut handler = handler;
        let initial_action = handler.initial_action();
        if let Some(message) = initial_action.message {
            log::debug!("sending {:?}", message);
            self.socket.write_all(&message.to_bytes()).await?
        }
        if initial_action.topic_finished {
            return handler.outcome();
        }

        'outer: loop {
            let mut buffer = IOBuffer::default();
            match self.socket.read(buffer.expose_writable_part()).await? {
                0 => return Err(PeerError::from("Remote node hung up")),
                n => {
                    buffer.register_added_content(n);
                    log::trace!("received {n} bytes, new buffer pos is {}", buffer.content().len());

                    'inner: loop {
                        log::trace!("trying to consume message, buffer pos is {}", buffer.content().len());
                        match RawMessage::try_consume_message(&mut buffer, self.chain) {
                            Ok(MessageParseOutcome::Message(raw_message)) => {
                                let received_message = raw_message.to_protocol_message()?;

                                log::debug!("received {:?}", received_message);
                                let handler_response = handler.on_message(received_message)?;
                                if let Some(response_message) = handler_response.message {
                                    log::debug!("sending {:?}", response_message);
                                    self.socket.write_all(&response_message.to_bytes()).await?;
                                }
                                if handler_response.topic_finished {
                                    break 'outer;
                                }
                            }
                            Ok(MessageParseOutcome::SkippedMessage) => {}
                            Ok(MessageParseOutcome::NoMessage) => {
                                // consistent state but no complete message available
                                break 'inner;
                            }
                            Err(err) => {
                                log::warn!("ignoring incoming message, because we couldn't decode it: {}", err)
                            }
                        }
                    }
                }
            }
        }

        handler.outcome()
    }
}
