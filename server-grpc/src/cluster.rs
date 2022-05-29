use cached::proc_macro::cached;
use parking_lot::Mutex;
use server_base::proto::{
    link_service_client::LinkServiceClient, BulkPubResp, BulkSubReq, ConnChannels, GetChannelsReq,
    GetConnsReq, PubReq, PubResp, PubStatus, PushConnReq, SubReq, UnSubReq,
};
use server_base::{minstant, tokio, tokio::sync::oneshot, tonic, HandyRwLock};
use server_state::Clusters;
use std::collections::{HashSet, VecDeque};
use std::sync::Arc;
use std::time::Duration;

#[derive(Clone)]
pub struct ClusterForwarder {
    cluster: Arc<Clusters>,
    bulk_trigger: Arc<BulkTrigger>,
    bulk_requests: Arc<Mutex<Vec<BulkRequest>>>,
    grpc_bulk: fn() -> (bool, u64, u64),
}

impl Default for ClusterForwarder {
    fn default() -> Self {
        let cluster = Arc::new(server_state::cluster_client_state());
        let bulk_trigger = Arc::new(BulkTrigger::new(grpc_bulk));
        let bulk_requests = Default::default();
        Self {
            cluster,
            bulk_requests,
            bulk_trigger,
            grpc_bulk,
        }
    }
}

#[derive(Debug)]
enum BulkRequest {
    Publish(PubReq, oneshot::Sender<Vec<PubResp>>),
}

#[cached(time = 60)]
fn grpc_timeout() -> Duration {
    let timeout = server_state::config().cluster_grpc_timeout_ms();
    Duration::from_millis(timeout)
}

#[cached(time = 60)]
fn grpc_bulk() -> (bool, u64, u64) {
    let config = server_state::config();
    let enable_bulk = config.get_bool("grpc_bulk_enable").unwrap_or(false);
    let max_us = config.get_int("grpc_bulk_max_us").unwrap_or(500) as u64;
    let max_size = config.get_int("grpc_bulk_max_size").unwrap_or(200) as u64;
    (enable_bulk, max_us, max_size)
}

fn timeout<T>(t: T) -> tonic::Request<T> {
    let mut request = tonic::Request::new(t);
    request.set_timeout(grpc_timeout());
    request
}

impl ClusterForwarder {
    pub fn new() -> Self {
        ClusterForwarder::default()
    }

    #[cfg(test)]
    pub(crate) fn new_for_test(grpc_bulk: fn() -> (bool, u64, u64)) -> Self {
        ClusterForwarder {
            grpc_bulk,
            ..Default::default()
        }
    }

