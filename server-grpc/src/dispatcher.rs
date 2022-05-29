use async_trait::async_trait;
use cached::proc_macro::cached;
use server_base::proto::message_service_client::MessageServiceClient;
use server_base::proto::MessageReq;
use server_base::tonic::Request;
use server_base::Dispatch;
use server_state::Services;
use std::sync::Arc;
use std::time::Duration;

#[cached(time = 60)]
fn grpc_dispatch_retries() -> u8 {
    server_state::config()
        .get_int("grpc_dispatch_retries")
        .unwrap_or(3) as u8
}

#[derive(Clone)]
pub struct Dispatcher {
    clients: Arc<Services>,
    zone: Option<String>,
}

#[cached(time = 60)]
fn grpc_timeout() -> Duration {
    let timeout = server_state::config().service_grpc_timeout_ms();
    Duration::from_millis(timeout)
}

#[async_trait]
impl Dispatch for Dispatcher {
    async fn dispatch(&self, namespace: String, request: MessageReq) -> bool {
        for _ in 0..grpc_dispatch_retries() {
            let mut request = Request::new(request.clone());
            request.set_timeout(grpc_timeout());
            match self.clients.pick_client(&namespace, self.zone.clone()) {
                Some((addr, client)) => {
                    let client = client.as_ref().to_owned();
                    match MessageServiceClient::new(client)
                        .handle_message(request)
                        .await
                    {
                        Ok(resp) => {
                            let resp = resp.into_inner();
                            return resp.success_num > 0;
                        }
                        Err(e) => {
                            log::warn!("on_message_async_opt err:{}, {}, {}", namespace, addr, e);
                        }
                    }
                }
                None => {
                    log::warn!("service client unavailable for namespace: {}", namespace,);
                    return false;
                }
            }
        }
        false
    }
}

impl Default for Dispatcher {
    fn default() -> Self {
        let clients = Arc::new(server_state::service_client_state());
        let zone = server_state::get_zone();
        Self { clients, zone }
    }
}
