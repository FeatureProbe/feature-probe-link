use crate::{now_ts, Error, NetworkType};
use crate::{tcp_conn::TcpConnection, Connection, PlatformCallback, State};
use client_proto::proto::packet::Packet;
use client_proto::proto::{Message, Ping};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc::{unbounded_channel as tchannel, UnboundedSender as TSender};
use tokio::sync::{Mutex, RwLock};
use tokio::time::{interval, timeout};

const CONNECTIVITY_CHECK_MS: u64 = 1000;
const PING_INTERVAL_SEC: u64 = 3;
const MAX_PENDING_PING_NUM: u8 = 3;

#[derive(Clone, Default)]
pub struct ConnManager {
    inner: Arc<ConnManagerInner>,
}

#[derive(Default)]
pub struct ConnManagerInner {
    pub main_conn: RwLock<Option<Box<dyn Connection>>>,
    pub timeout_ms: u64,
    pub host: String,
    pub ssl: bool,

    platform_ck: Option<Box<dyn PlatformCallback>>,
    open_lock: Arc<Mutex<u8>>,
    conn_state: RwLock<State>,
    ping_tx: RwLock<Option<TSender<u8>>>,
    pending_ping: Mutex<u8>,
    network_type: Mutex<NetworkType>,
}

impl ConnManager {
    #[allow(dead_code)]
    pub fn open(self: &Arc<Self>) {
        log::trace!("manager open");
        let slf = self.clone();
        tokio::spawn(async move {
            let _lock = slf.inner.open_lock.lock().await;
            if slf.is_opened().await {
                return;
            }
            slf.preemptive_open().await;
            slf.auto_reconnect();
            slf.start_ping_pong();
        });
    }

    #[allow(dead_code)]
    pub fn send(&self, message: Message) {
        // TODO: send queue
        let slf = self.clone();
        tokio::spawn(async move {
            let _ = slf.send_packet(Packet::Message(message));
        });
    }

    pub async fn set_main_conn_if_none(self: &Arc<Self>, conn: Box<dyn Connection>) {
        let slf = self.clone();
        log::trace!("set_main_conn_if_none get lock");
        let mut main_conn = self.inner.main_conn.write().await;
        log::trace!("set_main_conn_if_none got lock");
        if main_conn.is_none() {
            *main_conn = Some(conn);
            tokio::task::spawn_blocking(move || match slf.inner.platform_ck {
                Some(ref cb) => cb.auth(),
                None => {}
            });
        }
    }

    pub async fn recv(self: &Arc<Self>, packet: Result<Packet, Error>) {
        let slf = self.clone();
        match packet {
            Ok(Packet::Message(message)) => {
                tokio::task::spawn_blocking(move || match slf.inner.platform_ck {
                    Some(ref cb) => cb.recv(message),
                    None => {}
                });
            }
            Ok(Packet::Pong(pong)) => {
                log::debug!("recv pong, rtt is {}", now_ts() - pong.timestamp);
                slf.reset_pending_ping().await;
            }
            Err(e) => {
                log::error!("manager {:?}", e);
                self.cleanup().await
            }
            _ => log::info!("unsupport message type: {:?}", packet),
        }
    }

    pub async fn conn_state(&self) -> State {
        let guard = self.inner.conn_state.read().await;
        *guard
    }

    pub async fn set_conn_state(&self, new_state: State, unique_id: Option<&str>) {
        log::trace!("set_conn_state get lock");
        let main_conn = self.inner.main_conn.read().await;
        log::trace!("set_conn_state got lock");
        log::trace!(
            "manager set_conn_state {:?} by {:?} current main_conn {:?}",
            new_state,
            unique_id,
            main_conn.is_some()
        );
        match unique_id {
            Some(unique_id) if main_conn.is_some() => {
                if let Some(ref conn) = *main_conn {
                    if conn.is_same_conn(unique_id).await {
                        self._set_conn_state(new_state).await
                    }
                }
            }
            _ => self._set_conn_state(new_state).await,
        }
    }

    pub async fn is_connected(&self) -> bool {
        matches!(self.conn_state().await, State::Connected)
    }

    pub async fn is_closed(&self) -> bool {
        matches!(self.conn_state().await, State::Closed)
    }

    pub async fn drop_main_conn(&self) {
        log::trace!("manager drop main_conn get lock");
        let mut main_conn = self.inner.main_conn.write().await;
        log::trace!("manager drop main_conn got lock");
        main_conn.take(); // drop main_conn
    }