    pub async fn subscribe(&self, node_id: &str, request: SubReq) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .subscribe(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }
        log::error!(
            "bind_tag failed: not found client for connectionId: {}",
            &request.cid
        );
        Ok(false)
    }

    pub async fn unsubscribe(
        &self,
        node_id: &str,
        request: UnSubReq,
    ) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .unsubscribe(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }

        log::warn!(
            "unbind_tag failed, no client for nodeId: {}, connId: {}",
            node_id,
            &request.cid
        );
        Ok(false)
    }

    pub async fn bulk_subscribe(
        &self,
        node_id: &str,
        request: BulkSubReq,
    ) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .bulk_subscribe(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }
        log::warn!("bulk_subscribe failed, no client for cid: {}", &request.cid);
        Ok(false)
    }

    pub async fn push_conn(
        &self,
        node_id: &str,
        request: PushConnReq,
    ) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .push_conn(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }
        log::warn!(
            "push_conn failed: no client for connectionId: {}",
            &request.cid
        );
        Ok(false)
    }

    fn append_bulk_request(&self, bulk_request: BulkRequest) {
        log::trace!("append_bulk_request {:?}", bulk_request);
        let mut guard = self.bulk_requests.lock();
        (*guard).push(bulk_request);
    }

    fn clear_bulk_request(&self) -> Vec<BulkRequest> {
        let mut guard = self.bulk_requests.lock();
        let mut vec = vec![];
        std::mem::swap(&mut vec, &mut *guard);
        vec
    }

    pub async fn publish(&self, request: PubReq) -> PubResp {
        let (enable_bulk, _, _) = (self.grpc_bulk)();
        if enable_bulk {
            self.publish_bulk(request).await
        } else {
            self.publish_one_shot(request).await
        }
    }
    pub async fn publish_one_shot(&self, request: PubReq) -> PubResp {
        // TODO: Vec capacity from conf
        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = request.clone();
            futures.push(async move { client.publish(timeout(request)).await });
        }
        let mut channels: HashSet<String> = HashSet::new();
        let mut success = true;
        let mut status: Vec<PubStatus> = vec![];

        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    success = success && resp.success;
                    channels.extend(resp.channels);
                    status.extend(resp.status);
                }
                Err(e) => log::warn!("push_by_tags failed: {}", e),
            });
        PubResp {
            success,
            channels: channels.into_iter().collect(),
            status,
        }
    }

    pub async fn publish_bulk(&self, request: PubReq) -> PubResp {
        let (tx, rx) = oneshot::channel();
        let trigger = self.bulk_trigger.generate();
        let request_clone = request.clone();
        let bulk_request = BulkRequest::Publish(request_clone, tx);
        self.append_bulk_request(bulk_request);
        let mut should_trigger = trigger.is_now;

        if let Some(interval) = trigger.interval {
            log::trace!("publish should wait {}", interval);
            tokio::time::sleep(tokio::time::Duration::from_micros(interval)).await;
            should_trigger = true;
        }
        if should_trigger {
            log::trace!("publish_bulk should trigger");
            self.inner_publish_bulk().await;
        }

        let mut channels: HashSet<String> = HashSet::new();
        let mut success = true;
        let mut status: Vec<PubStatus> = vec![];
        match rx.await {
            Ok(rs) => {
                log::trace!("push_by_tags_bulk recv response {:?}", rs);
                rs.into_iter().for_each(|resp| {
                    success = success && resp.success;
                    channels.extend(resp.channels);
                    status.extend(resp.status);
                });
            }
            Err(e) => log::error!("push_by_tags failed: {}", e),
        };

        PubResp {
            success,
            channels: channels.into_iter().collect(),
            status,
        }
    }

    pub async fn get_conn_channels(&self, request: GetConnsReq) -> Vec<ConnChannels> {
        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = request.clone();
            futures.push(async move { client.get_conn_channels(timeout(request)).await });
        }
        let mut conn_vec = vec![];
        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    conn_vec.extend(resp.conn_channels);
                }
                Err(e) => log::warn!("get_conn_channels failed: {}", e),
            });

        conn_vec
    }

    pub async fn get_channels(&self, request: GetChannelsReq) -> Vec<String> {
        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = request.clone();
            futures.push(async move { client.get_channels(timeout(request)).await });
        }
        let mut vec = vec![];
        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    if let Some(channels) = resp.channels {
                        vec.extend(channels.channels);
                    }
                }
                Err(e) => log::warn!("get_channels failed: {}", e),
            });

        vec
    }

    async fn inner_publish_bulk(&self) {
        let requests = self.clear_bulk_request();
        log::debug!("inner_publish_bulk {} requests", requests.len());
        let mut rs = Vec::new();
        let mut txs = Vec::new();
        for r in requests {
            match r {
                BulkRequest::Publish(r, t) => {
                    rs.push(r);
                    txs.push(t);
                }
            }
        }
        let futures = self.publish_bulk_futures(rs);
        let mut client_responses: Vec<VecDeque<PubResp>> = Vec::with_capacity(txs.len());
        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    client_responses.push(VecDeque::from(resp.responses));
                }
                Err(e) => {
                    log::warn!("push_by_tags failed: {}", e);
                    client_responses.push(VecDeque::with_capacity(0));
                }
            });
        log::trace!("inner_publish_bulk each responses {:?}", client_responses);
        for tx in txs {
            let mut v = vec![];
            for client_response in &mut client_responses {
                if let Some(r) = client_response.pop_front() {
                    v.push(r);
                }
            }
            if let Err(e) = tx.send(v) {
                log::error!("inner_publish_bulk error: {:?}", e);
            }
        }
    }

    #[cfg(not(test))]
    fn publish_bulk_futures(
        &self,
        rs: Vec<PubReq>,
    ) -> Vec<impl futures::Future<Output = Result<tonic::Response<BulkPubResp>, tonic::Status>>>
    {
        use server_base::proto::BulkPubReq;

        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = BulkPubReq {
                requests: rs.clone(),
            };
            futures.push(async move { client.bulk_publish(timeout(request)).await });
        }
        futures
    }

    #[cfg(test)]
    fn publish_bulk_futures(
        &self,
        rs: Vec<PubReq>,
    ) -> Vec<impl futures::Future<Output = Result<tonic::Response<BulkPubResp>, tonic::Status>>>
    {
        let mut responses = vec![];
        for r in rs {
            let response = PubResp {
                success: true,
                status: vec![PubStatus {
                    conn_channels: None,
                    sent: true,
                }],
                channels: r.channels,
            };
            responses.push(response);
        }
        let mut futures = vec![];
        futures.push(async move { Ok(tonic::Response::new(BulkPubResp { responses })) });
        futures
    }
}

