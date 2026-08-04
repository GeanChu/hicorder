//! Log persistente para troubleshooting. Grava erros de API (e eventos
//! relevantes) num arquivo no diretório de dados do app, com timestamp e
//! categoria. Nunca grava segredos (chaves) — só status e corpo de erro.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use chrono::Local;
use tauri::{AppHandle, Manager};

const MAX_BYTES: u64 = 1_000_000; // rotaciona ~1MB p/ não crescer sem limite.

fn log_path(app: &AppHandle) -> Option<PathBuf> {
    let base = app.path().app_data_dir().ok()?;
    let _ = fs::create_dir_all(&base);
    Some(base.join("callrec.log"))
}

/// Anexa uma linha ao log. `level` ex.: "ERRO", "INFO". `category` ex.:
/// "transcricao", "resumo", "attio". Falhas de escrita são silenciosas.
pub fn log(app: &AppHandle, level: &str, category: &str, msg: &str) {
    let Some(path) = log_path(app) else { return };
    rotate_if_big(&path);
    let ts = Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("{ts} [{level}] {category}: {}\n", redact(msg).replace('\n', " "));
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// Mascara segredos antes de gravar. O que é logado inclui o **corpo cru** da
/// resposta do provedor e o Debug do erro HTTP, e nenhum dos dois é controlado
/// por nós: há provedor que ecoa a chave na mensagem de erro ("Incorrect API
/// key provided: sk-..."), e endpoint personalizado pode levar a chave na query
/// string, que aparece na URL do erro. Sem isto o log viraria um arquivo de
/// texto puro com credenciais.
fn redact(msg: &str) -> String {
    redact_secrets(&redact_url_paths(msg))
}

/// Tamanho a partir do qual um trecho de caminho de URL é tratado como segredo.
///
/// 16 é o ponto que separa os dois mundos observados: o maior trecho legítimo
/// dos endpoints que o app usa é `transcriptions` (15), enquanto segredos são
/// bem maiores (o token do ICS do Google tem 40). Endpoint nenhum é mascarado;
/// o segredo, sempre.
const SECRET_SEGMENT_LEN: usize = 16;

/// Mascara segredos embutidos no **caminho** de uma URL.
///
/// A URL do calendário (ICS) é uma credencial: quem tem o link lê a agenda
/// inteira, sem login. E ela cai no log toda vez que o refresh da agenda falha
/// (`falha ao buscar o ICS: ... /calendar/ical/<email>/private-<token>/basic.ics`).
/// O mascaramento por prefixo/query não pega isso, porque o segredo não é
/// parâmetro nem tem prefixo conhecido — é parte do caminho.
///
/// Host e trechos curtos ficam, para o log continuar servindo a diagnóstico.
fn redact_url_paths(msg: &str) -> String {
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;

    while let Some(pos) = find_url_start(rest) {
        out.push_str(&rest[..pos]);
        let tail = &rest[pos..];
        // A URL termina no primeiro caractere que não pode fazer parte dela.
        let end = tail
            .find(|c: char| c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ','))
            .unwrap_or(tail.len());
        let (url, after) = tail.split_at(end);
        out.push_str(&mask_path(url));
        rest = after;
    }
    out.push_str(rest);
    out
}

