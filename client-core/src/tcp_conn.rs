use crate::codec::Codec;
use crate::id_gen::ID_GEN;
use crate::manager::ConnManager;
use crate::{Connection, State, WeakManager};
use crate::{Error, SkipServerVerification};
use anyhow::{anyhow, Result as AnyResult};
use async_trait::async_trait;
use client_proto::proto::packet::Packet;
use futures::sink::SinkExt;
use futures::stream::{SplitSink, SplitStream, StreamExt};
use rustls::{ClientConfig, RootCertStore, ServerName};
use std::fmt::Display;
use std::net::ToSocketAddrs;
use std::pin::Pin;
use std::sync::{Arc, Weak};
use std::task::{Context, Poll};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::TcpStream;
use tokio::sync::Mutex;
use tokio::time::Duration as TDuration;
use tokio_rustls::client::TlsStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::Framed;

type TcpFramed = Framed<TStream, Codec>;
type TcpSplitSink = SplitSink<TcpFramed, Packet>;
type TcpSplitStream = SplitStream<TcpFramed>;

pub struct TcpConnection {
    pub unique_id: String,
    pub connecting_ts: u64,
    host: String,
    is_ssl: bool,
    timeout_ms: u64,
    manager: WeakManager<ConnManager>,
    open_lock: Arc<Mutex<u8>>,
    conn_state: Mutex<State>,
    sender: Mutex<Option<TcpSplitSink>>,
    connected_ms: Mutex<u64>,
}

#[async_trait]
impl Connection for TcpConnection {
    async fn open(&self) -> bool {
        let _lock = self.open_lock.lock().await;
        if self.is_opened().await {
            return false;
        }

        log::debug!("{} is opening", self);
        if let Some(ref manager) = &self.manager.upgrade() {
            if manager.is_connected().await {
                log::info!("some tcp is opened, skip");
                return false;
            }
        }

        self.change_conn_state(State::Connecting).await;
        match self
            .open_tcp_stream(&self.host, self.is_ssl, self.timeout_ms)
            .await
        {
            Ok(conn) => {
                self.handle_tcp_stream(conn).await;
                log::debug!("{} open success", self);
                return true;
            }
            Err(e) => {
                log::debug!("{} open failed {}", self, e.to_string());
                return false;
            }
        }
    }

    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let mut sender_guard = self.sender.lock().await;
        match *sender_guard {
            Some(ref mut s) => {
                log::info!("{} send {:?}", self, packet);
                s.send(packet)
                    .await
                    .map_err(|e| Error::SendError { msg: e.to_string() })?;
            }
            None => log::warn!("{} packet drop, conn not ready", self),
        };
        // false
        Ok(())
    }

    async fn close(&self) {}

    async fn state(&self) -> u8 {
        self.conn_state().await.into()
    }

    async fn is_same_conn(&self, unique_id: &str) -> bool {
        self.unique_id.eq(unique_id)
    }
}

impl TcpConnection {
    pub fn new(host: String, is_ssl: bool, timeout_ms: u64) -> Self {
        Self {
            unique_id: ID_GEN.generate(&host, "tcp"),
            manager: WeakManager::new(None),
            host,
            is_ssl,
            timeout_ms,
            open_lock: Arc::new(Mutex::new(0)),
            conn_state: Mutex::new(State::Init),
            sender: Mutex::new(None),
            connecting_ts: crate::now_ts(),
            connected_ms: Mutex::new(0),
        }
    }

    pub fn with_manager(mut self, manager: Weak<ConnManager>) -> Self {
        self.manager = WeakManager::new(Some(manager));
        self
    }

    pub async fn is_opened(&self) -> bool {
        matches!(
            self.conn_state().await,
            State::Connecting | State::Connected
        )
    }

    pub async fn open_tcp_stream(
        &self,
        host: &str,
        ssl: bool,
        timeout_ms: u64,
    ) -> AnyResult<TStream> {
        log::debug!("{} open_tcp_stream", self);

        let socket_addr = host
            .to_socket_addrs()?
            .next()
            .ok_or_else(|| anyhow!("couldn't resolve to an address"))?;

        log::info!("{} connecting addr: {:?}", self, socket_addr);
        match tokio::time::timeout(
            TDuration::from_millis(timeout_ms),
            TcpStream::connect(socket_addr),
        )
        .await
        {
            Err(e) => Err(anyhow!("{} connect timeout {}", self, e)),
            Ok(stream_result) => match stream_result {
                Err(e) => Err(anyhow!("{} connect failed {}", self, e)),
                Ok(stream) if ssl => self.open_tcp_tls_stream(timeout_ms, stream, host).await,
                Ok(stream) => {
                    log::debug!("{} stream opened", self);
                    Ok(TStream::Tcp(Box::new(stream)))
                }
            },
        }
    }

    pub async fn open_tcp_tls_stream(
        &self,
        timeout_ms: u64,
        stream: TcpStream,
        host: &str,
    ) -> AnyResult<TStream> {
        log::debug!("{} stream opened, start tls connecting", self);
        let server_name = ServerName::try_from(host)?;
        match tokio::time::timeout(
            TDuration::from_millis(timeout_ms),
            TlsConnector::from(client_config()).connect(server_name, stream),
        )
        .await
        {
            Err(e) => Err(anyhow!("{} connect tls timeout {}", self, e)),
            Ok(stream) => match stream {
                Err(e) => Err(anyhow!("{} tls connect failed: {}", self, e)),
                Ok(tls_stream) => {
                    log::debug!("{} tls done", self);
                    Ok(TStream::Tls(Box::new(tls_stream)))
                }
            },
        }
    }

