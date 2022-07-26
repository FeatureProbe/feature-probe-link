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
    async fn join(&self, request: JoinReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.sid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.join(request).await
            } else {
                match self.inner.cluster.join(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("join failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "join failed: node_id CAN NOT be extracted from sid: {}",
                &request.sid
            );
            false
        }
    }

    async fn leave(&self, request: LeaveReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.sid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.leave(request).await
            } else {
                match self.inner.cluster.leave(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("leave failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "leave failed: node_id CAN NOT be extracted from sid: {}",
                &request.sid
            );
            false
        }
    }

    async fn bulk_join(&self, request: BulkJoinReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.sid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.bulk_join(request).await
            } else {
                match self.inner.cluster.bulk_join(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("bulk_join failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "bulk_join failed: not found client for sid: {}",
                &request.sid
            );
            false
        }
    }

    async fn emit_sid(&self, request: EmitSidReq) -> bool {
        if let Some(node_id) = IdGen::node_id(&request.sid) {
            if self.inner.node.is_myself(&node_id) {
                self.inner.myself.emit_sid(request).await
            } else {
                match self.inner.cluster.emit_sid(&node_id, request).await {
                    Ok(is_success) => is_success,
                    Err(e) => {
                        log::error!("emit_sid failed! err = {:?}", e);
                        false
                    }
                }
            }
        } else {
            log::error!(
                "emit_sid failed: not found client for sid: {}",
                &request.sid
            );
            false
        }
    }

    async fn emit(&self, request: EmitReq) -> EmitResp {
        let (mut local_resp, cluster_resp) = futures::join!(
            self.inner.myself.emit(request.clone()),
            self.inner.cluster.emit(request)
        );
        let success = local_resp.success && cluster_resp.success;
        let mut rooms: Vec<String> = cluster_resp.rooms;
        let mut status = cluster_resp.status;
        status.append(&mut local_resp.status);
        rooms.append(&mut local_resp.rooms);
        // remove duplicated
        rooms.sort();
        rooms.dedup();

        EmitResp {
            success,
            rooms,
            status,
        }
    }

    async fn get_conn_rooms(&self, request: ConnRoomReq) -> Vec<ConnRooms> {
        let mut local_channels = self.inner.myself.get_conn_rooms(request.clone()).await;
        let mut rooms = self.inner.cluster.get_conn_rooms(request).await;
        rooms.append(&mut local_channels);
        rooms
    }

    async fn get_rooms(&self, request: GetRoomsReq) -> Vec<String> {
        let mut local_channels = self.inner.myself.get_rooms(request.clone()).await;
        let mut rooms = self.inner.cluster.get_rooms(request).await;
        rooms.append(&mut local_channels);
        rooms
    }
}
