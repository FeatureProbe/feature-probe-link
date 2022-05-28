use crate::conn::Conn;
use crate::Protocol;
use crate::{LifeCycle, SendMessage};
use featureprobe_link_proto::proto::Message;
use std::net::SocketAddr;
use std::sync::Arc;

pub struct ConnContext {
    pub proto: Protocol,
    pub timeout: u64,
    pub create_time: u64,
    pub conn_id: String,
    pub sender: Box<dyn SendMessage>,
    pub lifecycle: Arc<dyn LifeCycle>,
    pub peer_addr: Option<SocketAddr>,
}

impl ConnContext {
    #[allow(clippy::result_unit_err)]
    pub fn send(&self, msg: Message) -> Result<(), ()> {
        self.sender.send(msg)
    }

    /// - accept incoming message from socket
    /// - route this message to upstream by server-core
    pub fn accept_message(&self, msg: Message) {
        self.lifecycle
            .on_message_incoming(&self.conn_id, &self.proto, msg);
    }

    pub fn on_conn_create(self: &Arc<Self>) {
        self.lifecycle.on_conn_create(Conn::from(self.clone()));
    }

    pub fn on_conn_destroy(self: &Arc<Self>) {
        self.lifecycle.on_conn_destroy(Conn::from(self.clone()));
    }

    pub fn should_timeout(&self) -> bool {
        self.lifecycle.should_timeout()
    }
}
