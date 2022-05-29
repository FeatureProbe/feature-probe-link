use super::{ClusterForwarder, CoreOperation};
use async_trait::async_trait;
use server_base::proto::*;
use server_base::IdGen;
use server_state::NodeOperation;
use std::sync::Arc;

#[derive(Clone)]
pub struct ClusterOperator<N, C>
where
    N: NodeOperation,
    C: CoreOperation,
{
    inner: Arc<Inner<N, C>>,
}

struct Inner<N, C>
where
    N: NodeOperation,
    C: CoreOperation,
{
    cluster: ClusterForwarder,
    node: N,
    myself: C,
}

impl<N, C> Inner<N, C>
where
    N: NodeOperation,
    C: CoreOperation,
{
    pub fn new(node: N, cluster: ClusterForwarder, myself: C) -> Self {
        Self {
            node,
            cluster,
            myself,
        }
    }
}

impl<N, C> ClusterOperator<N, C>
where
    N: NodeOperation,
    C: CoreOperation,
{
    pub fn new(node_operator: N, core_operator: C, cluster: ClusterForwarder) -> Self {
        let inner = Arc::new(Inner::new(node_operator, cluster, core_operator));
        Self { inner }
    }
}

#[async_trait]
impl<N, C> CoreOperation for ClusterOperator<N, C>
where
    N: NodeOperation,
    C: CoreOperation,
{
    async fn subscribe(&self, request: SubReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.cid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.subscribe(request).await
            } else {
                match self.inner.cluster.subscribe(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("bind_tag failed! err =  {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "bind_tag failed: node_id CAN NOT be extracted from cid: {}",
                &request.cid
            );
            false
        }
    }

    async fn unsubscribe(&self, request: UnSubReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.cid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.unsubscribe(request).await
            } else {
                match self.inner.cluster.unsubscribe(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("unsubscribe failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "unsubscribe failed: node_id CAN NOT be extracted from cid: {}",
                &request.cid
            );
            false
        }
    }

    async fn bulk_subscribe(&self, request: BulkSubReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.cid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.bulk_subscribe(request).await
            } else {
                match self.inner.cluster.bulk_subscribe(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("bulk_subscribe failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "bulk_subscribe failed: not found client for cid: {}",
                &request.cid
            );
            false
        }
    }

    async fn push_conn(&self, request: PushConnReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.cid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.push_conn(request).await
            } else {
                match self.inner.cluster.push_conn(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("push_conn failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "push_conn failed: not found client for cid: {}",
                &request.cid
            );
            false
        }
    }

    async fn publish(&self, request: PubReq) -> PubResp {
        let (mut local_resp, cluster_resp) = futures::join!(
            self.inner.myself.publish(request.clone()),
            self.inner.cluster.publish(request)
        );
        let success = local_resp.success && cluster_resp.success;
        let mut channels: Vec<String> = cluster_resp.channels;
        let mut status = cluster_resp.status;
        status.append(&mut local_resp.status);
        channels.append(&mut local_resp.channels);
        // remove duplicated
        channels.sort();
        channels.dedup();

        PubResp {
            success,
            channels,
            status,
        }
    }

    async fn get_conn_channels(&self, request: GetConnsReq) -> Vec<ConnChannels> {
        let mut local_channels = self.inner.myself.get_conn_channels(request.clone()).await;
        let mut channels = self.inner.cluster.get_conn_channels(request).await;
        channels.append(&mut local_channels);
        channels
    }

    async fn get_channels(&self, request: GetChannelsReq) -> Vec<String> {
        let mut local_channels = self.inner.myself.get_channels(request.clone()).await;
        let mut channels = self.inner.cluster.get_channels(request).await;
        channels.append(&mut local_channels);
        channels
    }
}
