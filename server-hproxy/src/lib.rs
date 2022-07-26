mod http;
mod proxy;

use crate::http::HpRequestBuilder;
use proxy::HttpProxy;
use server_base::PushConn;
use thiserror::Error;

type HpResult<T> = std::result::Result<T, Error>;

pub const HPROXY_NAMESPACE: &str = "__http_proxy";

const HTTP_STATUS_CODE: &str = "__status_code";
const CALL_ID: &str = "__call_id";
const METHOD: &str = "__method";

#[derive(Debug, Error)]
pub enum Error {
    #[error("No CallId {url}")]
    NoCallId { url: String },

    #[error("No Method {url}")]
    NoMethod { url: String },

    #[error("Invalid Method {method} of {url}")]
    InvalidMethod { method: String, url: String },

    #[error("No Message in MessageReq of conn {sid}")]
    NoMessage { sid: String },
}

pub fn build_http_proxy<G>(pusher: G) -> Option<HttpProxy<G>>
where
    G: PushConn + Clone,
{
    if server_state::config().hproxy_map().is_empty() {
        log::info!("not set hproxy!");
        return None;
    }
    Some(HttpProxy::new(pusher))
}