fn find_url_start(s: &str) -> Option<usize> {
    match (s.find("https://"), s.find("http://")) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// Reescreve só o caminho da URL; esquema, host e query seguem intactos (a
/// query já é tratada pelo mascaramento por parâmetro).
fn mask_path(url: &str) -> String {
    let Some(scheme_end) = url.find("://") else {
        return url.to_string();
    };
    let after_scheme = scheme_end + 3;
    // Fim do host: primeira '/' depois do esquema.
    let Some(host_len) = url[after_scheme..].find('/') else {
        return url.to_string(); // sem caminho, nada a mascarar
    };
    let path_start = after_scheme + host_len;
    // O caminho vai até a query/fragmento.
    let path_end = url[path_start..]
        .find(['?', '#'])
        .map(|i| path_start + i)
        .unwrap_or(url.len());

    let masked: Vec<String> = url[path_start..path_end]
        .split('/')
        .map(|seg| {
            if seg.chars().count() >= SECRET_SEGMENT_LEN {
                "[REDACTED]".to_string()
            } else {
                seg.to_string()
            }
        })
        .collect();

    format!("{}{}{}", &url[..path_start], masked.join("/"), &url[path_end..])
}

/// Mascaramento por prefixo de chave e por parâmetro de query.
fn redact_secrets(msg: &str) -> String {
    let mut out = String::with_capacity(msg.len());
    let bytes: Vec<char> = msg.chars().collect();
    let mut i = 0;

    // Prefixos conhecidos de chave (sk-, sk-cp-, gsk_, nvapi-, ey… de JWT).
    const PREFIXES: &[&str] = &["sk-", "gsk_", "nvapi-", "xai-", "AIza", "ghp_", "Bearer "];
    // Parâmetros de query que carregam segredo.
    const PARAMS: &[&str] = &["key=", "api_key=", "apikey=", "access_token=", "token=", "password="];

    'outer: while i < bytes.len() {
        let rest: String = bytes[i..].iter().collect();
        for p in PREFIXES {
            if rest.starts_with(p) {
                out.push_str(p);
                out.push_str("[REDACTED]");
                i += p.chars().count();
                // Consome o valor: caracteres típicos de token.
                while i < bytes.len()
                    && (bytes[i].is_ascii_alphanumeric() || bytes[i] == '_' || bytes[i] == '-')
                {
                    i += 1;
                }
                continue 'outer;
            }
        }
        for p in PARAMS {
            if rest.to_lowercase().starts_with(p) {
                out.push_str(p);
                out.push_str("[REDACTED]");
                i += p.chars().count();
                // Valor vai até o separador da query/URL.
                while i < bytes.len() && !matches!(bytes[i], '&' | ' ' | '"' | '\'' | '#' | ')') {
                    i += 1;
                }
                continue 'outer;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::redact;

    #[test]
    fn mascara_chave_ecoada_pelo_provedor() {
        let raw = "provedor retornou 401: Incorrect API key provided: sk-proj-AbC123xyz789. Check it.";
        let out = redact(raw);
        assert!(!out.contains("AbC123xyz789"), "chave vazou: {out}");
        assert!(out.contains("sk-[REDACTED]"));
        // O resto da mensagem continua legível para diagnóstico.
        assert!(out.contains("401"));
    }

    #[test]
    fn mascara_chave_na_query_string() {
        let raw = "erro em https://api.exemplo.com/v1/x?key=SEGREDO123&model=abc";
        let out = redact(raw);
        assert!(!out.contains("SEGREDO123"), "chave vazou: {out}");
        assert!(out.contains("key=[REDACTED]"));
        assert!(out.contains("model=abc"), "resto da URL deve sobreviver: {out}");
    }

    #[test]
    fn mascara_nvapi_e_bearer() {
        assert!(!redact("Authorization: Bearer nvapi-abc123def").contains("abc123def"));
        assert!(!redact("chave nvapi-XYZ789 invalida").contains("XYZ789"));
    }

    #[test]
    fn nao_altera_mensagem_sem_segredo() {
        let raw = "ffmpeg falhou: Error opening input files: End of file";
        assert_eq!(redact(raw), raw);
    }

    #[test]
    fn mascara_o_token_secreto_do_ics() {
        // Caso real: o refresh da agenda falha e a URL inteira cai no log.
        // Quem tem esse link lê a agenda toda, sem login.
        let raw = "falha ao buscar o ICS: error sending request for url \
                   (https://calendar.google.com/calendar/ical/gean%40hi.capital/private-d8f0f87151c6dadf6f0e507bca4883f9/basic.ics)";
        let out = redact(raw);
        assert!(!out.contains("d8f0f87151c6dadf6f0e507bca4883f9"), "token vazou: {out}");
        assert!(!out.contains("gean%40hi.capital"), "email vazou: {out}");
        // O que sobra ainda diagnostica: dá para ver que é o ICS do Google.
        assert!(out.contains("calendar.google.com"), "host sumiu: {out}");
        assert!(out.contains("basic.ics"), "arquivo sumiu: {out}");
    }

    #[test]
    fn nao_mascara_endpoints_legitimos() {
        // Nenhum endpoint que o app usa pode ser destruído pelo mascaramento,
        // senão o log perde o valor de diagnóstico.
        for url in [
            "https://api.groq.com/openai/v1/audio/transcriptions",
            "https://api.openai.com/v1/chat/completions",
            "https://generativelanguage.googleapis.com/v1beta/openai/chat/completions",
            "https://integrate.api.nvidia.com/v1/chat/completions",
            "https://api.attio.com/v2/objects/people/records/query",
            "https://api.affinity.co/auth/whoami",
            "https://api.attio.com/v2/meetings?limit=1",
        ] {
            let out = redact(&format!("provedor retornou 500 em {url}"));
            assert!(out.contains(url), "endpoint foi mascarado: {out}");
        }
    }

    #[test]
    fn mascara_caminho_secreto_preservando_a_query() {
        let raw = "erro em https://host.com/a/umsegredomuitolongoaqui123/b?model=abc";
        let out = redact(raw);
        assert!(!out.contains("umsegredomuitolongoaqui123"), "segredo vazou: {out}");
        assert!(out.contains("model=abc"), "query destruída: {out}");
        assert!(out.contains("host.com/a/"), "início do caminho destruído: {out}");
    }

    #[test]
    fn url_sem_caminho_nao_quebra() {
        let raw = "falha ao conectar em https://api.exemplo.com";
        assert_eq!(redact(raw), raw);
    }

    #[test]
    fn mascara_as_duas_urls_da_mesma_linha() {
        let raw = "de https://a.com/tokensecretodemais0001/x para https://b.com/tokensecretodemais0002/y";
        let out = redact(raw);
        assert!(!out.contains("tokensecretodemais0001"), "1ª vazou: {out}");
        assert!(!out.contains("tokensecretodemais0002"), "2ª vazou: {out}");
    }

    #[test]
    fn segredo_no_caminho_e_chave_na_query_juntos() {
        let raw = "https://h.com/umcaminhosecretolongo/x?api_key=SEGREDO123&m=1";
        let out = redact(raw);
        assert!(!out.contains("umcaminhosecretolongo"), "caminho vazou: {out}");
        assert!(!out.contains("SEGREDO123"), "chave vazou: {out}");
        assert!(out.contains("m=1"), "resto da query sumiu: {out}");
    }
}

fn rotate_if_big(path: &PathBuf) {
    let Ok(meta) = fs::metadata(path) else { return };
    if meta.len() <= MAX_BYTES {
        return;
    }
    // Mantém a metade final do arquivo.
    if let Ok(data) = fs::read(path) {
        let keep = data.len() / 2;
        let tail = &data[data.len() - keep..];
        let _ = fs::write(path, tail);
    }
}

pub fn read(app: &AppHandle) -> String {
    log_path(app)
        .and_then(|p| fs::read_to_string(p).ok())
        .unwrap_or_default()
}

pub fn clear(app: &AppHandle) {
    if let Some(p) = log_path(app) {
        let _ = fs::write(p, b"");
    }
}

/// Converte um erro cru de provedor/HTTP numa mensagem clara para leigos.
/// O texto cru continua indo para o log; aqui é só o que o usuário lê.
pub fn humanize(raw: &str) -> String {
    let r = raw.to_lowercase();
    let has = |needles: &[&str]| needles.iter().any(|n| r.contains(n));

    if has(&["timedout", "timed out", "dns", "connect", "sem rota", "falha na conexão", "falha de conex"]) {
        return "Falha de conexão com o provedor. Verifique sua internet e tente novamente.".into();
    }
    if has(&["401", "unauthorized", "invalid_api_key", "invalid api key", "chave inválida", "authentication"]) {
        return "A chave de API parece inválida ou expirada. Confira a chave nas Configurações.".into();
    }
    if has(&["403", "forbidden", "permission"]) {
        return "A chave não tem permissão para esta operação. Verifique as permissões no provedor.".into();
    }
    if has(&["429", "insufficient_quota", "insufficient quota", "rate limit", "quota", "billing", "credit", "saldo"]) {
        return "Limite de uso atingido ou créditos esgotados no provedor. Verifique seu plano/saldo e tente novamente mais tarde.".into();
    }
    if has(&["404", "not found"]) {
        return "Endereço (endpoint) não encontrado. Confira o provedor selecionado nas Configurações.".into();
    }
    if has(&["500", "502", "503", "504", "server error", "bad gateway", "unavailable"]) {
        return "O provedor está instável no momento (erro no servidor). Tente novamente em instantes.".into();
    }
    "Ocorreu um erro ao falar com o provedor. Abra os Logs nas Configurações para ver os detalhes técnicos.".into()
}
