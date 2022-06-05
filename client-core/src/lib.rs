mod codec;
mod id_gen;
mod manager;
mod quic_conn;
mod tcp_conn;

pub use client_proto::proto::Message;

use async_trait::async_trait;
use client_proto::proto::packet::Packet;
use lazy_static::lazy_static;
use rustls::{Certificate, ServerName};
use std::{collections::HashMap, sync::Arc, sync::Weak, time::SystemTime};
use tokio::runtime::{Builder, Runtime};

lazy_static! {
    pub static ref TOKIO_RUNTIME: Runtime = Builder::new_multi_thread()
        .enable_all()
        .worker_threads(4)
        .thread_name("featureprobe")
        .build()
        .expect("can not start tokio runtime");
}

pub trait PlatformCallback: Send + Sync {
    fn auth(&self);
    fn recv(&self, message: Message);
    fn state_change(&self, old: u8, new: u8);
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetworkType {
    TypeUnknown,
    TypeNoNet,
    TypeWiFi,
    Type2G,
    Type3G,
    Type4G,
    Type5G,
}

impl Default for NetworkType {
    fn default() -> Self {
        NetworkType::TypeUnknown
    }
}

pub struct LinkClient {}

impl LinkClient {
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        let _enter = TOKIO_RUNTIME.enter();

        Self {}
    }

    pub fn open(&self) {}

    pub fn send(&self, _message: Message) {}

    pub fn close(&self) {}

    pub fn state(&self) -> u8 {
        0
    }

    pub fn go_background(&self) {}

    pub fn go_foreground(&self) {}

    pub fn network_change(&self, _old: NetworkType, _new: NetworkType) {}

    pub fn set_attrs(&self, _attrs: HashMap<String, String>) {}
}

#[async_trait]
pub trait Connection: Send + Sync {
    async fn open(&self) -> bool;

    async fn send(&self, packet: Packet) -> bool;

    async fn close(&self);

    async fn state(&self) -> u8;

    async fn is_same_conn(&self, unique_id: &str) -> bool;
}

#[derive(Clone)]
struct WeakManager<T: Clone> {
    manager: Option<Weak<T>>,
}

impl<T: Clone> WeakManager<T> {
    fn new(manager: Option<Weak<T>>) -> Self {
        Self { manager }
    }

    fn upgrade(&self) -> Option<Arc<T>> {
        if let Some(ref manager) = self.manager {
            if let Some(manager) = manager.upgrade() {
                return Some(manager);
            }
        }
        None
    }
}

#[derive(Debug, PartialEq, Copy, Clone)]
pub enum State {
    Init = 1,
    Connecting = 2,
    Connected = 3,
    DisConnected = 4,
    Closed = 5,
}

impl Default for State {
    fn default() -> Self {
        State::Init
    }
}

#[allow(clippy::from_over_into)]
impl Into<usize> for State {
    fn into(self) -> usize {
        self as usize
    }
}

#[allow(clippy::from_over_into)]
impl Into<u8> for State {
    fn into(self) -> u8 {
        self as u8
    }
}

pub fn now_ts() -> u64 {
    let start = std::time::SystemTime::now();
    let since_the_epoch = start
        .duration_since(std::time::UNIX_EPOCH)
        .expect("time went backwards");
    since_the_epoch.as_millis() as u64
}

// for test
pub struct SkipServerVerification;

impl SkipServerVerification {
    pub fn new() -> Arc<Self> {
        Arc::new(Self)
    }
}

impl rustls::client::ServerCertVerifier for SkipServerVerification {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::Certificate,
        _intermediates: &[Certificate],
        _server_name: &ServerName,
        _scts: &mut dyn Iterator<Item = &[u8]>,
        _ocsp_response: &[u8],
        _now: SystemTime,
    ) -> Result<rustls::client::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::ServerCertVerified::assertion())
    }
}
