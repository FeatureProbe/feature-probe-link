use crate::{Connect, GrpcClient};
use parking_lot::RwLock;
use server_base::HandyRwLock;
use std::collections::HashMap;
use std::fmt;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

pub type Services = Service<GrpcClient>;

struct ServiceClient<T: Connect + Default> {
    client: Arc<T::Item>,
    zone: Option<String>,
}

struct ServiceClients<T: Connect + Default> {
    clients: HashMap<String, Arc<ServiceClient<T>>>,
    fallback_client: Option<(String, Arc<T::Item>)>,
    counter: AtomicUsize,
}

impl<T: Connect + Default> Default for ServiceClients<T> {
    fn default() -> Self {
        Self {
            clients: Default::default(),
            fallback_client: None,
            counter: AtomicUsize::new(0),
        }
    }
}
impl<T: Connect + Default> ServiceClients<T> {
    fn set_fallback(&mut self, addr: &str, client: Arc<T::Item>) {
        self.fallback_client = Some((addr.into(), client));
    }

    fn pick_client(&self, zone: Option<String>) -> Option<(String, Arc<T::Item>)> {
        let total_count = self.clients.len();
        let zone_clients = self
            .clients
            .iter()
            .filter(|(_, client)| client.zone.eq(&zone))
            .collect::<Vec<_>>();
        let zone_count = zone_clients.len();
        if total_count == 0 {
            match &self.fallback_client {
                None => {
                    log::error!("pick_client no alive  clients, no fallback");
                    None
                }
                Some((addr, client)) => {
                    log::error!("pick_client no alive  clients, with fallback {:?}", addr);
                    Some((addr.to_string(), Arc::clone(client)))
                }
            }
        } else if zone_count == 0 {
            let index = self.counter.fetch_add(1, Ordering::SeqCst) % total_count;
            self.clients.iter().nth(index).map(|(addr, client)| {
                log::trace!("pick_client {} ignore zone {:?}", addr, zone);
                (addr.to_string(), Arc::clone(&client.client))
            })
        } else {
            let index = self.counter.fetch_add(1, Ordering::SeqCst) % zone_count;
            zone_clients.get(index).map(|(addr, client)| {
                log::trace!("pick_client {} with zone {:?}", addr, zone);
                (addr.to_string(), Arc::clone(&client.client))
            })
        }
    }
}

impl<T: Connect + Default> fmt::Display for ServiceClients<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let addrs: Vec<String> = self.clients.keys().cloned().collect();
        let fallback = match self.fallback_client {
            Some((ref ip, _)) => ip.clone(),
            None => "no fallback".to_string(),
        };
        write!(f, "addrs: {:?}, fallback: {}", addrs, fallback)
    }
}

#[derive(Clone)]
pub struct Service<T: Connect + Default> {
    inner: Arc<RwLock<Inner<T>>>,
}

impl<T: Connect + Default> Service<T> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn add_fallback(&self, namespace: &str, addr: &str) {
        self.inner.wl().add_fallback(namespace, addr);
    }

    pub fn add_client(&self, namespace: &str, addr: &str, zone: Option<String>) {
        self.inner.wl().add_client(namespace, addr, zone);
    }

    pub fn remove_client(&self, namespace: &str, addr: &str) {
        self.inner.wl().remove_client(namespace, addr);
    }

    pub fn pick_client(
        &self,
        namespace: &str,
        zone: Option<String>,
    ) -> Option<(String, Arc<<T as Connect>::Item>)> {
        self.inner
            .rl()
            .pick_client(namespace, zone)
            .map(|(addr, client)| (addr, Arc::clone(&client)))
    }

    pub fn all_clients(&self) -> HashMap<String, String> {
        self.inner
            .rl()
            .0
            .iter()
            .map(|(ns, ns_clients)| (ns.clone(), ns_clients.to_string()))
            .collect()
    }
}

impl<T: Default + Connect> Default for Service<T> {
    fn default() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner::<T>::new())),
        }
    }
}

struct Inner<T: Connect + Default>(HashMap<String, ServiceClients<T>>);

impl<T: Connect + Default> Default for Inner<T> {
    fn default() -> Self {
        Self(Default::default())
    }
}

impl<T: Connect + Default> Inner<T> {
    fn new() -> Self {
        Self::default()
    }

    fn add_fallback(&mut self, namespace: &str, addr: &str) {
        let timeout = crate::config().service_grpc_timeout_ms();
        let client = T::connect(addr, timeout);
        let fallback_client = Arc::new(client);

        if let Some(ref mut ns_clients) = self.0.get_mut(namespace) {
            ns_clients.set_fallback(addr, fallback_client);
        } else {
            let mut ns_clients = ServiceClients::default();
            ns_clients.set_fallback(addr, fallback_client);
            self.0.insert(namespace.to_string(), ns_clients);
        }

        log::info!(
            "add fallback to business Clients: ns:{}, addr:{}",
            namespace,
            addr
        );
    }

