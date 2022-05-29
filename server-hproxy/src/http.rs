use crate::{Error, HpResult, CALL_ID, METHOD};
use http::HeaderMap;
use reqwest::header::{HeaderName, HeaderValue};
use reqwest::{Client as HttpClient, Method, RequestBuilder};
use server_base::MessageReq;
use std::collections::HashMap;
use std::str::FromStr;

pub struct HpRequestBuilder {
    pub metadata: HashMap<String, String>,
    pub url: String,
    pub body: String,
    pub conn_id: String,
    pub method: Option<String>,
    pub call_id: Option<String>,
}

impl HpRequestBuilder {
    pub fn build(&self, url: &str, c: HttpClient) -> HpResult<RequestBuilder> {
        if self.call_id.is_none() {
            return Err(Error::NoCallId {
                url: url.to_owned(),
            });
        }
        if let Some(method) = &self.method {
            let method =
                Method::from_str(&method.to_uppercase()).map_err(|_| Error::InvalidMethod {
                    method: method.to_owned(),
                    url: url.to_owned(),
                })?;
            let b = c
                .request(method, url)
                .headers(get_header_map(&self.metadata))
                .body(self.body.clone());
            log::debug!("build_request {:?}", b);
            Ok(b)
        } else {
            Err(Error::NoMethod {
                url: url.to_owned(),
            })
        }
    }
}

impl TryFrom<MessageReq> for HpRequestBuilder {
    type Error = Error;

    fn try_from(item: MessageReq) -> Result<Self, Self::Error> {
        let message = match item.message {
            Some(m) => m,
            None => return Err(Error::NoMessage { cid: item.cid }),
        };
        let mut metadata = message.metadata.clone();
        let method = metadata.remove(METHOD);
        let call_id = metadata.remove(CALL_ID);
        let conn_id = item.cid.clone();
        let url = message.path;
        let body = message.body;

        Ok(HpRequestBuilder {
            metadata,
            body: String::from_utf8(body).unwrap_or_default(),
            url,
            method,
            conn_id,
            call_id,
        })
    }
}

fn get_header_map(metadata: &HashMap<String, String>) -> HeaderMap {
    log::info!("get_header_map_in={:?}", metadata);
    let mut headers = HeaderMap::new();
    for (k, v) in metadata.iter() {
        match HeaderName::from_bytes(k.as_bytes()) {
            Ok(n) => match v.parse::<HeaderValue>() {
                Ok(v) => {
                    headers.insert(n, v);
                }
                Err(e) => log::warn!("invalid_header_value={:?}||header_key={}||err={}", v, k, e),
            },
            Err(e) => log::warn!("invalid_header_key={}||error={:?}", k, e),
        }
    }
    log::info!("get_header_map={:?}", headers);
    headers
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_call_id() {
        let builder = HpRequestBuilder {
            conn_id: "123".to_owned(),
            metadata: HashMap::new(),
            body: "".to_owned(),
            call_id: None,
            method: Some("GET".to_owned()),
            url: "/".to_owned(),
        };
        let c = HttpClient::new();
        assert_eq!(builder.build("url", c).is_err(), true);
    }

    #[test]
    fn test_no_method() {
        let builder = HpRequestBuilder {
            conn_id: "123".to_owned(),
            metadata: HashMap::new(),
            body: "".to_owned(),
            call_id: Some("123".to_owned()),
            method: None,
            url: "/".to_owned(),
        };
        let c = HttpClient::new();
        assert_eq!(builder.build("url", c).is_err(), true);
    }

    #[test]
    fn test_build() {
        use http::method::Method;

        let mut metadata = HashMap::new();
        metadata.insert("trace".to_owned(), "aaa".to_owned());
        let builder = HpRequestBuilder {
            conn_id: "123".to_owned(),
            metadata,
            body: "".to_owned(),
            call_id: Some("123".to_owned()),
            method: Some("GET".to_owned()),
            url: "/".to_owned(),
        };
        let c = HttpClient::new();
        let url = "http://www.bing.com/";
        let client = builder.build(url, c);
        assert_eq!(client.is_ok(), true);
        let b = client.unwrap().build();
        println!("{:?}", b);
        assert_eq!(b.is_ok(), true);
        let r = b.unwrap();
        assert_eq!(r.method(), Method::GET);
        assert_eq!(r.headers().get("trace").is_some(), true);
        assert_eq!(format!("{}", r.url()), url);
    }

    #[test]
    fn test_header_map() {
        let mut metadata: HashMap<String, String> = HashMap::new();
        metadata.insert(METHOD.to_owned(), "GET".to_owned());
        let header_map = get_header_map(&metadata);
        assert_eq!(header_map.get(METHOD.to_owned()).is_some(), true);
        assert_eq!(header_map.get("trace".to_owned()).is_some(), false);
    }
}
