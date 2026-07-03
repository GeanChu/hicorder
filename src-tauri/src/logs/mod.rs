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
    let line = format!("{ts} [{level}] {category}: {}\n", msg.replace('\n', " "));
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
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
