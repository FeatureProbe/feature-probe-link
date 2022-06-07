use crate::accepter::Accepter;
use bytes::{Bytes, BytesMut};
use futures::stream::StreamExt;
use server_base::codec::{DecodeError, EncodeError, Message};
use server_base::proto;
use server_base::tokio;
use server_base::{ConnContext, LifeCycle, Protocol, RecvMessage, SendMessage};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_tungstenite::accept_async;
use tungstenite::Message as WsMessage;

pub struct WsCodec {}

impl WsCodec {
    pub fn new() -> Self {
        Self {}
    }

    pub fn encode(&self, item: proto::Message) -> Result<Bytes, EncodeError> {
        let packet = proto::packet::Packet::Message(item);
        let p = proto::Packet {
            packet: Some(packet),
        };
        let mut buf = BytesMut::with_capacity(p.encoded_len());
        p.encode(&mut buf)?;
        Ok(buf.freeze())
    }

    pub fn decode(&self, src: &mut BytesMut) -> Result<Option<proto::Message>, DecodeError> {
        let packet = proto::Packet::decode(src)?.packet;
        match packet {
            Some(proto::packet::Packet::Message(m)) => Ok(Some(m)),
            _ => Ok(None),
        }
    }
}

pub async fn ws_accept_stream<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    timeout: u64,
    stream: S,
    lifecycle: Arc<dyn LifeCycle>,
    peer_addr: Option<SocketAddr>,
) {
    match accept_async(stream).await {
        Ok(websocket) => {
            let (sender, receiver) = unbounded_channel();
            let context = ConnContext {
                proto: Protocol::Websocket,
                timeout,
                create_time: server_base::now_ts_milli(),
                conn_id: lifecycle.new_conn_id(Protocol::Websocket),
                sender: Box::new(Ws {
                    codec: WsCodec::new(),
                    sender: sender.clone(),
                }),
                lifecycle,
                peer_addr,
            };
            let (writer, reader) = websocket.split();
            Accepter::new(context).accept_stream(
                reader,
                writer,
                Box::new(Ws {
                    codec: WsCodec::new(),
                    sender,
                }),
                receiver,
            );
        }
        Err(e) => {
            log::warn!("websocket accept error: {}", e);
        }
    }
}

struct Ws {
    codec: WsCodec,
    sender: UnboundedSender<WsMessage>,
}

impl SendMessage for Ws {
    fn send(&self, msg: proto::Message) -> Result<(), ()> {
        let bytes = self
            .codec
            .encode(msg)
            .map_err(|e| log::warn!("ws encode err: {:?}", e))?;
        self.sender
            .send(WsMessage::Binary(bytes.to_vec()))
            .map_err(|_| ())
    }
}

impl RecvMessage for Ws {
    type Item = WsMessage;

    fn recv(&self, item: Self::Item) -> Result<Option<proto::Message>, ()> {
        match item {
            WsMessage::Text(_) => {}
            WsMessage::Binary(vec) => match self
                .codec
                .decode(&mut bytes::BytesMut::from(vec.as_slice()))
            {
                Ok(Some(message)) => return Ok(Some(message)),
                Ok(None) => {}
                Err(e) => {
                    log::warn!("ws decode err: {:?}", e);
                    return Err(());
                }
            },
            // Does Ws need ping pong?
            WsMessage::Ping(vec) => {
                let _ = self.sender.send(WsMessage::Pong(vec));
            }
            WsMessage::Pong(_) => {}
            WsMessage::Close(close_frame) => {
                log::warn!("websocket will closed: {:?}", close_frame);
                return Err(());
            }
        }
        Ok(None)
    }
}
