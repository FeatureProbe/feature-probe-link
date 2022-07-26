use cached::proc_macro::cached;
use parking_lot::Mutex;
use server_base::proto::{
    link_service_client::LinkServiceClient, BulkEmitResp, BulkJoinReq, ConnRoomReq, ConnRooms,
    EmitReq, EmitResp, EmitSidReq, EmitStatus, GetRoomsReq, JoinReq, LeaveReq,
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
    Emit(EmitReq, oneshot::Sender<Vec<EmitResp>>),
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

    pub async fn join(&self, node_id: &str, request: JoinReq) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .join(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }
        log::error!(
            "join failed: not found client for connectionId: {}",
            &request.sid
        );
        Ok(false)
    }

    pub async fn leave(&self, node_id: &str, request: LeaveReq) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .leave(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }

        log::warn!(
            "leave failed, no client for nodeId: {}, connId: {}",
            node_id,
            &request.sid
        );
        Ok(false)
    }

    pub async fn bulk_join(
        &self,
        node_id: &str,
        request: BulkJoinReq,
    ) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .bulk_join(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }
        log::warn!("bulk_join failed, no client for sid: {}", &request.sid);
        Ok(false)
    }

    pub async fn emit_sid(
        &self,
        node_id: &str,
        request: EmitSidReq,
    ) -> Result<bool, tonic::Status> {
        if let Some(client) = self.cluster.pick_client(node_id) {
            let response = LinkServiceClient::new(client)
                .emit_sid(timeout(request))
                .await?
                .into_inner();
            return Ok(response.success);
        }
        log::warn!(
            "emit_sid failed: no client for connectionId: {}",
            &request.sid
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

    pub async fn emit(&self, request: EmitReq) -> EmitResp {
        let (enable_bulk, _, _) = (self.grpc_bulk)();
        if enable_bulk {
            self.emit_bulk(request).await
        } else {
            self.emit_one_shot(request).await
        }
    }
    pub async fn emit_one_shot(&self, request: EmitReq) -> EmitResp {
        // TODO: Vec capacity from conf
        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = request.clone();
            futures.push(async move { client.emit(timeout(request)).await });
        }
        let mut rooms: HashSet<String> = HashSet::new();
        let mut success = true;
        let mut status: Vec<EmitStatus> = vec![];

        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    success = success && resp.success;
                    rooms.extend(resp.rooms);
                    status.extend(resp.status);
                }
                Err(e) => log::warn!("push_by_tags failed: {}", e),
            });
        EmitResp {
            success,
            rooms: rooms.into_iter().collect(),
            status,
        }
    }

    pub async fn emit_bulk(&self, request: EmitReq) -> EmitResp {
        let (tx, rx) = oneshot::channel();
        let trigger = self.bulk_trigger.generate();
        let request_clone = request.clone();
        let bulk_request = BulkRequest::Emit(request_clone, tx);
        self.append_bulk_request(bulk_request);
        let mut should_trigger = trigger.is_now;

        if let Some(interval) = trigger.interval {
            log::trace!("emit should wait {}", interval);
            tokio::time::sleep(tokio::time::Duration::from_micros(interval)).await;
            should_trigger = true;
        }
        if should_trigger {
            log::trace!("emit_bulk should trigger");
            self.inner_emit_bulk().await;
        }

        let mut rooms: HashSet<String> = HashSet::new();
        let mut success = true;
        let mut status: Vec<EmitStatus> = vec![];
        match rx.await {
            Ok(rs) => {
                log::trace!("push_by_tags_bulk recv response {:?}", rs);
                rs.into_iter().for_each(|resp| {
                    success = success && resp.success;
                    rooms.extend(resp.rooms);
                    status.extend(resp.status);
                });
            }
            Err(e) => log::error!("push_by_tags failed: {}", e),
        };

        EmitResp {
            success,
            rooms: rooms.into_iter().collect(),
            status,
        }
    }

    pub async fn get_conn_rooms(&self, request: ConnRoomReq) -> Vec<ConnRooms> {
        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = request.clone();
            futures.push(async move { client.get_conn_rooms(timeout(request)).await });
        }
        let mut conn_vec = vec![];
        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    conn_vec.extend(resp.rooms);
                }
                Err(e) => log::warn!("get_conn_rooms failed: {}", e),
            });

        conn_vec
    }

    pub async fn get_rooms(&self, request: GetRoomsReq) -> Vec<String> {
        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = request.clone();
            futures.push(async move { client.get_rooms(timeout(request)).await });
        }
        let mut vec = vec![];
        futures::future::join_all(futures)
            .await
            .into_iter()
            .for_each(|result| match result {
                Ok(resp) => {
                    let resp = resp.into_inner();
                    if let Some(rooms) = resp.rooms {
                        vec.extend(rooms.rooms);
                    }
                }
                Err(e) => log::warn!("get_rooms failed: {}", e),
            });

        vec
    }

    async fn inner_emit_bulk(&self) {
        let requests = self.clear_bulk_request();
        log::debug!("inner_emit_bulk {} requests", requests.len());
        let mut rs = Vec::new();
        let mut txs = Vec::new();
        for r in requests {
            match r {
                BulkRequest::Emit(r, t) => {
                    rs.push(r);
                    txs.push(t);
                }
            }
        }
        let futures = self.emit_bulk_futures(rs);
        let mut client_responses: Vec<VecDeque<EmitResp>> = Vec::with_capacity(txs.len());
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
        log::trace!("inner_emit_bulk each responses {:?}", client_responses);
        for tx in txs {
            let mut v = vec![];
            for client_response in &mut client_responses {
                if let Some(r) = client_response.pop_front() {
                    v.push(r);
                }
            }
            if let Err(e) = tx.send(v) {
                log::error!("inner_emit_bulk error: {:?}", e);
            }
        }
    }

    #[cfg(not(test))]
    fn emit_bulk_futures(
        &self,
        rs: Vec<EmitReq>,
    ) -> Vec<impl futures::Future<Output = Result<tonic::Response<BulkEmitResp>, tonic::Status>>>
    {
        use server_base::proto::BulkEmitReq;

        let mut futures = vec![];
        for (_node_id, client) in self.cluster.get_clients().rl().iter() {
            let mut client = LinkServiceClient::new(client.to_owned());
            let request = BulkEmitReq {
                requests: rs.clone(),
            };
            futures.push(async move { client.bulk_emit(timeout(request)).await });
        }
        futures
    }

    #[cfg(test)]
    fn emit_bulk_futures(
        &self,
        rs: Vec<EmitReq>,
    ) -> Vec<impl futures::Future<Output = Result<tonic::Response<BulkEmitResp>, tonic::Status>>>
    {
        let mut responses = vec![];
        for r in rs {
            let response = EmitResp {
                success: true,
                status: vec![EmitStatus {
                    rooms: None,
                    sent: true,
                }],
                rooms: r.rooms,
            };
            responses.push(response);
        }
        let mut futures = vec![];
        futures.push(async move { Ok(tonic::Response::new(BulkEmitResp { responses })) });
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
    async fn test_emit_bulk() {
        let forwarder = ClusterForwarder::new_for_test(|| (true, 10, 10));
        let mut futures = vec![];
        for i in 0..5 {
            let request = EmitReq {
                room_prefix: Some("mock".to_owned()),
                rooms: vec![format!("{}", i)],
                ..Default::default()
            };
            futures.push(async { forwarder.emit(request).await });
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
