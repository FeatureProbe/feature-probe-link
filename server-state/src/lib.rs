#[macro_use]
extern crate lazy_static;

mod cluster;
mod service;
mod state;

pub use cluster::Clusters;
pub use service::Services;
pub use state::{NodeOperation, State};

use parking_lot::RwLock;
use server_base::tonic::transport::Endpoint;
use server_base::Channel;
use server_base::FPConfig;
use server_base::HandyRwLock;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

lazy_static! {
    // these are initiated only once
    static ref CONFIG: RwLock<FPConfig> = RwLock::new(FPConfig::new());

    // these are maintained by fplink-cluster
    static ref CLUSTER_CLIENT_STATE: Clusters = Clusters::new();
    static ref SERVICE_CLIENT_STATE: Services = Services::new();
    static ref CLUSTER_STATE: State = State::new(config());

    // these are maintained by fplink-core
    static ref TCP_CONNS_COUNT: AtomicUsize = AtomicUsize::new(0);
    static ref WS_CONNS_COUNT: AtomicUsize = AtomicUsize::new(0);
    static ref QUIC_CONNS_COUNT: AtomicUsize = AtomicUsize::new(0);
}

pub fn set_config(config: FPConfig) {
    let mut guard = CONFIG.wl();
    *guard = config;
}

pub fn config() -> FPConfig {
    let guard = CONFIG.rl();
    guard.clone()
}

pub fn get_zone() -> Option<String> {
    let guard = CONFIG.rl();
    guard.zone()
}

pub fn cluster_state() -> State {
    CLUSTER_STATE.clone()
}

pub fn cluster_client_state() -> Clusters {
    CLUSTER_CLIENT_STATE.clone()
}

pub fn service_client_state() -> Services {
    SERVICE_CLIENT_STATE.clone()
}

pub fn inc_tcp_count() {
    TCP_CONNS_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn dec_tcp_count() {
    TCP_CONNS_COUNT.fetch_sub(1, Ordering::SeqCst);
}

pub fn get_tcp_count() -> isize {
    TCP_CONNS_COUNT.load(Ordering::Relaxed) as isize
}

pub fn inc_ws_count() {
    WS_CONNS_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn dec_ws_count() {
    WS_CONNS_COUNT.fetch_sub(1, Ordering::SeqCst);
}

pub fn get_ws_count() -> isize {
    WS_CONNS_COUNT.load(Ordering::Relaxed) as isize
}

pub fn inc_quic_count() {
    QUIC_CONNS_COUNT.fetch_add(1, Ordering::SeqCst);
}

pub fn dec_quic_count() {
    QUIC_CONNS_COUNT.fetch_sub(1, Ordering::SeqCst);
}

pub fn get_quic_count() -> isize {
    QUIC_CONNS_COUNT.load(Ordering::Relaxed) as isize
}

pub fn get_total_conn_count() -> isize {
    get_quic_count() + get_tcp_count() + get_ws_count()
}

pub trait Connect: Default {
    type Item: Clone;
    fn connect<S: Into<String>>(addr: S, timeout_ms: u64) -> Self::Item;
}

#[derive(Default, Clone)]
pub struct GrpcClient {
    addr: Option<String>,
}

impl fmt::Display for GrpcClient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "grpc_client: {:?}", self.addr.as_ref())
    }
}

impl Connect for GrpcClient {
    type Item = Channel;
    fn connect<S: Into<String>>(addr: S, timeout_ms: u64) -> Self::Item {
        let addr = format!("http://{}", addr.into());
        let endpoint: Endpoint = addr.try_into().expect("invalid addr");
        let endpoint = endpoint.timeout(Duration::from_millis(timeout_ms));
        endpoint.connect_lazy()
    }
}
