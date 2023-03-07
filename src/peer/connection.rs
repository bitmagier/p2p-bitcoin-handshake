use std::net::SocketAddr;

use tokio::io;
use tokio::io::{AsyncReadExt, AsyncWriteExt, ReadHalf, WriteHalf};
use tokio::net::TcpStream;

use crate::peer::buffer::IOBuffer;
use crate::peer::conversation::ConversationTopicHandler;
use crate::peer::PeerResult;
use crate::peer::wire_protocol::{MessageParseOutcome, RawMessage};

pub struct NodeConnection {
    read: ReadHalf<TcpStream>,
    write: WriteHalf<TcpStream>,
    pub local_addr: SocketAddr,
}

impl NodeConnection {
    pub async fn new(addr: SocketAddr) -> io::Result<Self> {
        let socket = TcpStream::connect(addr).await?;
        let local_addr = socket.local_addr()?;
        //TODO is the split necessary?
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

        'outer: loop {
            let mut buffer = IOBuffer::default();
            match self.read.read(buffer.expose_writable_part()).await? {
                0 => break, //TODO return Err
                n => {
                    buffer.register_added_content(n);
                    log::debug!("received {n} bytes, new buffer pos is {}", buffer.content().len());

                    'inner: loop {
                        log::debug!("trying to consume message, buffer pos is {}", buffer.content().len());
                        match RawMessage::try_consume_message(&mut buffer) {
                            Ok(MessageParseOutcome::Message(raw_message)) => {
                                let received_message = raw_message.to_protocol_message()?; //TODO no error
                                log::debug!("received {:?}", received_message);
                                let handler_response = handler.on_message(received_message)?;
                                if let Some(response_message) = handler_response.message {
                                    log::debug!("sending {:?}", response_message);
                                    self.write.write_all(&response_message.to_bytes()).await?;
                                }
                                if handler_response.topic_finished {
                                    break 'outer;
                                }
                            }
                            Ok(MessageParseOutcome::SkippedMessage) => {
                            }
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
