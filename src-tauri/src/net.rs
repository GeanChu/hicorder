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
use std::time::{Duration, Instant};

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

/// Diagnóstico de conectividade rodando dentro do processo do app.
/// Bate em api.attio.com sem auth (espera 401 rápido). Compara o client
/// padrão do reqwest com o nosso client (IPv4-only) p/ isolar a causa.
pub fn attio_selftest(key: Option<&str>, emails: &[String]) -> String {
    const URL: &str = "https://api.attio.com/v2/meetings?limit=1";
    let mut out = String::new();

    // 1. Client padrão do reqwest (sem nenhuma customização).
    let t = Instant::now();
    let plain = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build();
    match plain.and_then(|c| c.get(URL).send()) {
        Ok(r) => out.push_str(&format!("[padrao] OK status={} ({:?})\n", r.status(), t.elapsed())),
        Err(e) => out.push_str(&format!("[padrao] ERRO {e:?} ({:?})\n", t.elapsed())),
    }

    // 2. Nosso client (IPv4-only + native-tls + no-proxy).
    let t = Instant::now();
    match client(15).get(URL).send() {
        Ok(r) => out.push_str(&format!("[app-ipv4] OK status={} ({:?})\n", r.status(), t.elapsed())),
        Err(e) => out.push_str(&format!("[app-ipv4] ERRO {e:?} ({:?})\n", t.elapsed())),
    }

    // 3. Autenticado, se houver chave: isola auth e o param `participants`.
    if let Some(key) = key {
        let leg = |label: &str, url: String| -> String {
            let t = Instant::now();
            match client(20).get(&url).bearer_auth(key).send() {
                Ok(r) => {
                    let st = r.status();
                    let body = r.text().unwrap_or_default();
                    let snip: String = body.chars().take(160).collect();
                    format!("[{label}] OK status={st} ({:?}) body={snip}\n", t.elapsed())
                }
                Err(e) => format!("[{label}] ERRO {e:?} ({:?})\n", t.elapsed()),
            }
        };
        // A: authed sem participants.
        out.push_str(&leg("auth-sem-part", format!("{URL}")));
        // B: authed com participants (a chamada real que trava).
        if !emails.is_empty() {
            let joined = emails.join(",");
            let url = reqwest::Url::parse_with_params(
                "https://api.attio.com/v2/meetings",
                &[("limit", "25"), ("participants", joined.as_str())],
            )
            .map(|u| u.to_string())
            .unwrap_or_else(|_| URL.to_string());
            out.push_str(&leg("auth-com-part", url));
        }
    } else {
        out.push_str("[auth] sem chave configurada — pulei os testes autenticados\n");
    }

    out
}
