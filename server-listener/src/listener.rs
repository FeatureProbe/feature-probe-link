use crate::peek::PeekStream;
use crate::tcp::tcp_accept_stream;
use crate::tls::{TlsAcceptorBuilder, ALPN_TCP, ALPN_WS, DEPRECATED_TCP};
use crate::ws::ws_accept_stream;
use server_base::tokio;
use server_base::LifeCycle;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::{TcpListener, TcpStream};
use tokio_rustls::{Accept, TlsAcceptor};
use tokio_stream::wrappers::TcpListenerStream;
use tokio_stream::StreamExt;

const DEFAULT_TIMEOUT: u64 = 30;

pub struct Builder {
    address: Option<SocketAddr>,
    lifecycle: Option<Arc<dyn LifeCycle>>,
    accept_builder: TlsAcceptorBuilder,
    timeout: u64,
}

impl Builder {
    pub fn new(addr: &str) -> Self {
        Builder {
            address: addr.parse().ok(),
            lifecycle: None,
            accept_builder: TlsAcceptorBuilder::new(),
            timeout: DEFAULT_TIMEOUT,
        }
    }

    pub fn with_timeout(mut self, timeout: u64) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_cert(mut self, pem_file: Option<String>) -> Self {
        if let Some(ref file) = pem_file {
            match self
                .accept_builder
                .with_cert_pem_file(std::path::Path::new(&file))
            {
                Ok(_) => {
                    log::info!("load cert: {:?} success", file);
                }
                Err(e) => {
                    log::warn!("add_cert_pem_file failed: {:?}", e);
                }
            }
        }
        self
    }

    pub fn with_lifecycle(mut self, lifecycle: Arc<dyn LifeCycle>) -> Self {
        self.lifecycle = Some(lifecycle);
        self
    }

    pub fn build(self) -> Result<Listener, String> {
        if self.address.is_none() {
            return Err("address is none".to_owned());
        }

        Ok(Listener {
            address: self.address.unwrap(),
            timeout: self.timeout,
            tls_acceptor: self.accept_builder.build(),
            lifecycle: self.lifecycle.unwrap(),
        })
    }
}

pub struct Listener {
    address: SocketAddr,
    timeout: u64,
    tls_acceptor: Option<TlsAcceptor>,
    lifecycle: Arc<dyn LifeCycle>,
}

impl Listener {
    async fn accept_tls_stream(
        tls_stream: tokio_rustls::server::TlsStream<TcpStream>,
        timeout: u64,
        peer_addr: Option<SocketAddr>,
        lifecycle: Arc<dyn LifeCycle>,
    ) {
        let (_, session) = tls_stream.get_ref();
        if let Some(alpn) = session.alpn_protocol() {
            if let Ok(utf8_alpn) = std::str::from_utf8(alpn) {
                log::info!("tls alpn: {}", utf8_alpn);
                match utf8_alpn {
                    ALPN_TCP | DEPRECATED_TCP => {
                        tcp_accept_stream(timeout, tls_stream, lifecycle, peer_addr).await;
                    }
                    ALPN_WS => {
                        ws_accept_stream(timeout, tls_stream, lifecycle, peer_addr).await;
                    }
                    _ => log::warn!("unknown ALPN {}", utf8_alpn),
                }
                return;
            }
        }

        detect_accept(tls_stream, timeout, lifecycle, peer_addr).await
    }

    pub async fn listen(self) {
        let timeout = self.timeout;
        let listener = TcpListener::bind(&self.address)
            .await
            .expect("bind tcp failed!");
        let valve = crate::user_port_valve();
        let mut incoming = valve.wrap(TcpListenerStream::new(listener));

        while let Some(stream_result) = incoming.next().await {
            match stream_result {
                Ok(stream) => {
                    let lifecycle = self.lifecycle.clone();
                    let tls_acceptor = self.tls_acceptor.clone();
                    tokio::spawn(async move {
                        let peer_addr = stream.peer_addr().ok();
                        if let Some(ref acceptor) = tls_acceptor {
                            accept_tls_stream(
                                acceptor.accept(stream),
                                timeout,
                                peer_addr,
                                lifecycle,
                            )
                            .await;
                        } else {
                            detect_accept(stream, timeout, lifecycle, peer_addr).await;
                        }
                    });
                }
                Err(e) => log::error!("incoming err: {:?}", e),
            }
        }
        log::error!("halo_connector_server_stop: {:?}", self.address);
    }
}

async fn detect_accept<S: AsyncRead + AsyncWrite + Unpin + Send + 'static>(
    stream: S,
    timeout: u64,
    lifecycle: Arc<dyn LifeCycle>,
    peer_addr: Option<SocketAddr>,
) {
    let mut peek_stream = PeekStream::new(stream);
    match peek_stream.peek(1).await {
        Ok(buf) => {
            let s = std::str::from_utf8(&buf);
            match s {
                Ok(s) => {
                    if s.starts_with('G') {
                        log::info!("detect protocol ws");
                        ws_accept_stream(timeout, peek_stream, lifecycle, peer_addr).await;
                        return;
                    } else if s.starts_with('\u{1}') {
                        log::info!("detect protocol tcp ");
                    } else {
                        log::warn!("unknown protocol {}", s)
                    }
                }
                Err(e) => log::warn!("protocol try tcp  {:?}", e),
            }
        }
        Err(e) => log::warn!("peek err try tcp {:?}", e),
    }
    tcp_accept_stream(timeout, peek_stream, lifecycle, peer_addr).await;
}

async fn accept_tls_stream(
    accept_steam: Accept<TcpStream>,
    timeout: u64,
    peer_addr: Option<SocketAddr>,
    lifecycle: Arc<dyn LifeCycle>,
) {
    match accept_steam.await {
        Ok(tls_stream) => {
            Listener::accept_tls_stream(tls_stream, timeout, peer_addr, lifecycle).await;
        }
        Err(e) => {
            log::warn!("halo_connector tls_stream accept failed: {:?}", e)
        }
    }
}
