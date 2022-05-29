use crate::accepter::Accepter;
use crate::codec::Codec;
use cached::proc_macro::cached;
use futures::StreamExt;
use quinn::congestion::BbrConfig;
use quinn::ServerConfig;
use server_base::packet::Packet;
use server_base::tokio::sync::mpsc::{unbounded_channel, UnboundedSender};
use server_base::{tokio, Message, Pong};
use server_base::{ConnContext, LifeCycle, Protocol, RecvMessage, SendMessage};
use snafu::{ResultExt, Whatever};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::codec::{FramedRead, FramedWrite};

const ALPN_QUIC: &[&[u8]] = &[b"hq-29"];
const QUIC_KEEP_ALIVE: u64 = 15;
const QUIC_MAX_IDLE: u64 = 30;

pub async fn listen_quic(
    addr: String,
    timeout_secs: u64,
    cert_path: String,
    lifecycle: Arc<dyn LifeCycle>,
) {
    let mut err = None;
    // retry for integration test UDP bind error
    for _ in 0..3 {
        match do_listen_quic(
            addr.clone(),
            timeout_secs,
            cert_path.clone(),
            lifecycle.clone(),
        )
        .await
        {
            Err(e) => err = Some(e),
            Ok(()) => err = None,
        }
        if err.is_none() {
            break;
        }
        log::error!("listen quic failed, try again {:?}", err);
        tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    }
    if err.is_some() {
        panic!("error listen quic {:?}", err);
    }
}

async fn do_listen_quic(
    addr: String,
    timeout_secs: u64,
    cert_path: String,
    lifecycle: Arc<dyn LifeCycle>,
) -> Result<(), Whatever> {
    let valve = crate::user_port_valve();
    let mut incoming = {
        let addr = addr
            .parse()
            .with_whatever_context(|_| format!("addr parse error {}", addr))?;
        let (endpoint, incoming) = quinn::Endpoint::server(server_config(cert_path)?, addr)
            .with_whatever_context(|e| e.to_string())?;
        log::info!("listening quic on {:?}", endpoint.local_addr());
        valve.wrap(incoming)
    };

    while let Some(conn) = incoming.next().await {
        log::info!("quic connection incoming");
        tokio::spawn(accept_quic(conn, timeout_secs, lifecycle.clone()));
    }
    log::info!("quic server stop {}", addr);
    Ok(())
}

fn server_config(cert_path: String) -> Result<ServerConfig, Whatever> {
    let (certs, key) = crate::tls::cert_key(std::path::Path::new(&cert_path))?;
    let mut server_crypto = rustls::ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .with_whatever_context(|_| "cert error".to_string())?;
    server_crypto.alpn_protocols = ALPN_QUIC.iter().map(|&x| x.into()).collect();
    server_crypto.key_log = Arc::new(rustls::KeyLogFile::new());
    let mut server_config = quinn::ServerConfig::with_crypto(Arc::new(server_crypto));
    Arc::get_mut(&mut server_config.transport)
        .unwrap()
        .congestion_controller_factory(Arc::new(BbrConfig::default()))
        .max_concurrent_uni_streams(0_u8.into())
        .keep_alive_interval(Some(Duration::from_secs(QUIC_KEEP_ALIVE)))
        .max_idle_timeout(Some(Duration::from_secs(QUIC_MAX_IDLE).try_into().unwrap())); // safe to unwrap

    Ok(server_config)
}

async fn accept_quic(conn: quinn::Connecting, timeout_secs: u64, lifecycle: Arc<dyn LifeCycle>) {
    let quinn::NewConnection { mut bi_streams, .. } = match conn.await {
        Ok(c) => c,
        Err(e) => return log::error!("quinn new connection error: {:?}", e),
    };

    async {
        log::info!("quic connection established");

        // Each stream initiated by the client constitutes a new request.
        while let Some(stream) = bi_streams.next().await {
            log::info!("quic new stream incoming");
            let stream = match stream {
                Err(quinn::ConnectionError::ApplicationClosed { .. }) => {
                    log::info!("quic connection closed");
                    return;
                }
                Err(e) => {
                    log::warn!("quic connection error {}", e);
                    return;
                }
                Ok(s) => s,
            };
            tokio::spawn(accept_quic_stream(stream, timeout_secs, lifecycle.clone()));
        }
    }
    .await;
}

async fn accept_quic_stream(
    (send, recv): (quinn::SendStream, quinn::RecvStream),
    timeout: u64,
    lifecycle: Arc<dyn LifeCycle>,
) {
    #[cached(time = 60)]
    fn tokio_codec_size() -> usize {
        server_state::config().tokio_codec_size()
    }

    log::trace!("quic handle new stream");
    let (sender, receiver) = unbounded_channel();
    let context = ConnContext {
        proto: Protocol::Quic,
        timeout,
        create_time: server_base::now_ts_milli(),
        conn_id: lifecycle.new_conn_id(Protocol::Quic),
        sender: Box::new(Quic {
            sender: sender.clone(),
        }),
        lifecycle,
        peer_addr: None,
    };
    let reader = FramedRead::with_capacity(recv, Codec::new(), tokio_codec_size());
    let writer = FramedWrite::new(send, Codec::new());
    Accepter::new(context).accept_stream(reader, writer, Box::new(Quic { sender }), receiver);
    log::trace!("quic stream accept");
}

struct Quic {
    sender: UnboundedSender<Packet>,
}

impl RecvMessage for Quic {
    type Item = Packet;

    fn recv(&self, item: Self::Item) -> Result<Option<Message>, ()> {
        match item {
            Packet::Ping(ping) => {
                let _ = self.sender.send(Packet::Pong(Pong {
                    timestamp: ping.timestamp,
                }));
            }
            Packet::Message(msg) => return Ok(Some(msg)),
            _ => log::warn!("quic unknown packet: {:?}", item),
        }
        Ok(None)
    }
}

impl SendMessage for Quic {
    fn send(&self, msg: Message) -> Result<(), ()> {
        self.sender.send(Packet::Message(msg)).map_err(|_| ())
    }
}
