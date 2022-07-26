use dashmap::DashMap;
use fxhash::FxBuildHasher;
use server_base::Conn;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

fn gen_index_key(namespace: &str, k: &str, v: &str) -> String {
    format!("{}::{}::{}", namespace, k, v)
}

type ConnId = String;
type Namespace = String;
type RoomPrefix = String;
type Rooms = HashSet<String>;
type IndexKey = String;
type NsChannelMap = HashMap<Namespace, HashMap<RoomPrefix, Rooms>>;

#[derive(Debug, Clone)]
pub struct MemoryRepository {
    storage: Arc<DashMap<ConnId, NsChannelMap, FxBuildHasher>>,
    indexing: Arc<DashMap<IndexKey, HashSet<Conn>, FxBuildHasher>>,
}

impl MemoryRepository {
    pub fn new() -> Self {
        MemoryRepository {
            storage: Arc::new(DashMap::default()),
            indexing: Arc::new(DashMap::default()),
        }
    }

    pub fn search_by_room(
        &self,
        ns: &str,
        room_prefix: &str,
        rooms: Option<&[String]>,
    ) -> HashMap<Conn, Rooms> {
        let mut rv: HashMap<Conn, HashSet<String>> = HashMap::new();
        if rooms.is_none() {
            return rv;
        }
        for channel in rooms.unwrap() {
            if let Some(conns) = self.indexing.get(&gen_index_key(ns, room_prefix, channel)) {
                conns.iter().for_each(|conn| {
                    rv.entry(conn.clone()).or_default().insert(channel.clone());
                });
            }
        }
        rv
    }

    #[allow(dead_code)]
    pub fn search_all_channels(&self, ns: &str, room_prefix: &str) -> Rooms {
        let mut rooms: HashSet<String> = HashSet::new();
        self.storage.iter().for_each(|ns_channels| {
            if let Some(ns_channels) = ns_channels.get(ns) {
                if let Some(c) = ns_channels.get(room_prefix) {
                    rooms.extend(c.clone())
                }
            }
        });
        rooms
    }

    pub fn join_room(&self, conn: Conn, ns: &str, room_prefix: &str, channel: &str) {
        let mut channel_map = HashMap::new();
        channel_map.insert(room_prefix.to_owned(), channel.to_owned());
        self.store(&conn, ns, &channel_map);
        self.index(conn, ns, room_prefix, channel);
    }

    pub fn leave_room(&self, conn: &Conn, ns: &str, room_prefix: &str, channel: &str) {
        let mut removed = None;
        if let Some(mut ns_channels) = self.storage.get_mut(conn.id()) {
            if let Some(rooms) = ns_channels.get_mut(ns) {
                if let Some(rooms) = rooms.get_mut(room_prefix) {
                    if rooms.remove(channel) {
                        removed = Some(channel);
                    }
                }
            }
        }
        if let Some(removed_channel) = removed {
            self.remove_index(conn, ns, room_prefix, removed_channel);
        }
    }

    pub fn bulk_join_room(&self, conn: Conn, ns: &str, rooms: &HashMap<String, String>) {
        self.store(&conn, ns, rooms);
        rooms.iter().for_each(|(room_prefix, channel)| {
            self.index(conn.clone(), ns, room_prefix, channel);
        });
    }

    pub fn remove_channels(&self, conn: &Conn) {
        if let Some((_conn_id, ns_channels)) = self.storage.remove(conn.id()) {
            for (ns, rooms) in ns_channels.iter() {
                for (room_prefix, rooms) in rooms.iter() {
                    for channel in rooms.iter() {
                        self.remove_index(conn, ns, room_prefix, channel);
                    }
                }
            }
        }
    }

    pub fn list_ns_channels(&self, sid: &str) -> Option<NsChannelMap> {
        // too expansive to trace latency here
        self.storage
            .get(sid)
            .map(|ns_channels| (*ns_channels).clone())
    }

    pub fn list_channels(&self, sid: &str, ns: &str) -> Option<HashMap<RoomPrefix, Rooms>> {
        // too expansive to trace latency here
        self.storage.get(sid).and_then(|conn| conn.get(ns).cloned())
    }

    fn store(&self, conn: &Conn, ns: &str, rooms: &HashMap<String, String>) {
        let mut ns_channels = self.storage.entry(conn.id().to_owned()).or_default();
        let ns_channel = ns_channels.entry(ns.to_owned()).or_default();
        rooms.iter().for_each(|(room_prefix, channel)| {
            let rooms = ns_channel.entry(room_prefix.to_owned()).or_default();
            rooms.insert(channel.to_owned());
        });
    }

