#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Packet {
    #[prost(oneof = "packet::Packet", tags = "1, 2, 3")]
    pub packet: ::core::option::Option<packet::Packet>,
}
/// Nested message and enum types in `Packet`.
pub mod packet {
    #[derive(Clone, PartialEq, ::prost::Oneof)]
    pub enum Packet {
        #[prost(message, tag = "1")]
        Ping(super::Ping),
        #[prost(message, tag = "2")]
        Pong(super::Pong),
        #[prost(message, tag = "3")]
        Message(super::Message),
    }
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Ping {
    #[prost(uint64, tag = "1")]
    pub timestamp: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Pong {
    #[prost(uint64, tag = "1")]
    pub timestamp: u64,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Message {
    #[prost(string, tag = "1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub path: ::prost::alloc::string::String,
    #[prost(map = "string, string", tag = "3")]
    pub metadata:
        ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(bytes = "vec", tag = "4")]
    pub body: ::prost::alloc::vec::Vec<u8>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Trace {
    #[prost(string, tag = "1")]
    pub trace_id: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub span_id: ::prost::alloc::string::String,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubReq {
    #[prost(string, tag = "1")]
    pub cid: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "3")]
    pub channel_family: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "4")]
    pub channel: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct SubResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BulkSubReq {
    #[prost(string, tag = "1")]
    pub cid: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(map = "string, string", tag = "3")]
    pub channels:
        ::std::collections::HashMap<::prost::alloc::string::String, ::prost::alloc::string::String>,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BulkSubResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnSubReq {
    #[prost(string, tag = "1")]
    pub cid: ::prost::alloc::string::String,
    #[prost(string, tag = "2")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "3")]
    pub channel_family: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "4")]
    pub channel: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct UnSubResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PubReq {
    #[prost(message, optional, tag = "1")]
    pub message: ::core::option::Option<Message>,
    #[prost(string, optional, tag = "2")]
    pub channel_family: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, repeated, tag = "3")]
    pub channels: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(bool, tag = "4")]
    pub success_channel: bool,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BulkPubReq {
    #[prost(message, repeated, tag = "1")]
    pub requests: ::prost::alloc::vec::Vec<PubReq>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushConnReq {
    #[prost(string, tag = "1")]
    pub cid: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub message: ::core::option::Option<Message>,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PushConnResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(bool, tag = "2")]
    pub sent: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PubResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
    /// channels is emtry when PubReq success_channel is false
    #[prost(string, repeated, tag = "2")]
    pub channels: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
    #[prost(message, repeated, tag = "3")]
    pub status: ::prost::alloc::vec::Vec<PubStatus>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BulkPubResp {
    #[prost(message, repeated, tag = "1")]
    pub responses: ::prost::alloc::vec::Vec<PubResp>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConnsReq {
    #[prost(string, tag = "1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "2")]
    pub channel_family: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(string, tag = "3")]
    pub channel: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct Channels {
    #[prost(string, repeated, tag = "1")]
    pub channel: ::prost::alloc::vec::Vec<::prost::alloc::string::String>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct ConnChannels {
    #[prost(string, tag = "1")]
    pub cid: ::prost::alloc::string::String,
    #[prost(map = "string, message", tag = "2")]
    pub channels: ::std::collections::HashMap<::prost::alloc::string::String, Channels>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct PubStatus {
    #[prost(message, optional, tag = "1")]
    pub conn_channels: ::core::option::Option<ConnChannels>,
    #[prost(bool, tag = "2")]
    pub sent: bool,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetConnsResp {
    #[prost(bool, tag = "1")]
    pub success: bool,
    #[prost(message, repeated, tag = "2")]
    pub conn_channels: ::prost::alloc::vec::Vec<ConnChannels>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetChannelsReq {
    #[prost(string, tag = "1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "2")]
    pub channel_family: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "3")]
    pub channels: ::core::option::Option<Channels>,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct GetChannelsResp {
    #[prost(string, tag = "1")]
    pub namespace: ::prost::alloc::string::String,
    #[prost(string, optional, tag = "2")]
    pub channel_family: ::core::option::Option<::prost::alloc::string::String>,
    #[prost(message, optional, tag = "3")]
    pub channels: ::core::option::Option<Channels>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageReq {
    #[prost(string, tag = "1")]
    pub cid: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub message: ::core::option::Option<Message>,
    #[prost(map = "string, message", tag = "3")]
    pub channels: ::std::collections::HashMap<::prost::alloc::string::String, Channels>,
    #[prost(message, optional, tag = "10")]
    pub trace: ::core::option::Option<Trace>,
}
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct MessageResp {
    #[prost(int64, tag = "1")]
    pub num: i64,
}
/// Generated client implementations.
pub mod channel_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct ChannelServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl ChannelServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> ChannelServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> ChannelServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            ChannelServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn subscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::SubReq>,
        ) -> Result<tonic::Response<super::SubResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/Subscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn un_subscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::UnSubReq>,
        ) -> Result<tonic::Response<super::UnSubResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/UnSubscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn bulk_subscribe(
            &mut self,
            request: impl tonic::IntoRequest<super::BulkSubReq>,
        ) -> Result<tonic::Response<super::BulkSubResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/BulkSubscribe",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn push_conn(
            &mut self,
            request: impl tonic::IntoRequest<super::PushConnReq>,
        ) -> Result<tonic::Response<super::PushConnResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/PushConn",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn publish(
            &mut self,
            request: impl tonic::IntoRequest<super::PubReq>,
        ) -> Result<tonic::Response<super::PubResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/Publish",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn bulk_pushlish(
            &mut self,
            request: impl tonic::IntoRequest<super::BulkPubReq>,
        ) -> Result<tonic::Response<super::PubResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/BulkPushlish",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_conn_channels(
            &mut self,
            request: impl tonic::IntoRequest<super::GetConnsReq>,
        ) -> Result<tonic::Response<super::GetConnsResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/GetConnChannels",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
        pub async fn get_channels(
            &mut self,
            request: impl tonic::IntoRequest<super::GetChannelsReq>,
        ) -> Result<tonic::Response<super::GetChannelsResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.ChannelService/GetChannels",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated client implementations.
