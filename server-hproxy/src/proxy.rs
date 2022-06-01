use crate::{Error, HpRequestBuilder, CALL_ID, HTTP_STATUS_CODE};
use async_trait::async_trait;
use cached::proc_macro::cached;
use regex::Regex;
use reqwest::header::HeaderMap;
use reqwest::Client as HttpClient;
use server_base::proto::{Message, MessageReq, PushConnReq};
use server_base::{BuiltinService, Hproxy, PushConn};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

const TCP_KEEPALIVE_SEC: u64 = 90;
const FORWARDED_FOR: &str = "X-Forwarded-For";
const REAL_IP: &str = "X-Real-IP";

pub struct HttpProxy<G: PushConn + Clone> {
    inner: Arc<Inner<G>>,
}

enum ResponseResult {
    Response(Box<reqwest::Response>),
    HttpErr(reqwest::Error),
    ProxyErr(String),
}

impl From<Result<reqwest::Response, reqwest::Error>> for ResponseResult {
    fn from(result: Result<reqwest::Response, reqwest::Error>) -> ResponseResult {
        match result {
            Ok(response) => ResponseResult::Response(Box::new(response)),
            Err(e) => ResponseResult::HttpErr(e),
        }
    }
}

#[derive(Clone, Debug)]
struct HproxyRule {
    regex: Regex,
    replace: String,
    timeout_ms: u64,
}

type HproxyRules = Vec<HproxyRule>;
type HproxyMap = HashMap<String, Hproxy>;

struct Inner<G: PushConn + Clone> {
    pusher: G,
    http_client: reqwest::Client,
    hproxy_rules: HproxyRules,
}

impl<G: PushConn + Clone> Inner<G> {
    fn new(pusher: G, hproxy_rules: HproxyRules) -> Self {
        Self {
            pusher,
            http_client: HttpClient::builder()
                .tcp_keepalive(Some(Duration::from_secs(TCP_KEEPALIVE_SEC)))
                .build()
                .expect("create http client failed"),
            hproxy_rules,
        }
    }
}

#[async_trait]
impl<G: PushConn + Clone + Send + Sync> BuiltinService for HttpProxy<G> {
    async fn on_message(&self, conn_id: &str, peer_addr: Option<SocketAddr>, mut message: Message) {
        let cid = conn_id.to_string();
        add_forwarded(&mut message.metadata, peer_addr);
        add_real_ip(&mut message.metadata, peer_addr);
        self.handle_request(MessageReq {
            cid,
            message: Some(message),
            ..Default::default()
        })
        .await;
    }
}

impl<G: PushConn + Clone> HttpProxy<G> {
    pub fn new(pusher: G) -> Self {
        Self {
            inner: Arc::new(Inner::new(pusher, Vec::new())),
        }
    }

    pub fn with_hproxy_map(pusher: G, hproxy_map: HproxyMap) -> Self {
        let hproxy_rules = hproxy_map
            .into_iter()
            .filter_map(|(origin_url, hproxy)| match Regex::new(&origin_url) {
                Ok(regex) => Some(HproxyRule {
                    regex,
                    replace: hproxy.rewrite_url,
                    timeout_ms: hproxy.timeout,
                }),
                Err(e) => {
                    log::error!("hproxy rule config invalid: {}, err: {:?}", origin_url, e);
                    None
                }
            })
            .collect();
        Self {
            inner: Arc::new(Inner::new(pusher, hproxy_rules)),
        }
    }

    pub async fn handle_request(&self, origin_request: MessageReq) {
        log::debug!("recved: {:?}", origin_request);
        let request: Result<HpRequestBuilder, Error> = origin_request.try_into();
        if let Ok(request) = request {
            self.process(request).await;
        }
    }

    pub async fn process(&self, request: HpRequestBuilder) -> Option<u128> {
        let origin_url = request.url.as_str();
        let rewrite_url;
        let timeout;
        let pusher = self.inner.pusher.clone();
        let call_id = request.call_id.as_ref();
        let conn_id = &request.conn_id;

        if let Some((r, t)) = self.replace_url(origin_url) {
            rewrite_url = r;
            timeout = t;
        } else {
            let msg = format!("hproxy invalid url: {}", origin_url);
            log::error!("{}", msg);
            let request = grpc_push_request(ResponseResult::ProxyErr(msg), conn_id, call_id).await;
            pusher.push(request).await;
            return None;
        }
        let client = self.inner.http_client.clone();
        match request.build(rewrite_url.as_str(), client) {
            Ok(request) => {
                let start = Instant::now();
                let resp = request.timeout(timeout).send().await;
                let latency = start.elapsed().as_millis();
                let request = grpc_push_request(resp.into(), conn_id, call_id).await;
                pusher.push(request).await;
                Some(latency)
            }
            Err(e) => {
                // TODO: push error back
                log::error!("request build err: {:?}", e);
                None
            }
        };
        None
    }

