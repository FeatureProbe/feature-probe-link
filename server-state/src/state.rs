use parking_lot::RwLock;
use server_base::{FPConfig, HandyRwLock, RegistryNode, ServerNode, ServiceNode};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

pub trait NodeOperation: Send + Sync + Clone + 'static {
    fn server_node(&self) -> &ServerNode;

    fn registry_node(&self) -> &RegistryNode;

    fn is_myself(&self, node_id: &str) -> bool;

    fn find_node_by_host(&self, hostname: &str) -> Option<ServerNode>;

    fn find_node_by_id(&self, node_id: &str) -> Option<ServerNode>;
}

#[derive(Clone, Debug)]
pub struct State {
    inner: Arc<Inner>,
}

impl State {
    pub(crate) fn new(config: FPConfig) -> Self {
        let node_id = rand::random::<u64>().to_string();
        let hostname = config.hostname();
        let zone = config.zone();
        let server_node = ServerNode::new(node_id, hostname.clone(), 6321, 1215, zone);
        let registry_node = RegistryNode::new(
            hostname + ":8082", // FIXME: hard code 8082 read from conf
            config.cert_path().is_some(),
        );
        let inner = Inner::new(server_node, registry_node);

        Self {
            inner: Arc::new(inner),
        }
    }

    pub fn show_init_info(&self) {
        let server_node = self.server_node();
        let registry_node = self.registry_node();
        log::info!("_fplink_state:||server_node= {:?}", server_node);
        log::info!("_fplink_state:||registry_node= {:?}", registry_node);
    }

    pub fn update_cluster_nodes(&mut self, nodes: HashSet<ServerNode>) {
        let mut lock = self.inner.cluster_nodes.wl();
        *lock = nodes;
    }

    pub fn update_service_nodes(&mut self, namespace: &str, nodes: HashSet<ServiceNode>) {
        let mut lock = self.inner.service_nodes.wl();
        lock.insert(namespace.into(), nodes);
    }

    pub fn cluster_nodes(&self) -> HashSet<ServerNode> {
        self.inner.cluster_nodes.rl().clone()
    }

    pub fn service_nodes(&self) -> HashMap<String, HashSet<ServiceNode>> {
        self.inner.service_nodes.rl().clone()
    }
}

impl NodeOperation for State {
    fn server_node(&self) -> &ServerNode {
        &self.inner.server_node
    }

    fn registry_node(&self) -> &RegistryNode {
        &self.inner.registry_node
    }

    fn is_myself(&self, node_id: &str) -> bool {
        self.server_node().node_id == node_id
    }

    fn find_node_by_host(&self, hostname: &str) -> Option<ServerNode> {
        let server_node = self.server_node();
        if server_node.hostname == hostname {
            return Some(server_node.clone());
        }

        self.inner
            .cluster_nodes
            .rl()
            .iter()
            .find(|node| node.hostname == hostname)
            .cloned()
    }

    fn find_node_by_id(&self, node_id: &str) -> Option<ServerNode> {
        let server_node = self.server_node();
        if server_node.node_id == node_id {
            return Some(server_node.clone());
        }

        self.inner
            .cluster_nodes
            .rl()
            .iter()
            .find(|node| node.node_id == node_id)
            .cloned()
    }
}

#[derive(Debug)]
struct Inner {
    server_node: ServerNode,
    registry_node: RegistryNode,
    cluster_nodes: RwLock<HashSet<ServerNode>>,
    service_nodes: RwLock<HashMap<String, HashSet<ServiceNode>>>,
}

impl Inner {
    pub fn new(server_node: ServerNode, registry_node: RegistryNode) -> Self {
        Self {
            server_node,
            registry_node,
            cluster_nodes: Default::default(),
            service_nodes: Default::default(),
        }
    }
}
