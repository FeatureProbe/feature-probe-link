mod http;
mod proxy;

use crate::http::HpRequestBuilder;
use proxy::HttpProxy;
use server_base::PushConn;
use snafu::prelude::*;

type HpResult<T> = std::result::Result<T, Error>;

pub const HPROXY_NAMESPACE: &str = "__http_proxy";

const HTTP_STATUS_CODE: &str = "__status_code";
const CALL_ID: &str = "__call_id";
const METHOD: &str = "__method";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("No CallId {url}"))]
    NoCallId { url: String },

    #[snafu(display("No Method {url}"))]
    NoMethod { url: String },

    #[snafu(display("Invalid Method {method} of {url}"))]
    InvalidMethod { method: String, url: String },

    #[snafu(display("No Message in MessageReq of conn {cid}"))]
    NoMessage { cid: String },
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
