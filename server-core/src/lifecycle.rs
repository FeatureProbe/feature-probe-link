use crate::CoreOperator;
use server_base::tokio;
use server_base::{proto::Message, BuiltinService, HandyRwLock, IdGen, Protocol};
use server_base::{Conn, LifeCycle};

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

struct Inner {
    operator: CoreOperator,
    cid_gen: IdGen,
    builtin_services: Arc<HashMap<String, Arc<dyn BuiltinService>>>,
}

impl Inner {
    pub fn new(
        operator: CoreOperator,
        cid_gen: IdGen,
        builtin_services: Arc<HashMap<String, Arc<dyn BuiltinService>>>,
    ) -> Self {
        Self {
            operator,
            cid_gen,
            builtin_services,
        }
    }
}

#[derive(Clone)]
pub struct ConnLifeCycle {
    inner: Arc<Inner>,
}

impl ConnLifeCycle {
    pub fn new(
        operator: CoreOperator,
        cid_gen: IdGen,
        builtin_services: Arc<HashMap<String, Arc<dyn BuiltinService>>>,
    ) -> Self {
        let inner = Arc::new(Inner::new(operator, cid_gen, builtin_services));
        Self { inner }
    }

    fn peer_addr(&self, conn_id: &str) -> Option<SocketAddr> {
        self.inner
            .operator
            .conn(conn_id)
            .and_then(|conn| conn.inner.peer_addr)
    }
}

impl LifeCycle for ConnLifeCycle {
    fn new_conn_id(&self, protocol: Protocol) -> String {
        self.inner.cid_gen.conn_id(protocol)
    }

    /// connection create, connection can bind id from return value
    fn on_conn_create(&self, conn: Conn) {
        self.inner.operator.reg_conn(conn);
    }

    fn on_message_incoming(&self, conn_id: &str, protocol: &Protocol, message: Message) {
        let message = message;
        let namespace = message.namespace.clone();

        // perf: do not create temp String every time
        match protocol {
            Protocol::Tcp => {
                // meter!("tcp_incoming_message_meter", namespace);
            }
            Protocol::Websocket => {
                // meter!("ws_incoming_message_meter", namespace);
            }
            Protocol::Quic => {
                // meter!("quic_incoming_message_meter", namespace);
            }
        }
        let conn_id_string = conn_id.to_owned();

        if let Some(builtin_service) = self.inner.builtin_services.get(&namespace) {
            let service_clone = builtin_service.clone();
            let peer_addr = self.peer_addr(conn_id);
            tokio::spawn(async move {
                service_clone
                    .on_message(&conn_id_string, peer_addr, message)
                    .await;
            });
        } else {
            self.inner.operator.dispatch_message(conn_id, message);
        }
    }

    fn on_conn_destroy(&self, conn: Conn) {
        self.inner.operator.unreg_conn(&conn);
    }

    fn should_timeout(&self) -> bool {
        let conn_stop = server_base::USER_CONN_STOP.rl();
        *conn_stop
    }
}
