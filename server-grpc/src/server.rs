use server_base::proto::link_service_server::LinkService;
use server_base::proto::*;
use server_base::{tonic, CoreOperation};

#[derive(Clone, Default)]
pub struct ServerStub<O: CoreOperation> {
    operator: O,
    _addr: String,
}

impl<O: CoreOperation> ServerStub<O> {
    pub fn new(_addr: String, operator: O) -> Self {
        Self { _addr, operator }
    }
}

#[tonic::async_trait]
impl<O: CoreOperation> LinkService for ServerStub<O> {
    async fn join(
        &self,
        request: tonic::Request<JoinReq>,
    ) -> Result<tonic::Response<JoinResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.join(request).await;
        Ok(tonic::Response::new(JoinResp { success }))
    }

    async fn leave(
        &self,
        request: tonic::Request<LeaveReq>,
    ) -> Result<tonic::Response<LeaveResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.leave(request).await;
        Ok(tonic::Response::new(LeaveResp { success }))
    }

    async fn bulk_join(
        &self,
        request: tonic::Request<BulkJoinReq>,
    ) -> Result<tonic::Response<BulkJoinResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.bulk_join(request).await;
        Ok(tonic::Response::new(BulkJoinResp { success }))
    }

    async fn emit_sid(
        &self,
        request: tonic::Request<EmitSidReq>,
    ) -> Result<tonic::Response<EmitSidResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.emit_sid(request).await;
        Ok(tonic::Response::new(EmitSidResp {
            success,
            sent: success,
        }))
    }

    async fn emit(
        &self,
        request: tonic::Request<EmitReq>,
    ) -> Result<tonic::Response<EmitResp>, tonic::Status> {
        let request = request.into_inner();
        let response = self.operator.emit(request).await;
        Ok(tonic::Response::new(response))
    }

    async fn bulk_emit(
        &self,
        request: tonic::Request<BulkEmitReq>,
    ) -> Result<tonic::Response<BulkEmitResp>, tonic::Status> {
        let bulk_request = request.into_inner();
        let futures = bulk_request.requests.into_iter().map(|req| async {
            let response = self.operator.emit(req).await;
            response
        });
        let responses: Vec<EmitResp> = futures::future::join_all(futures)
            .await
            .into_iter()
            .collect();
        Ok(tonic::Response::new(BulkEmitResp { responses }))
    }

    async fn get_conn_rooms(
        &self,
        request: tonic::Request<ConnRoomReq>,
    ) -> Result<tonic::Response<ConnRoomResp>, tonic::Status> {
        let request = request.into_inner();
        let operator = self.operator.clone();
        let rooms = operator.get_conn_rooms(request).await;

        Ok(tonic::Response::new(ConnRoomResp {
            success: !rooms.is_empty(),
            rooms,
        }))
    }

    async fn get_rooms(
        &self,
        request: tonic::Request<GetRoomsReq>,
    ) -> Result<tonic::Response<GetChannelsResp>, tonic::Status> {
        let request = request.into_inner();
        let namespace = request.namespace.clone();
        let room_prefix = request.room_prefix.clone();
        let rooms = self.operator.get_rooms(request).await;
        Ok(tonic::Response::new(GetChannelsResp {
            namespace,
            room_prefix,
            rooms: Some(Rooms { rooms }),
        }))
    }
}
