//! Chaves de API por SO.
//!
//! - Windows/Linux: keychain do SO (crate `keyring`) — sem fricção.
//! - macOS: arquivo protegido (0600) na pasta de dados do app. Apps não
//!   assinados/notarizados sofrem prompts repetidos do chaveiro "login" no
//!   macOS; o arquivo evita isso. As chaves ficam só na pasta local do usuário.
//!
//! Preferências não-secretas (idioma, endpoints, etc.) ficam no SQLite.

use anyhow::Result;

const SERVICE: &str = "com.hicapital.hicorder";
const TRANSCRIPTION_KEY: &str = "transcription_api_key";
const SUMMARY_KEY: &str = "summary_api_key";
const ATTIO_KEY: &str = "attio_api_key";

// ---- macOS: arquivo protegido (sem keychain) ----
#[cfg(target_os = "macos")]
mod store {
    use super::SERVICE;
    use anyhow::{anyhow, Result};
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn secrets_path() -> Result<PathBuf> {
        let home = std::env::var_os("HOME").ok_or_else(|| anyhow!("HOME não definido"))?;
        Ok(PathBuf::from(home)
            .join("Library/Application Support")
            .join(SERVICE)
            .join("secrets.json"))
    }

    fn read_all() -> BTreeMap<String, String> {
        secrets_path()
            .ok()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    pub fn set(user: &str, key: &str) -> Result<()> {
        let path = secrets_path()?;
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let mut map = read_all();
        map.insert(user.to_string(), key.to_string());
        let json = serde_json::to_string(&map)?;
        std::fs::write(&path, json)?;
        // Apenas o dono lê/escreve.
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
        Ok(())
    }

    pub fn get(user: &str) -> Result<Option<String>> {
        Ok(read_all().get(user).cloned())
    }
}

// ---- Windows/Linux: keychain do SO ----
#[cfg(not(target_os = "macos"))]
mod store {
    use super::SERVICE;
    use anyhow::{anyhow, Result};
    use keyring::Entry;

    const OLD_SERVICE: &str = "com.hicapital.callrecorder";

    fn entry(user: &str) -> Result<Entry> {
        Entry::new(SERVICE, user).map_err(|e| anyhow!("keychain: {e}"))
    }

    pub fn set(user: &str, key: &str) -> Result<()> {
        entry(user)?
            .set_password(key)
            .map_err(|e| anyhow!("keychain: {e}"))
    }

    pub fn get(user: &str) -> Result<Option<String>> {
        match entry(user)?.get_password() {
            Ok(p) => Ok(Some(p)),
            Err(keyring::Error::NoEntry) => migrate_old(user),
            Err(e) => Err(anyhow!("keychain: {e}")),
        }
    }