    fn add_client(&mut self, namespace: &str, addr: &str, zone: Option<String>) {
        let timeout = crate::config().service_grpc_timeout_ms();
        let client = T::connect(addr, timeout);
        let client = Arc::new(ServiceClient {
            client: Arc::new(client),
            zone,
        });
        let ns_clients = self
            .0
            .entry(namespace.to_string())
            .or_insert_with(Default::default);
        ns_clients.clients.insert(addr.to_string(), client);

        log::info!("add client to Clients: ns:{}, addr:{}", namespace, addr);
    }

    fn remove_client(&mut self, namespace: &str, addr: &str) {
        if let Some(ns_clients) = self.0.get_mut(namespace) {
            ns_clients.clients.remove(addr);
            log::info!(
                "remove client from Clients: ns:{}, addr:{}",
                namespace,
                addr
            );
        } else {
            log::warn!(
                "trying to remove non-exist client from Clients, ns:{} addr:{}",
                namespace,
                addr
            );
        }
    }

    fn pick_client(
        &self,
        namespace: &str,
        zone: Option<String>,
    ) -> Option<(String, Arc<<T as Connect>::Item>)> {
        if let Some(ns_clients) = self.0.get(namespace) {
            ns_clients.pick_client(zone)
        } else {
            log::warn!("trying to pick non-exist client from ns:{} ", namespace,);
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct MockClient {}

    impl Connect for MockClient {
        type Item = u8;
        fn connect<S: Into<String>>(_addr: S, _timeout_ms: u64) -> Self::Item {
            0
        }
    }

    fn init() -> Service<MockClient> {
        let client_state = Service::<MockClient>::default();
        client_state.add_client("ns", "addr0", Some("zone1".to_owned()));
        client_state.add_client("ns", "addr1", Some("zone1".to_owned()));
        client_state.add_client("ns", "addr2", Some("zone2".to_owned()));
        client_state.add_client("ns", "addr3", None);
        client_state.add_fallback("ns", "addr4");
        client_state
    }

    #[test]
    fn test_pick_multiple_from_same_zone() {
        let client_state = init();
        for _ in 0..100 {
            match client_state.pick_client("ns", Some("zone1".to_owned())) {
                Some((addr, _c)) => match addr.as_str() {
                    "addr0" => assert!(true),
                    "addr1" => assert!(true),
                    "addr2" => assert!(false),
                    "addr3" => assert!(false),
                    "addr4" => assert!(false),
                    _ => assert!(false),
                },
                None => assert!(false),
            }
        }
    }

    #[test]
    fn test_pick_one_from_zone2() {
        let client_state = init();
        for _ in 0..100 {
            match client_state.pick_client("ns", Some("zone2".to_owned())) {
                Some((addr, _c)) => match addr.as_str() {
                    "addr2" => assert!(true),
                    _ => assert!(false),
                },
                None => assert!(false),
            }
        }
    }

    #[test]
    fn test_pick_unknown_zone() {
        let client_state = init();
        for _ in 0..100 {
            match client_state.pick_client("ns", Some("unknown".to_owned())) {
                Some((addr, _c)) => match addr.as_str() {
                    "addr0" => assert!(true),
                    "addr1" => assert!(true),
                    "addr2" => assert!(true),
                    "addr3" => assert!(true),
                    "addr4" => assert!(false), // should not fallback
                    _ => assert!(false),
                },
                None => assert!(false),
            }
        }
    }

    #[test]
    fn test_pick_none_zone() {
        let client_state = init();
        for _ in 0..100 {
            match client_state.pick_client("ns", None) {
                Some((addr, _c)) => match addr.as_str() {
                    "addr0" => assert!(true),
                    "addr1" => assert!(true),
                    "addr2" => assert!(true),
                    "addr3" => assert!(true),
                    "addr4" => assert!(false), // should not fallback
                    _ => assert!(false),
                },
                None => assert!(false),
            }
        }
    }

    #[test]
    fn test_pick_empty_str_zone() {
        let client_state = init();
        for _ in 0..100 {
            match client_state.pick_client("ns", Some("".to_owned())) {
                Some((addr, _c)) => match addr.as_str() {
                    "addr0" => assert!(true),
                    "addr1" => assert!(true),
                    "addr2" => assert!(true),
                    "addr3" => assert!(true),
                    "addr4" => assert!(false), // should not fallback
                    _ => assert!(false),
                },
                None => assert!(false),
            }
        }
    }

    #[test]
    fn test_fallback() {
        let client_state = Service::<MockClient>::default();
        client_state.add_fallback("ns", "addr4");
        for _ in 0..100 {
            match client_state.pick_client("ns", Some("".to_owned())) {
                Some((addr, _c)) => match addr.as_str() {
                    "addr4" => assert!(true), // should fallback
                    _ => assert!(false),
                },
                None => assert!(false),
            }
        }
    }
}
