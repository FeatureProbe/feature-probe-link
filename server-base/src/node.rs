use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

#[derive(Clone, Eq, Debug, Default, Serialize, Deserialize)]
pub struct ServerNode {
    pub node_id: String,
    pub hostname: String,
    pub port: u16,            // cluster grpc port
    pub outer_grpc_port: u16, // service grpc port
    pub created_ts: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RegistryNode {
    pub listen_addr: String,
    pub listen_tcp_addr: String, // deprecated
    pub listen_ws_addr: String,  // deprecated
    pub use_ssl: bool,
}

impl RegistryNode {
    pub fn new<T: Into<String>>(addr: T, ws_addr: T, use_ssl: bool) -> Self {
        let addr = addr.into();
        Self {
            listen_tcp_addr: addr.clone(),
            listen_addr: addr,
            listen_ws_addr: ws_addr.into(),
            use_ssl,
        }
    }
}

#[derive(Clone, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceNode {
    pub instance_id: String,
    pub service_id: String,
    pub host: String,
    pub port: u16,
    pub created_ts: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
}

impl ServiceNode {
    pub fn from_halo<T: Into<String>>(service_id: T, node: ServerNode) -> Self {
        Self::new(
            service_id.into(),
            node.node_id,
            node.zone,
            node.hostname,
            node.outer_grpc_port,
            Some(node.created_ts),
        )
    }

    pub fn new(
        service_id: String,
        instance_id: String,
        zone: Option<String>,
        host: String,
        port: u16,
        created_ts: Option<u64>,
    ) -> Self {
        Self {
            service_id,
            instance_id,
            zone,
            host,
            port,
            created_ts,
        }
    }
}

impl PartialEq for ServerNode {
    fn eq(&self, other: &ServerNode) -> bool {
        self.node_id == other.node_id
    }
}

impl Hash for ServerNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.node_id.hash(state);
    }
}

impl ServerNode {
    pub fn new<T: Into<String>>(
        node_id: T,
        halo_listen_host: T,
        halo_listen_port: u16,
        backend_listen_port: u16,
        zone: Option<String>,
    ) -> Self {
        Self {
            node_id: node_id.into(),
            hostname: halo_listen_host.into(),
            port: halo_listen_port,
            outer_grpc_port: backend_listen_port,
            created_ts: crate::now_ts_milli(),
            zone,
        }
    }
}

impl PartialEq for ServiceNode {
    fn eq(&self, other: &ServiceNode) -> bool {
        self.instance_id == other.instance_id
    }
}

impl Hash for ServiceNode {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.instance_id.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json;

    #[test]
    fn test_deserlize() {
        let string = "{\"serviceId\":\"voip-signal\",\"host\":\"10.96.102.109\",\"port\":8080,\"secure\":false,\"uri\":null,\"scheme\":\"http\",\"instanceId\":\"10.96.102.109\",\"haloServicePort\":\"2333\", \"unknown_field\": \"\"}";
        let node: ServiceNode = serde_json::from_str(string).unwrap();
        assert_eq!(node.service_id, "voip-signal".to_owned());
        assert_eq!(node.zone, None);
    }
}
