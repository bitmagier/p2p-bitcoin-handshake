use std::net::SocketAddr;

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;

use crate::peer::buffer::IOBuffer;
use crate::peer::conversation::ConversationTopicHandler;
use crate::peer::PeerResult;
use crate::peer::wire_protocol::{ProtocolMessage, RawMessage};

pub struct NodeConnection {
    read: ReadHalf<TcpStream>,
    write: WriteHalf<TcpStream>,
    pub local_addr: SocketAddr,
}

impl NodeConnection {
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let socket = TcpStream::connect(addr).await?;
        let local_addr = socket.local_addr()?;
        let (read, write) = io::split(socket);

        Ok(NodeConnection { read, write, local_addr })
    }

    pub async fn proceed_conversation<O, T: ConversationTopicHandler<O>>(&mut self, handler: T) -> PeerResult<O> {
        let mut handler = handler;
        let initial_action = handler.initial_action();
        if let Some(message) = initial_action.message {
            log::debug!("sending {:?}", message);
            self.write.write_all(&message.to_bytes()).await?
        }
        if initial_action.topic_finished {
            return handler.outcome();
        }

        loop {
            let mut buffer = IOBuffer::default();
            match self.read.read(buffer.expose_writable_part()).await? {
                0 => break,
                n => {
                    buffer.register_added_content(n);
                    if RawMessage::contains_a_complete_message(buffer.content())? {
                        let (size, received_message) = RawMessage::parse(buffer.content())?;
                        buffer.shift_left(size);

                        match ProtocolMessage::try_from(received_message) {
                            Ok(received_message) => {
                                log::debug!("received {:?}", received_message);
                                let handler_response = handler.on_message(received_message)?;
                                if let Some(response_message) = handler_response.message {
                                    log::debug!("sending {:?}", response_message);
                                    self.write.write_all(&response_message.to_bytes()).await?;
                                }
                                if handler_response.topic_finished {
                                    break;
                                }
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
