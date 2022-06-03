use crate::now_ts;
use crate::{tcp_conn::TcpConnection, Connection, PlatformCallback, State};
use client_proto::proto::packet::Packet;
use client_proto::proto::{Message, Ping};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc::{unbounded_channel as tchannel, UnboundedSender as TSender};
use tokio::sync::{Mutex as TMutex, RwLock as TRwLock};
use tokio::time::timeout;

const CONNECTIVITY_CHECK_MS: u64 = 500;
const PING_INTERVAL_SEC: u64 = 8;
const MAX_PENDING_PING_NUM: u8 = 3;

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
    ping_tx: TRwLock<Option<TSender<u8>>>,
    pending_ping: TMutex<u8>,
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

    pub async fn set_main_conn_if_none(self: &Arc<Self>, conn: Box<dyn Connection>) {
        let slf = self.clone();
        let mut main_conn = self.inner.main_conn.write().await;
        if main_conn.is_none() {
            *main_conn = Some(conn);
            slf.start_ping_pong();
            tokio::task::spawn_blocking(move || slf.inner.platform_ck.auth());
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

    pub async fn is_connected(&self) -> bool {
        matches!(self.conn_state().await, State::Connected)
    }

    pub async fn is_closed(&self) -> bool {
        matches!(self.conn_state().await, State::Closed)
    }

    fn start_ping_pong(self: &Arc<Self>) {
        let slf = self.clone();
        tokio::spawn(async move {
            let (ping_tx, mut ping_rx) = tchannel();
            let mut guard = slf.inner.ping_tx.write().await;
            *guard = Some(ping_tx);
            drop(guard);

            loop {
                let _ = timeout(Duration::from_secs(PING_INTERVAL_SEC), ping_rx.recv()).await;
                match slf.conn_state().await {
                    State::Connected if slf.pending_ping_num().await <= MAX_PENDING_PING_NUM => {
                        slf.inc_pending_ping().await;
                        slf.do_send(Packet::Ping(Ping {
                            timestamp: now_ts().to_be_bytes().to_vec(),
                        }))
                        .await;
                    }
                    _ => {
                        log::error!("tcp max ping fatal");
                        slf.reset_pending_ping().await;
                        tokio::spawn(async move { slf.on_fatal_error().await });
                        break;
                    }
                }
            }
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

    async fn send(&self, message: Message) {}

    async fn do_send(&self, packet: Packet) {}

    async fn reset_last_connecting_ts(&self) {
        let mut guard = self.inner.last_connecting_ts.write().await;
        *guard = Some(crate::now_ts())
    }

    async fn is_opened(&self) -> bool {
        matches!(
            self.conn_state().await,
            State::Connecting | State::Connected
        )
    }

    async fn _set_conn_state(&self, new_state: State) {
        let mut guard = self.inner.conn_state.write().await;
        *guard = new_state;
    }

    async fn reset_connecting_ts(&self) {
        let mut guard = self.inner.connecting_ts.write().await;
        *guard = Some(crate::now_ts())
    }

    async fn pending_ping_num(&self) -> u8 {
        let pending_ping = self.inner.pending_ping.lock().await;
        *pending_ping
    }

    async fn reset_pending_ping(&self) {
        let mut pending_ping = self.inner.pending_ping.lock().await;
        *pending_ping = 0;
    }

    async fn inc_pending_ping(&self) {
        let mut pending_ping = self.inner.pending_ping.lock().await;
        *pending_ping += 1;
    }

    async fn on_fatal_error(&self) {
        if self.is_closed().await {
            return;
        }
        self.try_disconnect().await;
    }

    async fn try_disconnect(&self) {
        let mut main_conn = self.inner.main_conn.write().await;
        main_conn.take(); // drop main_conn
    }
}
