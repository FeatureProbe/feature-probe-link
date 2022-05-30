use anyhow::{bail, Result};
use rustls::{Certificate, PrivateKey, ServerConfig};
use server_base::LifeCycle;
use std::path::Path;
use std::sync::Arc;
use tokio_rustls::TlsAcceptor;

pub const ALPN_TCP: &str = "tcp";
pub const ALPN_WS: &str = "http/1.1";

pub struct TlsAcceptorBuilder {
    cert_chain: Vec<Certificate>,
    private_key: Option<PrivateKey>,
}

impl TlsAcceptorBuilder {
    pub fn new() -> Self {
        TlsAcceptorBuilder {
            cert_chain: vec![],
            private_key: None,
        }
    }

    pub fn with_cert_pem_file(&mut self, path: &Path) -> Result<()> {
        let (certs, key) = crate::tls::cert_key(path)?;
        self.cert_chain = certs;
        self.private_key = Some(key);
        Ok(())
    }

    pub fn build(self) -> Option<TlsAcceptor> {
        let private_key = self.private_key?;
        if let Ok(mut server_config) = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(self.cert_chain, private_key)
        {
            server_config.alpn_protocols = vec![Vec::from(ALPN_TCP), Vec::from(ALPN_WS)];
            server_config.key_log = Arc::new(rustls::KeyLogFile::new());
            Some(TlsAcceptor::from(Arc::new(server_config)))
        } else {
            log::warn!("tls acceptor build failed");
            None
        }
    }
}

pub fn cert_key(path: &std::path::Path) -> Result<(Vec<Certificate>, PrivateKey)> {
    let key = std::fs::read(path)?;
    let key = if path.extension().map_or(false, |x| x == "der") {
        rustls::PrivateKey(key)
    } else {
        let pkcs8 = rustls_pemfile::pkcs8_private_keys(&mut &*key)?;
        match pkcs8.into_iter().next() {
            Some(x) => rustls::PrivateKey(x),
            None => {
                let rsa = rustls_pemfile::rsa_private_keys(&mut &*key)?;
                match rsa.into_iter().next() {
                    Some(x) => rustls::PrivateKey(x),
                    None => bail!("No Private Key Found"),
                }
            }
        }
    };
    let cert_chain = std::fs::read(path)?;
    let cert_chain = if path.extension().map_or(false, |x| x == "der") {
        vec![rustls::Certificate(cert_chain)]
    } else {
        rustls_pemfile::certs(&mut &*cert_chain)?
            .into_iter()
            .map(rustls::Certificate)
            .collect()
    };

    Ok((cert_chain, key))
}

pub async fn listen_tls(
    addr: String,
    timeout_secs: u64,
    cert: Option<String>,
    lifecycle: Arc<dyn LifeCycle>,
) {
    use crate::listener::Builder;
    let builder = Builder::new(addr.as_str())
        .with_cert(cert)
        .with_timeout(timeout_secs)
        .with_lifecycle(lifecycle);
    match builder.build() {
        Ok(listener) => listener.listen().await,
        Err(e) => panic!("start share tls server failed: {}", e),
    }
}