    fn replace_url(&self, origin_url: &str) -> Option<(String, Duration)> {
        let mut hproxy_rules = hproxy_rules_from_config();
        if !self.inner.hproxy_rules.is_empty() {
            hproxy_rules.extend(self.inner.hproxy_rules.clone());
        }
        for hproxy_rule in hproxy_rules {
            if hproxy_rule.regex.is_match(origin_url) {
                let rewrite_url = hproxy_rule
                    .regex
                    .replace(origin_url, hproxy_rule.replace)
                    .to_string();
                return Some((rewrite_url, Duration::from_millis(hproxy_rule.timeout_ms)));
            }
        }
        None
    }
}

#[cached(time = 300)]
fn hproxy_rules_from_config() -> HproxyRules {
    server_state::config()
        .hproxy_map()
        .into_iter()
        .filter_map(|(origin_url, hproxy)| match Regex::new(&origin_url) {
            Ok(regex) => Some(HproxyRule {
                regex,
                replace: hproxy.rewrite_url,
                timeout_ms: hproxy.timeout,
            }),
            Err(e) => {
                log::error!("hproxy rule config invalid: {}, err: {:?}", origin_url, e);
                None
            }
        })
        .collect()
}

trait ResponseWithHeader {
    fn headers_mut(&mut self) -> &mut HeaderMap;
}

impl ResponseWithHeader for reqwest::Response {
    fn headers_mut(&mut self) -> &mut HeaderMap {
        self.headers_mut()
    }
}

async fn grpc_push_request(
    resp: ResponseResult,
    conn_id: &str,
    call_id: Option<&String>,
) -> PushConnReq {
    let mut metadata: HashMap<String, String> = HashMap::new();
    let mut message = Message::default();
    if let Some(call_id) = call_id {
        metadata.insert(CALL_ID.to_owned(), call_id.to_owned());
    }

    match resp {
        ResponseResult::Response(resp) => {
            metadata.insert(
                HTTP_STATUS_CODE.to_owned(),
                resp.status().as_str().to_string(),
            );
            message.path = "".to_owned();
            let headers = resp.headers();
            for (k, v) in headers.iter() {
                match v.to_str() {
                    Ok(val) => {
                        metadata.insert(k.to_string(), val.to_string());
                    }
                    _ => log::warn!("header_value_error {:?}", v),
                }
            }
            match resp.bytes().await {
                Ok(body) => message.body = body.to_vec(),
                Err(e) => log::warn!("body_not_text e: {:?}", e),
            }
        }
        ResponseResult::HttpErr(e) => {
            // BAD GATEWAY
            metadata.insert(HTTP_STATUS_CODE.to_owned(), "502".to_owned());
            let msg = format!("hproxy http error: {}", e);
            message.body = msg.as_bytes().to_vec();
            log::warn!("resp_error {:?}", e);
        }
        ResponseResult::ProxyErr(e) => {
            // BAD GATEWAY
            metadata.insert(HTTP_STATUS_CODE.to_owned(), "502".to_owned());
            let msg = format!("hproxy proxy error: {}", e);
            message.body = msg.as_bytes().to_vec();
            log::warn!("resp_error {:?}", e);
        }
    }
    message.metadata = metadata;

    PushConnReq {
        cid: conn_id.to_owned(),
        message: Some(message),
        trace: None,
    }
}

fn add_forwarded(metadata: &mut HashMap<String, String>, peer_addr: Option<SocketAddr>) {
    let peer_ip = match peer_addr {
        Some(addr) => addr.ip().to_string(),
        None => return,
    };
    metadata.insert(FORWARDED_FOR.to_owned(), peer_ip);
}