    pub async fn handle_tcp_stream(&self, stream: TStream) {
        log::info!("{} handle_tcp_stream", self);
        let framed_stream = Framed::new(stream, Codec::new());
        let (sender, receiver) = framed_stream.split();
        let mut send_guard = self.sender.lock().await;
        *send_guard = Some(sender);
        drop(send_guard);

        self.do_recv(receiver).await;
        self.change_conn_state(State::Connected).await;
    }

    pub async fn do_recv(&self, mut receiver: TcpSplitStream) {
        let manager = self.manager.clone();
        tokio::spawn(async move {
            while let Some(p) = receiver.next().await {
                let manager = manager.clone();
                manager_recv(
                    manager,
                    p.map_err(|e| Error::RecvError { msg: e.to_string() }),
                )
                .await
            }
        });
    }

    pub async fn conn_state(&self) -> State {
        let guard = self.conn_state.lock().await;
        *guard
    }

    pub async fn change_conn_state(&self, new_state: State) {
        let current_state = self.conn_state().await;
        let old_state = current_state;

        match new_state {
            State::Connected => self.update_connected_ms().await,
            _ => self.clear_connected_ms().await,
        }

        if old_state != new_state {
            log::trace!("{} from {:?} to {:?}", self, old_state, new_state);
            self.set_conn_state(new_state).await;

            if let Some(ref manager) = &self.manager.upgrade() {
                manager
                    .set_conn_state(new_state, Some(&self.unique_id))
                    .await;
            }
        }
    }

    pub async fn update_connected_ms(&self) {
        let ts = self.connecting_ts;
        let now = crate::now_ts();
        let mut ms_guard = self.connected_ms.lock().await;
        let d = (now - ts) as u64;
        *ms_guard = d;
    }

    pub async fn clear_connected_ms(&self) {
        let mut ms_guard = self.connected_ms.lock().await;
        *ms_guard = 0;
    }

    pub async fn set_conn_state(&self, new_state: State) {
        let mut guard = self.conn_state.lock().await;
        *guard = new_state;
    }
}

async fn manager_recv(manager: WeakManager<ConnManager>, packet: Result<Packet, Error>) {
    if let Some(ref manager) = manager.upgrade() {
        manager.recv(packet).await
    }
}

impl Display for TcpConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.unique_id)
    }
}

impl Drop for TcpConnection {
    fn drop(&mut self) {
        log::info!("{} dropped", self);
    }
}

pub enum TStream {
    Tcp(Box<TcpStream>),
    Tls(Box<TlsStream<TcpStream>>),
}

impl AsyncWrite for TStream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        match &mut *self {
            Self::Tcp(ref mut s) => AsyncWrite::poll_write(Pin::new(s), cx, buf),
            Self::Tls(ref mut s) => AsyncWrite::poll_write(Pin::new(s), cx, buf),
        }
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::io::Result<()>> {
        match &mut *self {
            Self::Tcp(ref mut s) => AsyncWrite::poll_flush(Pin::new(s), cx),
            Self::Tls(ref mut s) => AsyncWrite::poll_flush(Pin::new(s), cx),
        }
    }

    fn poll_shutdown(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<std::io::Result<()>> {
        match &mut *self {
            Self::Tcp(ref mut s) => AsyncWrite::poll_shutdown(Pin::new(s), cx),
            Self::Tls(ref mut s) => AsyncWrite::poll_shutdown(Pin::new(s), cx),
        }
    }
}

impl AsyncRead for TStream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        match &mut *self {
            Self::Tcp(ref mut s) => AsyncRead::poll_read(Pin::new(s), cx, buf),
            Self::Tls(ref mut s) => AsyncRead::poll_read(Pin::new(s), cx, buf),
        }
    }
}

fn client_config() -> Arc<ClientConfig> {
    let mut client_config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(RootCertStore::empty())
        .with_no_client_auth();
    client_config.alpn_protocols = vec![Vec::from("tcp")];
    client_config
        .dangerous()
        .set_certificate_verifier(SkipServerVerification::new());
    Arc::new(client_config)
}

#[cfg(test)]
mod tests {

    use super::*;
    use client_proto::proto::Message;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_conn() {
        let _ = tracing_subscriber::fmt().with_env_filter("trace").init();
        let tcp_conn = TcpConnection::new("127.0.0.1:8082".to_owned(), false, 100);
        let conn: &dyn Connection = &tcp_conn;
        let c = &conn;

        c.open().await;

        let message = Message {
            namespace: "test".to_owned(),
            path: "/test".to_owned(),
            metadata: Default::default(),
            body: Default::default(),
            expire_at: None,
        };

        loop {
            let _ = conn.send(Packet::Message(message.clone())).await;
            tokio::time::sleep(Duration::from_secs(2)).await;
        }

        // conn.send(message.clone()).await;
        // conn.send(message.clone()).await;
        // conn.send(message).await;

        // tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
