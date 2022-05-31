#[macro_use]
extern crate lazy_static;

mod config;
mod conn;
mod context;
mod id_gen;
mod node;
mod utils;

pub use crate::config::*;
pub use crate::utils::*;
pub use server_proto as codec;
pub use server_proto::proto;
pub use server_proto::tonic;
pub use conn::Conn;
pub use context::ConnContext;
pub use id_gen::IdGen;
pub use minstant;
pub use node::{RegistryNode, ServerNode, ServiceNode};
pub use tokio;
pub use tonic::transport::Channel;

use async_trait::async_trait;
use server_proto::proto::*;
use parking_lot::RwLock;
use std::net::SocketAddr;

lazy_static! {
    pub static ref USER_PORT_LISTEN: RwLock<bool> = RwLock::new(false);
    pub static ref USER_CONN_STOP: RwLock<bool> = RwLock::new(false);
}

#[async_trait]
pub trait PushConn: Send + Sync {
    async fn push(&self, req: PushConnReq);
}

#[async_trait]
pub trait Dispatch: Send + Sync {
    async fn dispatch(&self, namespace: String, request: MessageReq) -> bool;
}

#[async_trait]
pub trait BuiltinService: Send + Sync {
    async fn on_message(&self, conn_id: &str, peer_addr: Option<SocketAddr>, message: Message);
}

#[async_trait]
pub trait CoreOperation: Send + Sync + Clone + 'static {
    async fn subscribe(&self, request: SubReq) -> bool;

    async fn unsubscribe(&self, request: UnSubReq) -> bool;

    async fn bulk_subscribe(&self, request: BulkSubReq) -> bool;

    async fn publish(&self, request: PubReq) -> PubResp;

    async fn push_conn(&self, request: PushConnReq) -> bool;

    async fn get_conn_channels(&self, request: GetConnsReq) -> Vec<ConnChannels>;

    async fn get_channels(&self, request: GetChannelsReq) -> Vec<String>;
}

pub trait LifeCycle: Send + Sync {
    fn new_conn_id(&self, protocol: Protocol) -> String;

    fn on_conn_create(&self, conn: Conn);

    fn on_message_incoming(&self, conn_id: &str, protocol: &Protocol, message: Message);

    fn on_conn_destroy(&self, conn: Conn);

    fn should_timeout(&self) -> bool;
}

pub trait SendMessage: Send + Sync {
    #[allow(clippy::result_unit_err)]
    fn send(&self, msg: Message) -> Result<(), ()>;
}

pub trait RecvMessage: Send + Sync {
    type Item;

    #[allow(clippy::result_unit_err)]
    fn recv(&self, item: Self::Item) -> Result<Option<Message>, ()>;
}

#[derive(Clone, Debug, PartialEq, Copy)]
pub enum Protocol {
    Tcp,
    Websocket,
    Quic,
}

// log online time will use this
impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            Protocol::Tcp => write!(f, "tcp"),
            Protocol::Websocket => write!(f, "ws"),
            Protocol::Quic => write!(f, "quic"),
        }
    }
}
