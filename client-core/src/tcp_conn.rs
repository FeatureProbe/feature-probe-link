use client_proto::proto::Message;

use crate::Connection;
use crate::TOKIO_RUNTIME;

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