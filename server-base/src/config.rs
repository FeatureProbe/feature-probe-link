use config::File;
pub use config::{Config, ConfigError, Environment, Source, Value};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub struct Service {
    pub etcd_name: String,
    pub fallback_addr: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Hproxy {
    pub origin_url: String,
    pub rewrite_url: String,
    pub timeout: u64,
}

#[derive(Clone)]
pub struct FPConfig {
    config: Arc<Config>,
}

impl FPConfig {
    pub fn new() -> Self {
        FPConfig::default()
    }

    pub fn new_with_config(config: Config) -> Self {
        Self {
            config: Arc::new(config),
        }
    }

    pub fn config_content(&self) -> String {
        let mut config_context = String::new();
        let kvs: HashMap<String, Value> = self.config.collect().unwrap_or_else(|_| HashMap::new());
        for (k, v) in kvs {
            config_context.push_str(&format!("_fplink_conf||config={} => {}\r\n", k, v));
        }
        config_context
    }

    pub fn show_config(&self) {
        let kvs: HashMap<String, Value> = self.config.collect().unwrap_or_else(|_| HashMap::new());
        for (k, v) in kvs {
            log::info!("_fplink_conf||config={} => {}", k, v);
        }
    }

    pub fn cert_path(&self) -> Option<String> {
        match self.config.get_str("cert_path") {
            Ok(path) if !path.trim().is_empty() => Some(path),
            _ => None,
        }
    }

    pub fn cert_path_quic(&self) -> Option<String> {
        match self.config.get_str("cert_path_quic") {
            Ok(path) if !path.trim().is_empty() => Some(path),
            _ => self.cert_path(),
        }
    }

    pub fn etcd_addr(&self) -> String {
        self.config
            .get_str("etcd")
            .unwrap_or_else(|_| "localhost:2379".to_owned())
    }

    pub fn service_id(&self) -> String {
        self.config
            .get_str("service_id")
            .unwrap_or_else(|_| "fplink--grpc".to_owned())
    }

    pub fn etcd_prefix_base(&self) -> String {
        let mut prefix = self
            .config
            .get_str("etcd_prefix")
            .unwrap_or_else(|_| "/".to_owned());
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        prefix
    }

    pub fn etcd_prefix_broadcast(&self) -> String {
        let mut prefix = self
            .config
            .get_str("etcd_prefix_broadcast")
            .unwrap_or_else(|_| self.etcd_prefix_base());
        if !prefix.ends_with('/') {
            prefix.push('/');
        }
        prefix
    }

    pub fn conn_listen_addr(&self) -> String {
        self.config
            .get_str("listen_addr")
            .unwrap_or_else(|_| "0.0.0.0:8082".to_owned())
    }

    pub fn conn_listen_addr_deprecated(&self) -> String {
        self.config
            .get_str("listen_addr2")
            .unwrap_or_else(|_| "0.0.0.0:8083".to_owned())
    }

    pub fn peer_listen_addr(&self) -> String {
        self.config
            .get_str("peer_listen_addr")
            .unwrap_or_else(|_| "0.0.0.0:6321".to_owned())
    }

    pub fn service_listen_addr(&self) -> String {
        self.config
            .get_str("listen_service_addr")
            .unwrap_or_else(|_| "0.0.0.0:1215".to_owned())
    }

    pub fn hostname(&self) -> String {
        self.config
            .get_str("hostname")
            .unwrap_or_else(|_| "127.0.0.1".to_owned())
    }

    pub fn zone(&self) -> Option<String> {
        if let Ok(zone) = self.config.get_str("zone") {
            return Some(zone);
        }
        None
    }

    pub fn hproxy_map(&self) -> HashMap<String, Hproxy> {
        let mut hproxy_map: HashMap<String, Hproxy> = HashMap::new();
        let mut index = 1;
        while let Ok(hproxy_str) = self.config.get_str(&format!("hproxy_{}", index)) {
            index += 1;
            let tokens: Vec<&str> = hproxy_str.trim().split('#').collect();
            let origin_url: String;
            let hproxy: Hproxy;
            if let Some(&o_url) = tokens.get(0) {
                origin_url = o_url.trim().to_string();
            } else {
                log::warn!("invalid hproxy {}", hproxy_str);
                continue;
            }
            if origin_url.is_empty() {
                log::warn!("invalid hproxy {}", hproxy_str);
                continue;
            }
            if let Some(&rewrite_url) = tokens.get(1) {
                let rewrite_url = rewrite_url.trim().to_string();
                if rewrite_url.is_empty() {
                    log::warn!("invalid hproxy {}", hproxy_str);
                    continue;
                }
                let mut timeout = self.hproxy_timeout();
                if let Some(&fallback) = tokens.get(2) {
                    if let Ok(t) = fallback.trim().parse::<u64>() {
                        timeout = t;
                    } else {
                        log::warn!("parse timeout failed {}", hproxy_str);
                    }
                } else {
                    log::warn!("hproxy {} not set timeout", hproxy_str);
                }
                hproxy = Hproxy {
                    origin_url: origin_url.clone(),
                    rewrite_url,
                    timeout,
                };
                hproxy_map.insert(origin_url, hproxy);
            } else {
                log::warn!("invalid hproxy {}", hproxy_str);
                continue;
            }
        }
        hproxy_map
    }

    pub fn service_map(&self) -> HashMap<String, Service> {
        let mut ns_service_map = HashMap::new();
        let mut index = 1;
        while let Ok(service_str) = self.config.get_str(&format!("service_{}", index)) {
            index += 1;
            let tokens: Vec<&str> = service_str.trim().split('#').collect();
            let namespace: String;
            let service: Service;
            if let Some(&ns) = tokens.get(0) {
                namespace = ns.trim().to_string();
            } else {
                log::warn!("invalid service {}", service_str);
                continue;
            }
            if namespace.is_empty() {
                log::warn!("invalid service {}", service_str);
                continue;
            }
            if let Some(&etcd_name) = tokens.get(1) {
                let etcd_name = etcd_name.trim().to_string();
                if etcd_name.is_empty() {
                    log::warn!("invalid service {}", service_str);
                    continue;
                }
                let mut fallback_addr = None;
                if let Some(&fallback) = tokens.get(2) {
                    fallback_addr = Some(fallback.trim().to_string());
                } else {
                    log::warn!("service {} not set fallback addr", service_str);
                }
                service = Service {
                    etcd_name,
                    fallback_addr,
                };
                ns_service_map.insert(namespace, service);
            } else {
                log::warn!("invalid service {}", service_str);
                continue;
            }
        }
        ns_service_map
    }

    pub fn cluster_grpc_timeout_ms(&self) -> u64 {
        self.config
            .get_int("grpc_cluster_timeout_ms")
            .unwrap_or(500) as u64
    }

    pub fn service_grpc_timeout_ms(&self) -> u64 {
        if let Ok(timeout) = self.config.get_int("service_grpc_timeout_ms") {
            timeout as u64
        } else {
            self.config.get_int("biz_grpc_timeout_ms").unwrap_or(1000) as u64
        }
    }

    pub fn hproxy_timeout(&self) -> u64 {
        if let Ok(timeout) = self.config.get::<u64>("hproxy_http_timeout_ms") {
            timeout
        } else {
            10_000
        }
    }

    pub fn quic_trace_log(&self) -> bool {
        self.config.get_bool("quic_trace_log").unwrap_or(false)
    }

    pub fn worker_num(&self) -> usize {
        let worker_num = match self.get_int("runtime_worker_num") {
            Ok(num) => num as usize,
            Err(_) => num_cpus::get(),
        };
        log::info!("tokio runtime_worker_num is: {}", worker_num);
        worker_num
    }

    // default 8k
    pub fn tokio_codec_size(&self) -> usize {
        self.get_int("tokio_codec_size")
            .map(|size| size as usize)
            .unwrap_or(8 * 1024)
    }

    pub fn log_directory(&self) -> String {
        self.get_str("log_directory")
            .unwrap_or_else(|_| "/tmp/fplink".to_owned())
    }
}

impl std::ops::Deref for FPConfig {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

impl Default for FPConfig {
    fn default() -> Self {
        let mut config = Config::default();
        let config_path = "./resources/fplink_config.toml";
        if std::path::Path::new(config_path).exists() {
            let file = File::with_name(config_path).required(false);
            log::info!("config load from file {:?}", file);
            let _ = config.merge(file).expect("merge config file error!");
        } else {
            log::info!("{} not found!", config_path);
        }

        let _ = config
            .merge(Environment::with_prefix("fplink"))
            .expect("merge env config error!");

        Self {
            config: Arc::new(config),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_service_map() {
        let mut config = Config::new();
        let _ = config.set("service_1", " ns1#name1#fallback1 ");
        let _ = config.set("service_2", " ns2#name2#fallback2 ");
        let _ = config.set("service_3", " ns3#name3 ");
        let _ = config.set("service_4", " ## ");
        let _ = config.set("service_5", " #name5# ");
        let _ = config.set("service_6", " #### ");
        let _ = config.set("service_7", "");
        let _ = config.set("service_8", " ns8##");

        let config = FPConfig::new_with_config(config);
        let service_map = config.service_map();

        assert_eq!(service_map.len(), 3);
        assert_eq!(service_map.get("ns1").unwrap().etcd_name, "name1");
        assert_eq!(
            service_map.get("ns1").unwrap().fallback_addr,
            Some("fallback1".to_string())
        );

        assert_eq!(
            service_map.get("ns3").unwrap().fallback_addr.is_none(),
            true
        );
    }

    #[test]
    fn test_hproxy_map() {
        let mut config = Config::new();
        let _ = config.set("hproxy_1", "http://www.baidu.com#http://10.172.19.11#2000");
        let _ = config.set("hproxy_2", " http://www.souhu.com# http://10.172.20.11 # ");
        let _ = config.set("hproxy_3", "#http://10.172.19.18#1000");
        let _ = config.set("hproxy_4", "http://www.bing.com##1000");
        let _ = config.set("hproxy_5", " ##");
        let _ = config.set("hproxy_6", " ##1000");
        let _ = config.set("hproxy_7", " http://www.google.com# http://10.172.20.12 ");

        let config = FPConfig::new_with_config(config);
        let hproxy_map: HashMap<String, Hproxy> = config.hproxy_map();

        assert_eq!(hproxy_map.len(), 3);
        assert_eq!(
            "http://10.172.19.11",
            hproxy_map.get("http://www.baidu.com").unwrap().rewrite_url
        );
        assert_eq!(
            2000 as u64,
            hproxy_map.get("http://www.baidu.com").unwrap().timeout
        );

        assert_eq!(
            "http://10.172.20.11",
            hproxy_map.get("http://www.souhu.com").unwrap().rewrite_url
        );
        assert_eq!(
            10000 as u64,
            hproxy_map.get("http://www.souhu.com").unwrap().timeout
        );

        assert_eq!(
            "http://10.172.20.12",
            hproxy_map.get("http://www.google.com").unwrap().rewrite_url
        );
        assert_eq!(
            10000 as u64,
            hproxy_map.get("http://www.google.com").unwrap().timeout
        );
    }
}
