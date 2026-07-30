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
