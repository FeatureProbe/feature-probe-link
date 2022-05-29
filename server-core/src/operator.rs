use super::repository::MemoryRepository;
use async_trait::async_trait;
use dashmap::DashMap;
use fxhash::FxBuildHasher;
use server_base::proto::{
    BulkSubReq, Channels, ConnChannels, GetChannelsReq, GetConnsReq, Message, MessageReq, PubReq,
    PubResp, PubStatus, PushConnReq, SubReq, UnSubReq,
};
use server_base::{tokio, Conn, CoreOperation, Dispatch, PushConn};
use std::borrow::ToOwned;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

type ConnectionId = String;

#[derive(Clone)]
pub struct CoreOperator {
    inner: Arc<Inner>,
}

struct Inner {
    conns: DashMap<ConnectionId, Conn, FxBuildHasher>,
    repository: MemoryRepository,
    dispatcher: Box<dyn Dispatch>,
}

impl CoreOperator {
    pub fn new(dispatcher: Box<dyn Dispatch>) -> Self {
        let conns = DashMap::default();
        Self {
            inner: Arc::new(Inner {
                repository: MemoryRepository::new(),
                conns,
                dispatcher,
            }),
        }
    }
}

impl CoreOperator {
    pub fn conn(&self, conn_id: &str) -> Option<Conn> {
        self.inner.conns.get(conn_id).map(|conn| (*conn).clone())
    }

    fn publish_to(
        &self,
        namespace: &str,
        message: Message,
        conn: &Conn,
        hitting: &HashSet<String>,
    ) -> Option<(bool, HashSet<String>, PubStatus)> {
        let mut sent_channels: HashSet<String> = HashSet::new();
        if self.push_to_conn(conn.id(), message).is_ok() {
            sent_channels = &sent_channels | hitting;
            let mut status = PubStatus {
                conn_channels: None,
                sent: true,
            };
            if let Some(c) = self.conn_channels(conn, namespace) {
                status.conn_channels = Some(c);
            }
            Some((true, sent_channels, status))
        } else {
            // sender use unbounded_channel, send will not return error unless closed
            // only when connection is gone, push failed is ok
            Option::None
        }
    }

    //---------------connection outgoing operation-----------------------------

    #[allow(clippy::result_unit_err)]
    pub fn push_to_conn(&self, conn_id: &str, msg: Message) -> Result<(), ()> {
        log::trace!("push_to_conn: conn_id: {}, {:?}", conn_id, msg);
        if let Some(conn) = self.conn(conn_id) {
            conn.push(msg)
        } else {
            log::warn!("Connection Id:{} not found", conn_id);
            Err(())
        }
    }

    //---------------connection incoming operation-----------------------------

    pub fn reg_conn(&self, conn: Conn) {
        log::info!("reg_conn||conn_id={}", conn.id());
        self.inner.conns.insert(conn.id().to_string(), conn);
    }

    pub fn unreg_conn(&self, conn: &Conn) {
        log::info!("unreg_conn||conn_id={}", conn.id());
        self.inner.repository.remove_channels(conn);
        self.inner.conns.remove(conn.id());
    }

    pub fn dispatch_message(&self, conn_id: &str, message: Message) {
        let namespace = message.namespace.clone();
        let channel_map = self.list_channels(conn_id, &namespace);

        let channels: HashMap<String, Channels> = if let Some(channels) = channel_map {
            channels
                .iter()
                .map(|(k, vs)| {
                    let cs = Channels {
                        channels: vs.iter().cloned().collect::<Vec<String>>(),
                    };
                    (k.clone(), cs)
                })
                .collect()
        } else {
            Default::default()
        };
        let request = MessageReq {
            cid: conn_id.to_string(),
            message: Some(Message {
                namespace: namespace.clone(),
                metadata: message.metadata,
                path: message.path,
                body: message.body.to_vec(),
            }),
            channels,
            trace: None,
        };
        let s = self.clone();
        tokio::spawn(async move {
            s.inner
                .dispatcher
                .dispatch(namespace.clone(), request)
                .await;
        });
    }

    //---------------channel operation-----------------------------

    fn subscribe_operation(
        &self,
        conn_id: &str,
        namespace: &str,
        channel_family: &str,
        channel: &str,
    ) -> bool {
        if let Some(conn) = self.conn(conn_id) {
            self.inner
                .repository
                .subscribe_channel(conn, namespace, channel_family, channel);
            log::info!(
                "_fplink_subscribe||conn_id={}||ns={}||cf={}|channel={}",
                conn_id,
                namespace,
                channel_family,
                channel
            );
            return true;
        }
        false
    }

    fn bulk_subscribe_operation(
        &self,
        conn_id: &str,
        namespace: &str,
        channels: &HashMap<String, String>,
    ) -> bool {
        if let Some(conn) = self.conn(conn_id) {
            self.inner
                .repository
                .bulk_subscribe_channel(conn, namespace, channels);
            return true;
        }
        false
    }

