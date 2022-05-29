mod cluster;
mod dispatcher;
mod operator;
mod server;

pub use cluster::ClusterForwarder;
pub use dispatcher::Dispatcher;
pub use operator::ClusterOperator;
pub use server::ServerStub;
use server_base::{proto::link_service_server::LinkServiceServer, tonic, CoreOperation};

pub fn grpc_listen<O>(addr: &str, operator: O)
where
    O: CoreOperation + 'static,
{
    log::info!("gRPC server is listening at: {}", addr);
    let stub = ServerStub::new(addr.to_owned(), operator);
    let addr = addr.parse().expect("failed to parse grpc addr");
    let config = server_state::config();
    let max_concurrent_streams = config.get::<u32>("grpc_max_concurrent_streams").ok();
    let initial_stream_window_size = config
        .get::<u32>("grpc_initial_stream_window_size")
        .unwrap_or(65535 * 100); // 6.5M
    let initial_connection_window_size = config
        .get::<u32>("grpc_initial_connection_window_size")
        .unwrap_or(65535 * 200); // 13M
    let concurrency_limit_per_connection = config
        .get::<usize>("grpc_concurrency_limit_per_connection")
        .unwrap_or(1024);

    server_base::tokio::spawn(async move {
        if let Err(e) = tonic::transport::Server::builder()
            .max_concurrent_streams(max_concurrent_streams)
            .initial_stream_window_size(initial_stream_window_size)
            .initial_connection_window_size(initial_connection_window_size)
            .concurrency_limit_per_connection(concurrency_limit_per_connection)
            .add_service(LinkServiceServer::new(stub))
            .serve(addr)
            .await
        {
            log::error!("failed to start grpc {} : {}", addr, e);
        }
    });
}