#[derive(Debug, PartialEq)]
struct Trigger {
    pub interval: Option<u64>,
    pub is_now: bool,
}

struct BulkTrigger {
    inner: Arc<Mutex<BulkTriggerInner>>,
}

struct BulkTriggerInner {
    is_init: bool,
    seq: u64,
    instant: minstant::Instant,
    max_fn: fn() -> (bool, u64, u64),
}

impl BulkTrigger {
    pub fn new(max_fn: fn() -> (bool, u64, u64)) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BulkTriggerInner {
                is_init: true,
                seq: 0,
                instant: minstant::Instant::now(),
                max_fn,
            })),
        }
    }

    pub fn generate(&self) -> Trigger {
        let mut guard = self.inner.lock();
        (*guard).generate()
    }
}

impl BulkTriggerInner {
    pub fn generate(&mut self) -> Trigger {
        let (_, max_micro, max_size) = (self.max_fn)();
        let now = minstant::Instant::now();
        let seq = &mut self.seq;
        let instant = &mut self.instant;
        if self.is_init {
            self.is_init = false;
            *instant = now;
            *seq += 1;
            Trigger {
                interval: Some(max_micro),
                is_now: false,
            }
        } else if *seq + 1 >= max_size {
            *instant = now;
            *seq = 0;
            Trigger {
                interval: None,
                is_now: true,
            }
        } else if now.duration_since(*instant).as_micros() as u64 > max_micro {
            *instant = now;
            *seq = 0;
            Trigger {
                interval: Some(max_micro),
                is_now: false,
            }
        } else {
            *seq += 1;
            Trigger {
                interval: None,
                is_now: false,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread::sleep;

    #[tokio::test]
    async fn test_publish_bulk() {
        let forwarder = ClusterForwarder::new_for_test(|| (true, 10, 10));
        let mut futures = vec![];
        for i in 0..5 {
            let request = PubReq {
                channel_family: Some("mock".to_owned()),
                channels: vec![format!("{}", i)],
                ..Default::default()
            };
            futures.push(async { forwarder.publish(request).await });
        }
        let mut i = 0;
        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|r| {
                log::info!("{:?}", r);
                i = i + 1;
            });
        assert_eq!(i, 5);
    }

    #[test]
    fn test_trigger_max_size() {
        let bulk_trigger: BulkTrigger = BulkTrigger::new(|| (true, 10, 2));
        let mut triggers = vec![];
        for _ in 0..4 {
            let trigger = bulk_trigger.generate();
            triggers.push(trigger);
        }

        assert!(triggers[0].is_now == false);
        assert!(triggers[0].interval.is_some());
        assert!(triggers[1].is_now == true);
        assert!(triggers[1].interval.is_none());
        assert!(triggers[2].is_now == false);
        assert!(triggers[2].interval.is_none());
        assert!(triggers[3].is_now == true);
        assert!(triggers[3].interval.is_none());
    }

    #[test]
    fn test_trigger_max_time() {
        // linux x86 200us works
        let bulk_trigger: BulkTrigger = BulkTrigger::new(|| (true, 10_000, 10));
        let mut triggers = vec![];
        for _ in 0..6 {
            sleep(std::time::Duration::from_micros(3000));
            let trigger = bulk_trigger.generate();
            triggers.push(trigger);
        }
        assert!(triggers[0].is_now == false);
        assert!(triggers[0].interval.is_some()); // 0ms, trigger
        assert!(triggers[1].is_now == false);
        assert!(triggers[1].interval.is_none()); // 3ms
        assert!(triggers[2].is_now == false);
        assert!(triggers[2].interval.is_none()); // 6ms
        assert!(triggers[3].is_now == false);
        // sleep time may shift, should trigger
        assert!(triggers[3].interval.is_some() || triggers[4].interval.is_some());
        assert!(triggers[4].is_now == false);
        assert!(triggers[5].is_now == false);
    }
}
