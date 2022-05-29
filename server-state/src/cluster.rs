use crate::{Connect, GrpcClient};
use parking_lot::RwLock;
use server_base::HandyRwLock;
use std::collections::HashMap;
use std::sync::Arc;

pub type Clusters = Cluster<GrpcClient>;

#[derive(Clone)]
pub struct Cluster<T: Connect + Default> {
    //map for node_id to grpc_client
    clients: Arc<RwLock<HashMap<String, T::Item>>>,
    // map for addr to node_id
    addr_node_map: Arc<RwLock<HashMap<String, String>>>,
}

impl<T: Connect> Default for Cluster<T> {
    fn default() -> Self {
        Cluster {
            clients: Arc::new(RwLock::new(HashMap::new())),
            addr_node_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl<T: Connect> Cluster<T> {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub fn get_node_id(&self, addr: &str) -> Option<String> {
        self.addr_node_map.rl().get(addr).cloned()
    }

    fn add_node_id(&mut self, addr: &str, node_id: &str) -> Option<String> {
        self.addr_node_map
            .wl()
            .insert(addr.to_owned(), node_id.to_owned())
    }

    fn remove_node_id(&mut self, addr: &str, node_id: &str) {
        if let Some(old_node_id) = self.get_node_id(addr) {
            if node_id == old_node_id {
                self.addr_node_map.wl().remove(addr);
            }
        }
    }

    pub fn add_client(&mut self, node_id: &str, host: &str, port: u16) {
        let timeout_ms = crate::config().cluster_grpc_timeout_ms();
        let addr = format!("{}:{}", host, port);
        if let Some(old_node_id) = self.add_node_id(&addr, node_id) {
            self.clients.wl().remove(old_node_id.as_str());
        }
        let c = T::connect(addr, timeout_ms);
        self.clients.wl().insert(node_id.to_string(), c);
    }

    pub fn remove_client(&mut self, node_id: &str, host: &str, port: u16) {
        let addr = format!("{}:{}", host, port);
        log::warn!("remove client for node: {}, {}", node_id, addr);
        self.clients.wl().remove(node_id);
        self.remove_node_id(&addr, node_id);
    }

    pub fn pick_client(&self, node_id: &str) -> Option<T::Item> {
        self.clients.rl().get(node_id).cloned()
    }

    pub fn get_clients(&self) -> Arc<RwLock<HashMap<String, T::Item>>> {
        Arc::clone(&self.clients)
    }

    pub fn len(&self) -> usize {
        self.clients.rl().len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn all_clients(&self) -> Vec<String> {
        self.clients.rl().keys().cloned().collect()
    }

    pub fn all_nodes(&self) -> HashMap<String, String> {
        self.addr_node_map.rl().clone()
    }
}
