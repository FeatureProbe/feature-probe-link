use crate::accepter::Accepter;
use crate::codec::Codec;
use cached::proc_macro::cached;
use futures::StreamExt;
use server_base::proto::{packet::Packet, Message, Pong};
use server_base::{tokio, ConnContext, LifeCycle, Protocol, RecvMessage, SendMessage};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use tokio_util::codec::Framed;

pub async fn tcp_accept_stream<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    timeout: u64,
    stream: S,
    lifecycle: Arc<dyn LifeCycle>,
    peer_addr: Option<SocketAddr>,
) {
    #[cached(time = 60)]
    fn tokio_codec_size() -> usize {
        server_state::config().tokio_codec_size()
    }

    let (sender, receiver) = unbounded_channel();
    let context = ConnContext {
        proto: Protocol::Tcp,
        timeout,
        create_time: server_base::now_ts_milli(),
        conn_id: lifecycle.new_conn_id(Protocol::Tcp),
        sender: Box::new(Tcp {
            sender: sender.clone(),
        }),
        lifecycle,
        peer_addr,
    };

    let frame = Framed::with_capacity(stream, Codec::default(), tokio_codec_size());
    let (writer, reader) = frame.split();
    Accepter::new(context).accept_stream(reader, writer, Box::new(Tcp { sender }), receiver)
}

struct Tcp {
    sender: UnboundedSender<Packet>,
}

impl RecvMessage for Tcp {
    type Item = Packet;

    fn recv(&self, item: Self::Item) -> Result<Option<Message>, ()> {
        match item {
            Packet::Ping(ping) => {
                let _ = self.sender.send(Packet::Pong(Pong {
                    timestamp: ping.timestamp,
                }));
            }
            Packet::Message(msg) => return Ok(Some(msg)),
            _ => log::warn!("tcp unknown packet: {:?}", item),
        }
        Ok(None)
    }
}

impl SendMessage for Tcp {
    fn send(&self, msg: Message) -> Result<(), ()> {
        self.sender.send(Packet::Message(msg)).map_err(|_| ())
    }
}