pub mod message_service_client {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    #[derive(Debug, Clone)]
    pub struct MessageServiceClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl MessageServiceClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: std::convert::TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> MessageServiceClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> MessageServiceClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<http::Request<tonic::body::BoxBody>>>::Error:
                Into<StdError> + Send + Sync,
        {
            MessageServiceClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with `gzip`.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_gzip(mut self) -> Self {
            self.inner = self.inner.send_gzip();
            self
        }
        /// Enable decompressing responses with `gzip`.
        #[must_use]
        pub fn accept_gzip(mut self) -> Self {
            self.inner = self.inner.accept_gzip();
            self
        }
        pub async fn handle_message(
            &mut self,
            request: impl tonic::IntoRequest<super::MessageReq>,
        ) -> Result<tonic::Response<super::MessageResp>, tonic::Status> {
            self.inner.ready().await.map_err(|e| {
                tonic::Status::new(
                    tonic::Code::Unknown,
                    format!("Service was not ready: {}", e.into()),
                )
            })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/featureprobe.link.service.MessageService/HandleMessage",
            );
            self.inner.unary(request.into_request(), path, codec).await
        }
    }
}
/// Generated server implementations.
pub mod channel_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with ChannelServiceServer.
    #[async_trait]
    pub trait ChannelService: Send + Sync + 'static {
        async fn subscribe(
            &self,
            request: tonic::Request<super::SubReq>,
        ) -> Result<tonic::Response<super::SubResp>, tonic::Status>;
        async fn un_subscribe(
            &self,
            request: tonic::Request<super::UnSubReq>,
        ) -> Result<tonic::Response<super::UnSubResp>, tonic::Status>;
        async fn bulk_subscribe(
            &self,
            request: tonic::Request<super::BulkSubReq>,
        ) -> Result<tonic::Response<super::BulkSubResp>, tonic::Status>;
        async fn push_conn(
            &self,
            request: tonic::Request<super::PushConnReq>,
        ) -> Result<tonic::Response<super::PushConnResp>, tonic::Status>;
        async fn publish(
            &self,
            request: tonic::Request<super::PubReq>,
        ) -> Result<tonic::Response<super::PubResp>, tonic::Status>;
        async fn bulk_pushlish(
            &self,
            request: tonic::Request<super::BulkPubReq>,
        ) -> Result<tonic::Response<super::PubResp>, tonic::Status>;
        async fn get_conn_channels(
            &self,
            request: tonic::Request<super::GetConnsReq>,
        ) -> Result<tonic::Response<super::GetConnsResp>, tonic::Status>;
        async fn get_channels(
            &self,
            request: tonic::Request<super::GetChannelsReq>,
        ) -> Result<tonic::Response<super::GetChannelsResp>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct ChannelServiceServer<T: ChannelService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: ChannelService> ChannelServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for ChannelServiceServer<T>
    where
        T: ChannelService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/featureprobe.link.service.ChannelService/Subscribe" => {
                    #[allow(non_camel_case_types)]
                    struct SubscribeSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::SubReq> for SubscribeSvc<T> {
                        type Response = super::SubResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::SubReq>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).subscribe(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = SubscribeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/UnSubscribe" => {
                    #[allow(non_camel_case_types)]
                    struct UnSubscribeSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::UnSubReq> for UnSubscribeSvc<T> {
                        type Response = super::UnSubResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::UnSubReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).un_subscribe(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = UnSubscribeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/BulkSubscribe" => {
                    #[allow(non_camel_case_types)]
                    struct BulkSubscribeSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::BulkSubReq> for BulkSubscribeSvc<T> {
                        type Response = super::BulkSubResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BulkSubReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).bulk_subscribe(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BulkSubscribeSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/PushConn" => {
                    #[allow(non_camel_case_types)]
                    struct PushConnSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::PushConnReq> for PushConnSvc<T> {
                        type Response = super::PushConnResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::PushConnReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).push_conn(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PushConnSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/Publish" => {
                    #[allow(non_camel_case_types)]
                    struct PublishSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::PubReq> for PublishSvc<T> {
                        type Response = super::PubResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(&mut self, request: tonic::Request<super::PubReq>) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).publish(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = PublishSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/BulkPushlish" => {
                    #[allow(non_camel_case_types)]
                    struct BulkPushlishSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::BulkPubReq> for BulkPushlishSvc<T> {
                        type Response = super::PubResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::BulkPubReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).bulk_pushlish(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = BulkPushlishSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/GetConnChannels" => {
                    #[allow(non_camel_case_types)]
                    struct GetConnChannelsSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::GetConnsReq> for GetConnChannelsSvc<T> {
                        type Response = super::GetConnsResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetConnsReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_conn_channels(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetConnChannelsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                "/featureprobe.link.service.ChannelService/GetChannels" => {
                    #[allow(non_camel_case_types)]
                    struct GetChannelsSvc<T: ChannelService>(pub Arc<T>);
                    impl<T: ChannelService> tonic::server::UnaryService<super::GetChannelsReq> for GetChannelsSvc<T> {
                        type Response = super::GetChannelsResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::GetChannelsReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).get_channels(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = GetChannelsSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: ChannelService> Clone for ChannelServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: ChannelService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: ChannelService> tonic::transport::NamedService for ChannelServiceServer<T> {
        const NAME: &'static str = "featureprobe.link.service.ChannelService";
    }
}
/// Generated server implementations.
pub mod message_service_server {
    #![allow(unused_variables, dead_code, missing_docs, clippy::let_unit_value)]
    use tonic::codegen::*;
    ///Generated trait containing gRPC methods that should be implemented for use with MessageServiceServer.
    #[async_trait]
    pub trait MessageService: Send + Sync + 'static {
        async fn handle_message(
            &self,
            request: tonic::Request<super::MessageReq>,
        ) -> Result<tonic::Response<super::MessageResp>, tonic::Status>;
    }
    #[derive(Debug)]
    pub struct MessageServiceServer<T: MessageService> {
        inner: _Inner<T>,
        accept_compression_encodings: (),
        send_compression_encodings: (),
    }
    struct _Inner<T>(Arc<T>);
    impl<T: MessageService> MessageServiceServer<T> {
        pub fn new(inner: T) -> Self {
            Self::from_arc(Arc::new(inner))
        }
        pub fn from_arc(inner: Arc<T>) -> Self {
            let inner = _Inner(inner);
            Self {
                inner,
                accept_compression_encodings: Default::default(),
                send_compression_encodings: Default::default(),
            }
        }
        pub fn with_interceptor<F>(inner: T, interceptor: F) -> InterceptedService<Self, F>
        where
            F: tonic::service::Interceptor,
        {
            InterceptedService::new(Self::new(inner), interceptor)
        }
    }
    impl<T, B> tonic::codegen::Service<http::Request<B>> for MessageServiceServer<T>
    where
        T: MessageService,
        B: Body + Send + 'static,
        B::Error: Into<StdError> + Send + 'static,
    {
        type Response = http::Response<tonic::body::BoxBody>;
        type Error = std::convert::Infallible;
        type Future = BoxFuture<Self::Response, Self::Error>;
        fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            Poll::Ready(Ok(()))
        }
        fn call(&mut self, req: http::Request<B>) -> Self::Future {
            let inner = self.inner.clone();
            match req.uri().path() {
                "/featureprobe.link.service.MessageService/HandleMessage" => {
                    #[allow(non_camel_case_types)]
                    struct HandleMessageSvc<T: MessageService>(pub Arc<T>);
                    impl<T: MessageService> tonic::server::UnaryService<super::MessageReq> for HandleMessageSvc<T> {
                        type Response = super::MessageResp;
                        type Future = BoxFuture<tonic::Response<Self::Response>, tonic::Status>;
                        fn call(
                            &mut self,
                            request: tonic::Request<super::MessageReq>,
                        ) -> Self::Future {
                            let inner = self.0.clone();
                            let fut = async move { (*inner).handle_message(request).await };
                            Box::pin(fut)
                        }
                    }
                    let accept_compression_encodings = self.accept_compression_encodings;
                    let send_compression_encodings = self.send_compression_encodings;
                    let inner = self.inner.clone();
                    let fut = async move {
                        let inner = inner.0;
                        let method = HandleMessageSvc(inner);
                        let codec = tonic::codec::ProstCodec::default();
                        let mut grpc = tonic::server::Grpc::new(codec).apply_compression_config(
                            accept_compression_encodings,
                            send_compression_encodings,
                        );
                        let res = grpc.unary(method, req).await;
                        Ok(res)
                    };
                    Box::pin(fut)
                }
                _ => Box::pin(async move {
                    Ok(http::Response::builder()
                        .status(200)
                        .header("grpc-status", "12")
                        .header("content-type", "application/grpc")
                        .body(empty_body())
                        .unwrap())
                }),
            }
        }
    }
    impl<T: MessageService> Clone for MessageServiceServer<T> {
        fn clone(&self) -> Self {
            let inner = self.inner.clone();
            Self {
                inner,
                accept_compression_encodings: self.accept_compression_encodings,
                send_compression_encodings: self.send_compression_encodings,
            }
        }
    }
    impl<T: MessageService> Clone for _Inner<T> {
        fn clone(&self) -> Self {
            Self(self.0.clone())
        }
    }
    impl<T: std::fmt::Debug> std::fmt::Debug for _Inner<T> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{:?}", self.0)
        }
    }
    impl<T: MessageService> tonic::transport::NamedService for MessageServiceServer<T> {
        const NAME: &'static str = "featureprobe.link.service.MessageService";
    }
}
