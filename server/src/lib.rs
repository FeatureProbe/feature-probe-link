use std::{collections::HashMap, net::SocketAddr, sync::Arc};

use server_base::{
    proto::{EmitSidReq, Message},
    tokio,
    tonic::async_trait,
    BuiltinService, FPConfig, IdGen, PushConn,
};
use server_core::{lifecycle::ConnLifeCycle, CoreOperator};
use server_grpc::{ClusterForwarder, ClusterOperator, Dispatcher};
use server_hproxy::HPROXY_NAMESPACE;
use server_listener::{listen_quic, listen_tls};
use server_state::NodeOperation;
use tracing_appender::{non_blocking::WorkerGuard, rolling};
use tracing_core::LevelFilter;
use tracing_subscriber::{
    fmt::layer, prelude::__tracing_subscriber_SubscriberExt, util::SubscriberInitExt, Layer,
};

pub type BuiltinServiceMap = HashMap<String, Arc<dyn BuiltinService>>;
const CONN_EXPIRE_TIMEOUT: u64 = 30;

pub fn main() {
    let config = server_state::config();
    let _guard = init_logger(&config);
    show_build_info();
    server_state::cluster_state().show_init_info();
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .worker_threads(config.worker_num())
        .thread_name("feature-probe")
        .build()
        .expect("can not start tokio runtime")
        .block_on(async {
            let core = CoreOperator::new(Box::new(Dispatcher::default()));
            start_grpc_server(core.clone());
            tokio::spawn(start_conn_listener(core));
            tokio::signal::ctrl_c().await.expect("shut down");
        });
}

fn init_logger(config: &FPConfig) -> (WorkerGuard, WorkerGuard, WorkerGuard) {
    let dir = config.log_directory();
    let (info_appender, info_guard) =
        tracing_appender::non_blocking(rolling::daily(&dir, "info.log"));
    let (warn_appender, warn_guard) =
        tracing_appender::non_blocking(rolling::daily(&dir, "warn.log"));
    let (err_appender, err_guard) =
        tracing_appender::non_blocking(rolling::daily(&dir, "error.log"));
    let stdout = layer()
        .with_line_number(true)
        .with_filter(LevelFilter::TRACE);
    let info = layer()
        .with_writer(info_appender)
        .with_line_number(true)
        .with_filter(LevelFilter::INFO);
    let warn = layer()
        .with_writer(warn_appender)
        .with_line_number(true)
        .with_filter(LevelFilter::WARN);
    let err = layer()
        .with_writer(err_appender)
        .with_line_number(true)
        .with_filter(LevelFilter::ERROR);

    tracing_subscriber::registry()
        .with(stdout)
        .with(info)
        .with(warn)
        .with(err)
        .init();

    (info_guard, warn_guard, err_guard)
}

fn show_build_info() {
    log::info!("__conf||commit={}", env!("VERGEN_SHA"));
    log::info!("__conf||commit_date={}", env!("VERGEN_COMMIT_DATE"));
    log::info!("__conf||build_ts={}", env!("VERGEN_BUILD_TIMESTAMP"));
    log::info!("__conf||target={}", env!("VERGEN_TARGET_TRIPLE"));
}

async fn start_conn_listener(core: CoreOperator) {
    let builtin_services = Arc::new(init_builtin_services(core.clone()));
    let config = server_state::config();
    let node_id = server_state::cluster_state().server_node().node_id.clone();
    let sid_gen = IdGen::new(node_id);
    let listen_addr = config.conn_listen_addr();
    let cert_path = config.cert_path();
    let lifecycle = Arc::new(ConnLifeCycle::new(core, sid_gen, builtin_services.clone()));
    log::info!("tls cert path: {:?}", cert_path);

    if let Some(cert_path) = config.cert_path_quic() {
        tokio::spawn(listen_quic(
            listen_addr.clone(),
            CONN_EXPIRE_TIMEOUT,
            cert_path,
            lifecycle.clone(),
        ));
    }

    tokio::spawn(listen_tls(
        listen_addr,
        CONN_EXPIRE_TIMEOUT,
        cert_path,
        lifecycle,
    ));
}

fn init_builtin_services(core: CoreOperator) -> BuiltinServiceMap {
    let mut builtin_handlers: HashMap<String, Arc<dyn BuiltinService>> = HashMap::new();
    builtin_handlers.insert(
        "__ECHO".to_owned(),
        Arc::new(EchoService {
            pusher: Box::new(core.clone()),
        }),
    );
    if let Some(http_proxy) = server_hproxy::build_http_proxy(core) {
        let http_proxy = Arc::new(http_proxy);
        builtin_handlers.insert(HPROXY_NAMESPACE.to_string(), http_proxy);
    }
    builtin_handlers
}

fn start_grpc_server(core: CoreOperator) {
    let config = server_state::config();
    let cluster_addr = config.peer_listen_addr();
    let forwarded_grpc = core.clone();
    server_grpc::grpc_listen(&cluster_addr, forwarded_grpc);

    let service_addr = server_state::config().service_listen_addr();
    let core_operator = core;
    let cluster_forwarder = ClusterForwarder::new();
    let node_operator = server_state::cluster_state();
    let cluster_operator = ClusterOperator::new(node_operator, core_operator, cluster_forwarder);
    server_grpc::grpc_listen(&service_addr, cluster_operator);
}

struct EchoService {
    pusher: Box<dyn PushConn>,
}

#[async_trait]
impl BuiltinService for EchoService {
    async fn on_message(&self, sid: &str, _peer_addr: Option<SocketAddr>, message: Message) {
        log::info!("EchoService recv {:?} from {}", message, sid);
        let sid = sid.to_owned();
        let message = Some(message);
        let req = EmitSidReq {
            sid,
            message,
            trace: None,
        };
        log::info!("EchoService push {:?}", req);
        self.pusher.push(req).await;
    }
}