    fn index(&self, conn: Conn, ns: &str, chanel_family: &str, channel: &str) {
        self.indexing
            .entry(gen_index_key(ns, chanel_family, channel))
            .or_default()
            .insert(conn);
    }

    fn remove_index(&self, conn: &Conn, ns: &str, room_prefix: &str, channel: &str) {
        let index_key = gen_index_key(ns, room_prefix, channel);
        let mut hit = false;
        let mut empty = false;
        if let Some(mut conns) = self.indexing.get_mut(&index_key) {
            hit = conns.remove(conn);
            empty = conns.is_empty();
        }
        if empty {
            self.indexing.remove(&index_key);
        }
        if !hit {
            // meter!("remove_index_miss")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use server_base::{proto::Message, Protocol};
    use server_base::{Conn, ConnContext, LifeCycle, SendMessage};

    #[test]
    fn test_gen_index_key() {
        assert_eq!("ns::key::val", gen_index_key("ns", "key", "val"));
    }

    #[test]
    fn test_bind() {
        let m = bind_some();
        assert!(m.storage.contains_key("conn_1"));
        assert!(m.storage.contains_key("conn_2"));
        assert!(m.indexing.contains_key("ns::uid::2001"));
        assert!(m.indexing.contains_key("ns::uid::2002"));
        assert!(m.indexing.contains_key("ns::os::ios"));
    }

    #[test]
    fn test_unbind() {
        let m = bind_some();
        let conn_2 = mock_conn_sender("conn_2");
        m.leave_room(&conn_2, "ns", "os", "ios");
        m.leave_room(&conn_2, "ns", "uid", "2002");
        assert!(m.storage.contains_key("conn_1"));
        assert!(m.storage.contains_key("conn_2"));
        assert!(m.indexing.contains_key("ns::uid::2001"));
        assert!(!m.indexing.contains_key("ns::uid::2002"));
        assert!(!m.indexing.contains_key("ns::os::ios"));
    }

    #[test]
    fn test_remove_conn_rooms() {
        let m = bind_some();
        let conn_2 = mock_conn_sender("conn_2");
        m.remove_channels(&conn_2);
        assert!(m.storage.contains_key("conn_1"));
        assert!(!m.storage.contains_key("conn_2"));
        assert!(m.indexing.contains_key("ns::uid::2001"));
        assert!(!m.indexing.contains_key("ns::uid::2002"));
        assert!(!m.indexing.contains_key("ns::os::ios"));
    }

    #[test]
    fn test_list_channels_by_id() {
        let m = bind_some();
        let res = m.list_channels("conn_2", "ns").unwrap();
        let mut expect: HashMap<String, HashSet<String>> = HashMap::new();
        expect
            .entry("uid".to_owned())
            .or_default()
            .insert("2002".to_owned());
        expect
            .entry("os".to_owned())
            .or_default()
            .insert("ios".to_owned());
        assert_eq!(expect, res);
    }

    #[test]
    fn test_list_all_channels_by_id() {
        let m = bind_some();
        let res = m.list_ns_channels("conn_2").unwrap();
        let mut expect: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();
        expect
            .entry("ns".to_owned())
            .or_default()
            .entry("uid".to_owned())
            .or_default()
            .insert("2002".to_owned());
        expect
            .entry("ns".to_owned())
            .or_default()
            .entry("os".to_owned())
            .or_default()
            .insert("ios".to_owned());
        assert_eq!(expect, res);
    }

    #[test]
    fn test_search_all_channelues() {
        let m = bind_some();
        let res = m.search_all_channels("ns", "uid");
        let mut expect = HashSet::new();
        expect.insert("2001".to_owned());
        expect.insert("2002".to_owned());
        assert_eq!(expect, res);
    }

    #[test]
    fn test_search_by_channels() {
        let m = bind_some();
        let res = m.search_by_room("ns", "uid", Some(&["2001".to_owned(), "2002".to_owned()]));
        let mut expect: HashMap<Conn, HashSet<String>> = HashMap::new();
        expect
            .entry(mock_conn_sender("conn_1"))
            .or_default()
            .insert("2001".to_owned());
        expect
            .entry(mock_conn_sender("conn_2"))
            .or_default()
            .insert("2002".to_owned());
        assert_eq!(expect, res);
    }

    fn bind_some() -> MemoryRepository {
        let m = MemoryRepository::new();
        let conn = mock_conn_sender("conn_1");
        let ns = "ns";
        m.join_room(conn, ns, "uid", "2001");
        let mut rooms = HashMap::new();
        rooms.insert("uid".to_owned(), "2002".to_owned());
        rooms.insert("os".to_owned(), "ios".to_owned());
        let conn = mock_conn_sender("conn_2");
        m.bulk_join_room(conn, ns, &rooms);
        m
    }

    #[test]
    fn test_index() {
        let m = MemoryRepository::new();
        let conn = mock_conn_sender("conn_1");
        let ns = "ns";
        let room_prefix = "key";
        let channel_1 = "value1";
        let channel_2 = "value2";
        m.index(conn.clone(), ns, room_prefix, channel_1);
        m.index(conn.clone(), ns, room_prefix, channel_2);
        assert!(m
            .indexing
            .contains_key(&gen_index_key(ns, room_prefix, channel_1)));
        assert!(m
            .indexing
            .contains_key(&gen_index_key(ns, room_prefix, channel_2)));

        m.remove_index(&conn, ns, room_prefix, channel_1);
        assert!(!m
            .indexing
            .contains_key(&gen_index_key(ns, room_prefix, channel_1)));
        assert!(m
            .indexing
            .contains_key(&gen_index_key(ns, room_prefix, channel_2)));
    }

    #[derive(Default)]
    struct MockConnLifeCycle {}

    impl LifeCycle for MockConnLifeCycle {
        fn new_conn_id(&self, _protocol: Protocol) -> String {
            "some_conn_id".to_owned()
        }

        fn on_conn_create(&self, _conn: Conn) {}

        fn on_message_incoming(&self, _conn_id: &str, _protocol: &Protocol, _message: Message) {}

        fn on_conn_destroy(&self, _conn: Conn) {}

        fn should_timeout(&self) -> bool {
            false
        }
    }

    #[derive(Default)]
    struct Tcp {}

    impl SendMessage for Tcp {
        fn send(&self, _msg: Message) -> Result<(), ()> {
            Ok(())
        }
    }

    fn mock_conn_sender(conn_id: &str) -> Conn {
        Conn {
            inner: Arc::new(ConnContext {
                proto: Protocol::Tcp,
                timeout: 60,
                create_time: 1234,
                conn_id: conn_id.to_string(),
                sender: Box::new(Tcp::default()),
                lifecycle: Arc::new(MockConnLifeCycle::default()),
                peer_addr: None,
            }),
        }
    }

    #[test]
    fn test_concurrent() {
        let m = MemoryRepository::new();
        let mut joins = Vec::new();
        let thread_num = 50;
        let cycle_num = 100;
        for i in 0..thread_num {
            let mc = m.clone();
            let start = i * cycle_num;
            let end = start + cycle_num;
            let join = std::thread::spawn(move || {
                for i in start..end {
                    let conn = mock_conn_sender(&format!("conn_{}", i));
                    let mut rooms = HashMap::new();
                    rooms.insert("uid".to_owned(), format!("uid_{}", i));
                    rooms.insert("os".to_owned(), "ios".to_owned());
                    mc.bulk_join_room(conn, "ns", &rooms);
                }
            });
            joins.push(join);
        }
        for join in joins {
            join.join().unwrap();
        }
        for i in 0..cycle_num * thread_num {
            assert!(m.storage.contains_key(&format!("conn_{}", i)));
            assert!(m.indexing.contains_key(&format!("ns::uid::uid_{}", i)));
        }

        let mut joins = Vec::new();
        for i in 0..thread_num {
            let mc = m.clone();
            let start = i * cycle_num;
            let end = start + cycle_num;
            let join = std::thread::spawn(move || {
                for i in start..end {
                    let conn = mock_conn_sender(&format!("conn_{}", i));
                    mc.leave_room(&conn, "ns", "uid", &format!("uid_{}", i));
                }
            });
            joins.push(join);
        }
        for join in joins {
            join.join().unwrap();
        }
        for i in 0..cycle_num * thread_num {
            assert!(m.storage.contains_key(&format!("conn_{}", i)));
            assert!(!m.indexing.contains_key(&format!("ns::uid::uid_{}", i)));
        }
    }
}
