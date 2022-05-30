use std::{collections::HashMap, sync::Arc};

use server_base::{tokio, BuiltinService, IdGen};
use server_core::{lifecycle::ConnLifeCycle, CoreOperator};
use server_hproxy::HPROXY_NAMESPACE;
use server_listener::{listen_quic, listen_tls};
use server_state::NodeOperation;

pub type BuiltinServiceMap = HashMap<String, Arc<dyn BuiltinService>>;
const CONN_EXPIRE_TIMEOUT: u64 = 30;

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
    let cid_gen = IdGen::new(node_id);
    let listen_addr = config.conn_listen_addr();
    let cert_path = config.cert_path();
    let lifecycle = Arc::new(ConnLifeCycle::new(core, cid_gen, builtin_services.clone()));
    log::info!("tls cert path: {:?}", cert_path);

    if let Some(cert_path) = config.cert_path_quic() {
        tokio::spawn(listen_quic(
            listen_addr.clone(),
            CONN_EXPIRE_TIMEOUT,
            cert_path.clone(),
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
    if let Some(http_proxy) = server_hproxy::build_http_proxy(core) {
        let http_proxy = Arc::new(http_proxy);
        builtin_handlers.insert(HPROXY_NAMESPACE.to_string(), http_proxy.clone());
    }
    builtin_handlers
}
