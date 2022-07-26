use super::repository::MemoryRepository;
use async_trait::async_trait;
use dashmap::DashMap;
use fxhash::FxBuildHasher;
use server_base::proto::{
    BulkJoinReq, ConnRoomReq, ConnRooms, EmitReq, EmitResp, EmitSidReq, EmitStatus, GetRoomsReq,
    JoinReq, LeaveReq, Message, MessageReq, Rooms,
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

    fn emit_to(
        &self,
        namespace: &str,
        message: Message,
        conn: &Conn,
        hitting: &HashSet<String>,
    ) -> Option<(bool, HashSet<String>, EmitStatus)> {
        let mut sent_rooms: HashSet<String> = HashSet::new();
        if self.push_to_conn(conn.id(), message).is_ok() {
            sent_rooms = &sent_rooms | hitting;
            let mut status = EmitStatus {
                rooms: None,
                sent: true,
            };
            if let Some(c) = self.conn_rooms(conn, namespace) {
                status.rooms = Some(c);
            }
            Some((true, sent_rooms, status))
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

        let rooms: HashMap<String, Rooms> = if let Some(rooms) = channel_map {
            rooms
                .iter()
                .map(|(k, vs)| {
                    let cs = Rooms {
                        rooms: vs.iter().cloned().collect::<Vec<String>>(),
                    };
                    (k.clone(), cs)
                })
                .collect()
        } else {
            Default::default()
        };
        let request = MessageReq {
            sid: conn_id.to_string(),
            message: Some(Message {
                namespace: namespace.clone(),
                metadata: message.metadata,
                path: message.path,
                body: message.body.to_vec(),
                expire_at: None,
            }),
            rooms,
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

    fn join_operation(
        &self,
        conn_id: &str,
        namespace: &str,
        room_prefix: &str,
        channel: &str,
    ) -> bool {
        if let Some(conn) = self.conn(conn_id) {
            self.inner
                .repository
                .join_room(conn, namespace, room_prefix, channel);
            log::info!(
                "_fplink_join||conn_id={}||ns={}||cf={}|channel={}",
                conn_id,
                namespace,
                room_prefix,
                channel
            );
            return true;
        }
        false
    }

    fn bulk_join_operation(
        &self,
        conn_id: &str,
        namespace: &str,
        rooms: &HashMap<String, String>,
    ) -> bool {
        if let Some(conn) = self.conn(conn_id) {
            self.inner.repository.bulk_join_room(conn, namespace, rooms);
            return true;
        }
        false
    }

    pub fn leave_operation(&self, conn_id: &str, namespace: &str, k: &str, v: &str) -> bool {
        if let Some(conn) = self.conn(conn_id) {
            self.inner.repository.leave_room(&conn, namespace, k, v);
            return true;
        }
        false
    }

    pub fn search_by_room(
        &self,
        namespace: &str,
        room_prefix: &str,
        rooms: &[String],
    ) -> HashMap<Conn, HashSet<String>> {
        self.inner
            .repository
            .search_by_room(namespace, room_prefix, Some(rooms))
    }

    pub fn list_channels(
        &self,
        sid: &str,
        namespace: &str,
    ) -> Option<HashMap<String, HashSet<String>>> {
        self.inner.repository.list_channels(sid, namespace)
    }

    pub fn list_ns_channels(
        &self,
        conn_id: &str,
    ) -> Option<HashMap<String, HashMap<String, HashSet<String>>>> {
        self.inner.repository.list_ns_channels(conn_id)
    }

    fn conn_rooms(&self, conn: &Conn, namespace: &str) -> Option<ConnRooms> {
        self.list_channels(conn.id(), namespace)
            .map(|channel_map| ConnRooms {
                sid: conn.id().to_owned(),
                rooms: channel_map
                    .iter()
                    .map(|(k, vs)| {
                        let ts = Rooms {
                            rooms: vs.iter().cloned().collect::<Vec<String>>(),
                        };
                        (k.clone(), ts)
                    })
                    .collect(),
            })
    }
}

#[async_trait]
impl CoreOperation for CoreOperator {
    async fn join(&self, request: JoinReq) -> bool {
        let room_prefix = room_prefix(&request.room_prefix);
        let (connection_id, namespace, channel) = (&request.sid, &request.namespace, &request.room);
        self.join_operation(connection_id, namespace, room_prefix, channel)
    }

    async fn leave(&self, request: LeaveReq) -> bool {
        let room_prefix = room_prefix(&request.room_prefix);
        let (connection_id, namespace, channel) = (&request.sid, &request.namespace, &request.room);
        self.leave_operation(connection_id, namespace, room_prefix, channel)
    }

    async fn bulk_join(&self, request: BulkJoinReq) -> bool {
        let (connection_id, namespace, rooms) = (&request.sid, &request.namespace, &request.rooms);
        self.bulk_join_operation(connection_id, namespace, rooms)
    }

    async fn emit_sid(&self, request: EmitSidReq) -> bool {
        let message = match request.message {
            Some(m) => m,
            None => return false,
        };
        self.push_to_conn(&request.sid, message).is_ok()
    }

    async fn emit(&self, request: EmitReq) -> EmitResp {
        let message = match request.message {
            Some(message) => message,
            None => {
                return EmitResp {
                    success: false,
                    rooms: vec![],
                    status: vec![],
                };
            }
        };
        let namespace = &message.namespace;
        let room_prefix = match request.room_prefix {
            Some(ref cf) => cf,
            None => "default",
        };
        let conns_with_hitting = self.search_by_room(namespace, room_prefix, &request.rooms);
        let mut success = true;
        let mut status: Vec<EmitStatus> = vec![];
        let mut sent_rooms = HashSet::new(); // TODO allocate before updating element

        for (conn, hitting) in conns_with_hitting {
            if let Some((is_success, sent, s)) =
                self.emit_to(namespace, message.clone(), &conn, &hitting)
            {
                success = success && is_success;
                status.push(s);
                sent_rooms = &sent_rooms | &sent;
            }
        }

        EmitResp {
            success,
            rooms: sent_rooms.into_iter().collect::<Vec<_>>(),
            status,
        }
    }

    async fn get_conn_rooms(&self, request: ConnRoomReq) -> Vec<ConnRooms> {
        let namespace = &request.namespace;
        let channel = &request.room;
        let room_prefix = room_prefix(&request.room_prefix);

        let hit_conns = self.search_by_room(namespace, room_prefix, &[channel.to_owned()]);
        hit_conns
            .iter()
            .filter_map(|(conn, _)| self.conn_rooms(conn, namespace))
            .collect::<Vec<_>>()
    }

    async fn get_rooms(&self, request: GetRoomsReq) -> Vec<String> {
        let ns = &request.namespace;
        let room_prefix = room_prefix(&request.room_prefix);
        let mut rooms_ref = None;
        if let Some(channels) = &request.rooms {
            rooms_ref = Some(channels.rooms.as_slice())
        }
        let map: HashMap<Conn, HashSet<String>> =
            self.inner
                .repository
                .search_by_room(ns, room_prefix, rooms_ref);

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
    async fn push(&self, req: EmitSidReq) {
        self.emit_sid(req).await;
    }
}

fn room_prefix(room_prefix: &Option<String>) -> &str {
    match room_prefix {
        Some(ref cf) => cf,
        None => "default",
    }
}