    /// Migração preguiçosa do serviço antigo (Call Recorder).
    fn migrate_old(user: &str) -> Result<Option<String>> {
        let old = Entry::new(OLD_SERVICE, user).map_err(|e| anyhow!("keychain: {e}"))?;
        match old.get_password() {
            Ok(p) => {
                let _ = set(user, &p);
                Ok(Some(p))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(anyhow!("keychain: {e}")),
        }
    }
}

fn set_key(user: &str, key: &str) -> Result<()> {
    store::set(user, key)
}
fn get_key(user: &str) -> Result<Option<String>> {
    store::get(user)
}

/// Host da URL, sem esquema nem caminho ("https://api.groq.com/x" → "api.groq.com").
fn host_of(url: &str) -> String {
    url.split("://")
        .nth(1)
        .unwrap_or(url)
        .split('/')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

/// Identificador sob o qual a chave é guardada — define quantas chaves distintas
/// o app mantém.
///
/// - **NVIDIA NIM**: uma chave POR MODELO. O programa de incentivo da NVIDIA
///   emite uma chave para cada modelo, então trocar de modelo troca de chave.
/// - **Demais provedores**: uma chave por host. A mesma chave da Groq vale para
///   todos os modelos Whisper; a da MiniMax vale para todos os modelos dela.
pub fn key_scope(kind: &str, endpoint_url: &str, model: &str) -> String {
    let ep = endpoint_url.to_lowercase();
    if ep.contains("integrate.api.nvidia.com") {
        return format!("{kind}:nvidia:{}", model.trim().to_lowercase());
    }
    format!("{kind}:{}", host_of(&ep))
}

/// Lê a chave do escopo; se ainda não houver, cai na chave única antiga (de
/// antes das chaves por provedor) para não quebrar quem já tinha configurado.
fn get_scoped(scope: &str, legacy: &str) -> Result<Option<String>> {
    if let Some(v) = get_key(scope)? {
        return Ok(Some(v));
    }
    get_key(legacy)
}

// Transcrição (Groq/Whisper).
pub fn set_api_key(endpoint_url: &str, model: &str, key: &str) -> Result<()> {
    set_key(&key_scope("stt", endpoint_url, model), key)
}
pub fn get_api_key(endpoint_url: &str, model: &str) -> Result<Option<String>> {
    get_scoped(&key_scope("stt", endpoint_url, model), TRANSCRIPTION_KEY)
}
pub fn has_api_key(endpoint_url: &str, model: &str) -> bool {
    matches!(get_api_key(endpoint_url, model), Ok(Some(_)))
}

// Resumo (LLM).
pub fn set_summary_key(endpoint_url: &str, model: &str, key: &str) -> Result<()> {
    set_key(&key_scope("summary", endpoint_url, model), key)
}
pub fn get_summary_key(endpoint_url: &str, model: &str) -> Result<Option<String>> {
    get_scoped(&key_scope("summary", endpoint_url, model), SUMMARY_KEY)
}
pub fn has_summary_key(endpoint_url: &str, model: &str) -> bool {
    matches!(get_summary_key(endpoint_url, model), Ok(Some(_)))
}

#[cfg(test)]
mod tests {
    use super::key_scope;

    #[test]
    fn mesma_chave_para_modelos_da_mesma_familia() {
        let a = key_scope("stt", "https://api.groq.com/openai/v1/audio/transcriptions", "whisper-large-v3");
        let b = key_scope("stt", "https://api.groq.com/openai/v1/audio/transcriptions", "whisper-large-v3-turbo");
        assert_eq!(a, b);

        let m1 = key_scope("summary", "https://api.minimax.io/v1/chat/completions", "MiniMax-M3");
        let m2 = key_scope("summary", "https://api.minimax.io/v1/chat/completions", "MiniMax-Text-01");
        assert_eq!(m1, m2);
    }

    #[test]
    fn nvidia_tem_chave_por_modelo() {
        let ep = "https://integrate.api.nvidia.com/v1/chat/completions";
        let a = key_scope("summary", ep, "minimaxai/minimax-m3");
        let b = key_scope("summary", ep, "deepseek-ai/deepseek-v4-pro");
        assert_ne!(a, b);
    }

    #[test]
    fn provedores_diferentes_nao_compartilham() {
        let g = key_scope("summary", "https://api.openai.com/v1/chat/completions", "gpt-4o");
        let m = key_scope("summary", "https://api.minimax.io/v1/chat/completions", "MiniMax-M3");
        assert_ne!(g, m);
    }

    #[test]
    fn stt_e_resumo_nao_compartilham_mesmo_host() {
        let a = key_scope("stt", "https://api.openai.com/v1/audio/transcriptions", "whisper-1");
        let b = key_scope("summary", "https://api.openai.com/v1/chat/completions", "gpt-4o");
        assert_ne!(a, b);
    }
}

// Attio (CRM).
pub fn set_attio_key(key: &str) -> Result<()> {
    set_key(ATTIO_KEY, key)
}
pub fn get_attio_key() -> Result<Option<String>> {
    get_key(ATTIO_KEY)
}
pub fn has_attio_key() -> bool {
    matches!(get_attio_key(), Ok(Some(_)))
}
