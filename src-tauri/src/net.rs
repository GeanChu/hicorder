//! Cliente HTTP compartilhado.
//!
//! - TLS nativo (store de certificados do SO — passa por inspeção HTTPS ex. Kaspersky).
//! - Sem proxy do sistema (proxy do Kaspersky quebrava as conexões).
//! - Força IPv4 via resolver DNS custom (filtra getaddrinfo p/ IPv4). Redes com
//!   IPv6 sem rota travavam a conexão até o timeout; `local_address(0.0.0.0)`
//!   não resolvia. O resolver remove IPv6 do jogo por completo.
//! - Timeout explícito.

use std::net::{SocketAddr, ToSocketAddrs};
use std::sync::Arc;
use std::time::Duration;

use reqwest::dns::{Addrs, Name, Resolve, Resolving};

/// Resolver que devolve só endereços IPv4 (via getaddrinfo do SO).
struct Ipv4Only;

impl Resolve for Ipv4Only {
    fn resolve(&self, name: Name) -> Resolving {
        let host = name.as_str().to_owned();
        Box::pin(async move {
            let addrs = (host.as_str(), 0u16)
                .to_socket_addrs()
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            let v4: Vec<SocketAddr> = addrs.filter(|a| a.is_ipv4()).collect();
            let it: Addrs = Box::new(v4.into_iter());
            Ok(it)
        })
    }
}

pub fn client(timeout_secs: u64) -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .use_native_tls()
        .no_proxy()
        .dns_resolver(Arc::new(Ipv4Only))
        .timeout(Duration::from_secs(timeout_secs))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}
