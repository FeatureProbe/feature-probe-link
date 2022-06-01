use std::collections::HashMap;

pub use client_proto::proto::Message;
use lazy_static::lazy_static;
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

pub enum NetworkType {
    TypeUnknown,
    TypeNoNet,
    TypeWiFi,
    Type2G,
    Type3G,
    Type4G,
    Type5G,
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

pub trait Connection: Send + Sync {
    #[allow(clippy::new_without_default)]
    fn new() -> Self;

    fn open(&self);

    fn send(&self, _message: Message);

    fn close(&self);

    fn state(&self) -> u8;
}

pub struct TcpConnection {}

impl Connection for TcpConnection {
    #[allow(clippy::new_without_default)]
    fn new() -> Self {
        let _enter = TOKIO_RUNTIME.enter();

        Self {}
    }

    fn open(&self) {}

    fn send(&self, _message: Message) {}

    fn close(&self) {}

    fn state(&self) -> u8 {
        0
    }
}