    async fn cleanup(&self) {
        if self.is_closed().await {
            return;
        }
        self.drop_main_conn().await;
        self.set_conn_state(State::DisConnected, None).await;
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
                    State::Connected if slf.pending_ping_num().await < MAX_PENDING_PING_NUM => {
                        slf.inc_pending_ping().await;
                        slf.do_send(Packet::Ping(Ping {
                            timestamp: now_ts(),
                        }))
                        .await;
                    }
                    State::Connected => {
                        log::error!("max pending ping");
                        slf.reset_pending_ping().await;
                        tokio::spawn(async move { slf.cleanup().await });
                        break;
                    }
                    _ => slf.reset_pending_ping().await,
                }
            }
        });
    }

    // call multiple times create multiple connections, fastest wins as current connection
    async fn preemptive_open(self: &Arc<Self>) {
        self.set_conn_state(State::Connecting, None).await;
        log::debug!("preemptive opening");
        let conn = TcpConnection::new(
            self.inner.host.clone(),
            self.inner.ssl,
            self.inner.timeout_ms,
        )
        .with_manager(Arc::downgrade(self));
        if conn.open().await {
            self.set_main_conn_if_none(Box::new(conn)).await;
        }
    }

    fn auto_reconnect(self: &Arc<Self>) {
        let slf = self.clone();
        tokio::spawn(async move {
            let mut interval = interval(Duration::from_millis(CONNECTIVITY_CHECK_MS));
            let mut last_run_time = Instant::now();
            loop {
                if slf.is_closed().await {
                    break;
                }
                let now = Instant::now();
                if now.duration_since(last_run_time).as_millis() >= CONNECTIVITY_CHECK_MS as u128 {
                    last_run_time = now;
                    slf.reconnect_if_need().await;
                }
                interval.tick().await;
            }
        });
    }

    async fn reconnect_if_need(self: &Arc<Self>) {
        // if state is closed, mean no need to reconnect
        // only reconnect when state is disconnected
        log::trace!("reconnect_if_need get read lock");
        let main_conn = self.inner.main_conn.read().await;
        log::trace!("reconnect_if_need got read lock");
        log::debug!(
            "main_conn {:?} current state {:?}",
            main_conn.is_some(),
            self.conn_state().await
        );
        if (self.conn_state().await == State::DisConnected
            || self.conn_state().await == State::Connecting)
            && self.network_type().await != NetworkType::TypeNoNet
        {
            log::trace!("need reconnect");
            let slf = self.clone();
            tokio::spawn(async move { slf.preemptive_open().await });
        }
    }

    #[allow(dead_code)]
    async fn set_network_type(&self, network_type: NetworkType) -> NetworkType {
        let mut guard = self.inner.network_type.lock().await;
        let old_network_type = *guard;
        *guard = network_type;
        old_network_type
    }

    async fn network_type(&self) -> NetworkType {
        let guard = self.inner.network_type.lock().await;
        *guard
    }

    async fn do_send(self: &Arc<Self>, packet: Packet) {
        let slf = self.clone();
        tokio::spawn(async move { slf.send_packet(packet).await });
    }

    async fn send_packet(&self, packet: Packet) {
        log::trace!("do_send get lock");
        let main_conn = self.inner.main_conn.read().await;
        log::trace!("do_send got lock");
        match *main_conn {
            Some(ref conn) => {
                if let Err(e) = conn.send(packet).await {
                    log::error!("send error: {}", e.to_string());
                    drop(main_conn); // drop lock
                    self.cleanup().await;
                }
            }
            None => log::info!("send failed, conn not ready"),
        };
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
}

#[cfg(test)]
mod tests {

    use super::*;
    use std::time::Duration;

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn test_manager() {
        let _ = tracing_subscriber::fmt().with_env_filter("trace").init();
        let manager = Arc::new(ConnManager {
            inner: Arc::new(ConnManagerInner {
                host: "127.0.0.1:8082".to_owned(),
                timeout_ms: 100,
                ..Default::default()
            }),
        });

        manager.open();

        let message = Message {
            namespace: "__ECHO".to_owned(),
            path: "/test".to_owned(),
            metadata: Default::default(),
            body: Default::default(),
            expire_at: None,
        };

        // loop {
        //     manager.send(message.clone());
        //     tokio::time::sleep(Duration::from_secs(2)).await;
        // }

        manager.send(message.clone());
        manager.send(message.clone());
        manager.send(message);

        tokio::time::sleep(Duration::from_secs(2)).await;
    }
}
