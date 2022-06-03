use client_proto::proto::packet::Packet;
use std::sync::Arc;
use tokio::sync::{Mutex as TMutex, RwLock as TRwLock};

use crate::{tcp_conn::TcpConnection, Connection, PlatformCallback, State};

#[derive(Clone)]
pub struct ConnManager {
    inner: Arc<ConnManagerInner>,
}

pub struct ConnManagerInner {
    pub main_conn: TRwLock<Option<Box<dyn Connection>>>,
    pub connecting_ts: TRwLock<Option<u128>>, // first connecting ts
    pub host: String,
    pub ssl: bool,

    platform_ck: Box<dyn PlatformCallback>,
    open_lock: Arc<TMutex<u8>>,
    conn_state: TRwLock<State>,
    max_connecting_ms: TRwLock<u64>,
    last_connecting_ts: TRwLock<Option<u128>>,
}

impl ConnManager {
    #[allow(dead_code)]
    pub fn open(self: &Arc<Self>) {
        let slf = self.clone();
        tokio::spawn(async move {
            let _lock = slf.inner.open_lock.lock().await;
            if slf.is_opened().await {
                return;
            }
            slf.set_conn_state(State::Connecting, None).await;
            slf.reset_connecting_ts().await;
            slf.preemptive_open().await;
        });
    }

    // call multiple times create multiple connections, fastest wins as current connection
    async fn preemptive_open(self: &Arc<Self>) {
        log::info!("tcp preemptive opening");
        self.reset_last_connecting_ts().await;
        let timeout_ms = self.inner.max_connecting_ms.read().await;
        let conn = TcpConnection::new(self.inner.host.clone(), self.inner.ssl, *timeout_ms)
            .with_manager(Arc::downgrade(self));
        if conn.open().await {
            self.set_main_conn_if_none(Box::new(conn)).await;
        }
    }

    pub async fn set_main_conn_if_none(self: &Arc<Self>, conn: Box<dyn Connection>) {
        let manager = self.inner.clone();
        let mut main_conn = self.inner.main_conn.write().await;
        if main_conn.is_none() {
            *main_conn = Some(conn);
            tokio::task::spawn_blocking(move || manager.platform_ck.auth());
            // TODO: start ping
        }
    }

    pub async fn recv(self: &Arc<Self>, packet: Packet) {
        let manager = self.inner.clone();
        match packet {
            Packet::Message(message) => {
                tokio::task::spawn_blocking(move || manager.platform_ck.recv(message));
            }
            //TODO: ping pong
            _ => {}
        }
    }

    pub async fn reset_last_connecting_ts(&self) {
        let mut guard = self.inner.last_connecting_ts.write().await;
        *guard = Some(crate::now_ts())
    }

    pub async fn is_opened(&self) -> bool {
        matches!(
            self.conn_state().await,
            State::Connecting | State::Connected
        )
    }

    pub async fn is_connected(&self) -> bool {
        matches!(self.conn_state().await, State::Connected)
    }

    pub async fn conn_state(&self) -> State {
        let guard = self.inner.conn_state.read().await;
        *guard
    }

    pub async fn set_conn_state(&self, new_state: State, unique_id: Option<&str>) {
        match unique_id {
            Some(unique_id) => {
                let main_conn = self.inner.main_conn.read().await;
                if let Some(ref conn) = *main_conn {
                    if conn.is_same_conn(unique_id).await {
                        self._set_conn_state(new_state).await
                    }
                }
            }
            None => self._set_conn_state(new_state).await,
        }
    }

    async fn _set_conn_state(&self, new_state: State) {
        let mut guard = self.inner.conn_state.write().await;
        *guard = new_state;
    }

    pub async fn reset_connecting_ts(&self) {
        let mut guard = self.inner.connecting_ts.write().await;
        *guard = Some(crate::now_ts())
    }
}