    pub fn unsubscribe_operation(&self, conn_id: &str, namespace: &str, k: &str, v: &str) -> bool {
        if let Some(conn) = self.conn(conn_id) {
            self.inner
                .repository
                .unsubscribe_channel(&conn, namespace, k, v);
            return true;
        }
        false
    }

    pub fn search_by_channel(
        &self,
        namespace: &str,
        channel_family: &str,
        channels: &[String],
    ) -> HashMap<Conn, HashSet<String>> {
        self.inner
            .repository
            .search_by_channel(namespace, channel_family, Some(channels))
    }

    pub fn list_channels(
        &self,
        cid: &str,
        namespace: &str,
    ) -> Option<HashMap<String, HashSet<String>>> {
        self.inner.repository.list_channels(cid, namespace)
    }

    pub fn list_ns_channels(
        &self,
        conn_id: &str,
    ) -> Option<HashMap<String, HashMap<String, HashSet<String>>>> {
        self.inner.repository.list_ns_channels(conn_id)
    }

    fn conn_channels(&self, conn: &Conn, namespace: &str) -> Option<ConnChannels> {
        self.list_channels(conn.id(), namespace)
            .map(|channel_map| ConnChannels {
                cid: conn.id().to_owned(),
                channels: channel_map
                    .iter()
                    .map(|(k, vs)| {
                        let ts = Channels {
                            channels: vs.iter().cloned().collect::<Vec<String>>(),
                        };
                        (k.clone(), ts)
                    })
                    .collect(),
            })
    }
}

#[async_trait]
impl CoreOperation for CoreOperator {
    async fn subscribe(&self, request: SubReq) -> bool {
        let channel_family = channel_family(&request.channel_family);
        let (connection_id, namespace, channel) =
            (&request.cid, &request.namespace, &request.channel);
        self.subscribe_operation(connection_id, namespace, channel_family, channel)
    }

    async fn unsubscribe(&self, request: UnSubReq) -> bool {
        let channel_family = channel_family(&request.channel_family);
        let (connection_id, namespace, channel) =
            (&request.cid, &request.namespace, &request.channel);
        self.unsubscribe_operation(connection_id, namespace, channel_family, channel)
    }

    async fn bulk_subscribe(&self, request: BulkSubReq) -> bool {
        let (connection_id, namespace, channels) =
            (&request.cid, &request.namespace, &request.channels);
        self.bulk_subscribe_operation(connection_id, namespace, channels)
    }

    async fn push_conn(&self, request: PushConnReq) -> bool {
        let message = match request.message {
            Some(m) => m,
            None => return false,
        };
        self.push_to_conn(&request.cid, message).is_ok()
    }

    async fn publish(&self, request: PubReq) -> PubResp {
        let message = match request.message {
            Some(message) => message,
            None => {
                return PubResp {
                    success: false,
                    channels: vec![],
                    status: vec![],
                };
            }
        };
        let namespace = &message.namespace;
        let channel_family = match request.channel_family {
            Some(ref cf) => cf,
            None => "default",
        };
        let conns_with_hitting =
            self.search_by_channel(namespace, channel_family, &request.channels);
        let mut success = true;
        let mut status: Vec<PubStatus> = vec![];
        let mut sent_channels = HashSet::new(); // TODO allocate before updating element

        for (conn, hitting) in conns_with_hitting {
            if let Some((is_success, sent, s)) =
                self.publish_to(namespace, message.clone(), &conn, &hitting)
            {
                success = success && is_success;
                status.push(s);
                sent_channels = &sent_channels | &sent;
            }
        }

        PubResp {
            success,
            channels: sent_channels.into_iter().collect::<Vec<_>>(),
            status,
        }
    }

    async fn get_conn_channels(&self, request: GetConnsReq) -> Vec<ConnChannels> {
        let namespace = &request.namespace;
        let channel = &request.channel;
        let channel_family = channel_family(&request.channel_family);

        let hit_conns = self.search_by_channel(namespace, channel_family, &[channel.to_owned()]);
        hit_conns
            .iter()
            .filter_map(|(conn, _)| self.conn_channels(conn, namespace))
            .collect::<Vec<_>>()
    }

    async fn get_channels(&self, request: GetChannelsReq) -> Vec<String> {
        let ns = &request.namespace;
        let channel_family = channel_family(&request.channel_family);
        let mut channels_ref = None;
        if let Some(channels) = &request.channels {
            channels_ref = Some(channels.channels.as_slice())
        }
        let map: HashMap<Conn, HashSet<String>> =
            self.inner
                .repository
                .search_by_channel(ns, channel_family, channels_ref);

        let mut exist_channels = HashSet::new();
        for vals in map.values() {
            for val in vals {
                exist_channels.insert(val.to_owned());
            }
        }

        exist_channels.into_iter().collect()
    }
}

#[async_trait]
impl PushConn for CoreOperator {
    async fn push(&self, req: PushConnReq) {
        self.push_conn(req).await;
    }
}

fn channel_family(channel_family: &Option<String>) -> &str {
    match channel_family {
        Some(ref cf) => cf,
        None => "default",
    }
}
