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
    async fn subscribe(
        &self,
        request: tonic::Request<SubReq>,
    ) -> Result<tonic::Response<SubResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.subscribe(request).await;
        Ok(tonic::Response::new(SubResp { success }))
    }

    async fn unsubscribe(
        &self,
        request: tonic::Request<UnSubReq>,
    ) -> Result<tonic::Response<UnSubResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.unsubscribe(request).await;
        Ok(tonic::Response::new(UnSubResp { success }))
    }

    async fn bulk_subscribe(
        &self,
        request: tonic::Request<BulkSubReq>,
    ) -> Result<tonic::Response<BulkSubResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.bulk_subscribe(request).await;
        Ok(tonic::Response::new(BulkSubResp { success }))
    }

    async fn push_conn(
        &self,
        request: tonic::Request<PushConnReq>,
    ) -> Result<tonic::Response<PushConnResp>, tonic::Status> {
        let request = request.into_inner();
        let success = self.operator.push_conn(request).await;
        Ok(tonic::Response::new(PushConnResp {
            success,
            sent: success,
        }))
    }

    async fn publish(
        &self,
        request: tonic::Request<PubReq>,
    ) -> Result<tonic::Response<PubResp>, tonic::Status> {
        let request = request.into_inner();
        let response = self.operator.publish(request).await;
        Ok(tonic::Response::new(response))
    }

    async fn bulk_publish(
        &self,
        request: tonic::Request<BulkPubReq>,
    ) -> Result<tonic::Response<BulkPubResp>, tonic::Status> {
        let bulk_request = request.into_inner();
        let futures = bulk_request.requests.into_iter().map(|req| async {
            let response = self.operator.publish(req).await;
            response
        });
        let responses: Vec<PubResp> = futures::future::join_all(futures)
            .await
            .into_iter()
            .collect();
        Ok(tonic::Response::new(BulkPubResp { responses }))
    }

    async fn get_conn_channels(
        &self,
        request: tonic::Request<GetConnsReq>,
    ) -> Result<tonic::Response<GetConnsResp>, tonic::Status> {
        let request = request.into_inner();
        let operator = self.operator.clone();
        let conn_channels = operator.get_conn_channels(request).await;

        Ok(tonic::Response::new(GetConnsResp {
            success: !conn_channels.is_empty(),
            conn_channels,
        }))
    }

    async fn get_channels(
        &self,
        request: tonic::Request<GetChannelsReq>,
    ) -> Result<tonic::Response<GetChannelsResp>, tonic::Status> {
        let request = request.into_inner();
        let namespace = request.namespace.clone();
        let channel_family = request.channel_family.clone();
        let channels = self.operator.get_channels(request).await;
        Ok(tonic::Response::new(GetChannelsResp {
            namespace,
            channel_family,
            channels: Some(Channels { channels }),
        }))
    }
}