fn add_real_ip(metadata: &mut HashMap<String, String>, peer_addr: Option<SocketAddr>) {
    let peer_ip = match peer_addr {
        Some(addr) => addr.ip().to_string(),
        None => return,
    };
    metadata.insert(REAL_IP.to_owned(), peer_ip);
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{add_forwarded, FORWARDED_FOR};
    use server_base::tokio;
    use server_base::FPConfig;
    use std::collections::HashMap;

    #[derive(Clone, Default)]
    struct MockGrpcClient {}

    #[async_trait]
    impl PushConn for MockGrpcClient {
        async fn push(&self, resp: PushConnReq) {
            assert!(resp.message.is_some());
            let message = resp.message.unwrap();
            let metadata = message.metadata;
            assert_eq!(metadata.get(HTTP_STATUS_CODE), Some(&"200".to_owned()));
        }
    }

    #[test]
    fn test_url_replace() {
        let router_url = "http://inner.backend.com/inner_path/teams/some_path/";
        let regex_url = "https{0,1}://some.backend.com/api/";
        let origin_base = "https://some.backend.com/api/";
        let origin_url = format!("{}{}", origin_base, "subpath?a=b");
        let target_url = format!("{}{}", router_url, "subpath?a=b");
        let regex_url1 = "https{0,1}://some.backend.com/ext/";
        let origin_base1 = "https://some.backend.com/ext/";
        let origin_url1 = format!("{}{}", origin_base1, "subpath?a=b");
        let target_url1 = format!("{}{}", router_url, "subpath?a=b");
        let mut hproxy_map = HashMap::new();
        hproxy_map.insert(
            regex_url.to_string(),
            Hproxy {
                origin_url: regex_url.to_string(),
                rewrite_url: router_url.to_string(),
                timeout: 2000_u64,
            },
        );
        let mut config = config::Config::new();
        config
            .set(
                "hproxy_1",
                format!("{}#{}#{}", regex_url1, router_url, "2001"),
            )
            .unwrap();
        server_state::set_config(FPConfig::new_with_config(config));
        let pusher = MockGrpcClient::default();
        let c = HttpProxy::with_hproxy_map(pusher, hproxy_map);
        assert_eq!(
            c.replace_url(origin_url.as_str()),
            Some((target_url, Duration::from_millis(2000)))
        );
        assert_eq!(
            c.replace_url(origin_url1.as_str()),
            Some((target_url1, Duration::from_millis(2001)))
        );
    }

    #[tokio::test]
    async fn test_request() {
        let router_url = "http://www.bing.com/";
        let regex_url = "https{0,1}://www.baidu.com/";
        let origin = "https://www.baidu.com/search?q=123";
        let metadata: HashMap<String, String> = HashMap::new();
        let builder = HpRequestBuilder {
            conn_id: "123".to_owned(),
            metadata,
            body: "".to_owned(),
            call_id: Some("123".to_owned()),
            method: Some("GET".to_owned()),
            url: origin.to_owned(),
        };
        let mut hproxy_map = HashMap::new();
        hproxy_map.insert(
            regex_url.to_string(),
            Hproxy {
                origin_url: regex_url.to_string(),
                rewrite_url: router_url.to_string(),
                timeout: 20000_u64,
            },
        );
        let grpc = MockGrpcClient::default();
        let c = HttpProxy::with_hproxy_map(grpc, hproxy_map);
        c.process(builder).await;
    }

    struct MockResponse {
        pub headers: HeaderMap,
    }

    impl ResponseWithHeader for MockResponse {
        fn headers_mut(&mut self) -> &mut HeaderMap {
            &mut self.headers
        }
    }

    #[test]
    fn test_add_forwarded() {
        let mut metadata: HashMap<String, String> = HashMap::new();
        metadata.insert(FORWARDED_FOR.to_string(), "1.1.1.1".to_string());
        add_forwarded(&mut metadata, "2.2.2.2:8080".parse().ok());
        let expect = "2.2.2.2".to_string();
        assert_eq!(&expect, metadata.get(FORWARDED_FOR).unwrap());

        let mut metadata: HashMap<String, String> = HashMap::new();
        add_forwarded(&mut metadata, "2.2.2.2:8080".parse().ok());
        let expect = "2.2.2.2".to_string();
        assert_eq!(&expect, metadata.get(FORWARDED_FOR).unwrap());
    }

    #[test]
    fn test_add_real_ip() {
        let mut metadata: HashMap<String, String> = HashMap::new();
        metadata.insert(REAL_IP.to_string(), "1.1.1.1".to_string());
        add_real_ip(&mut metadata, "2.2.2.2:8080".parse().ok());
        let expect = "2.2.2.2".to_string();
        assert_eq!(&expect, metadata.get(REAL_IP).unwrap());

        let mut metadata: HashMap<String, String> = HashMap::new();
        add_real_ip(&mut metadata, "2.2.2.2:8080".parse().ok());
        let expect = "2.2.2.2".to_string();
        assert_eq!(&expect, metadata.get(REAL_IP).unwrap());
    }
}
