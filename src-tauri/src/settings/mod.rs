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

/// Identificador sob o qual a chave é guardada — uma chave por (tipo, host).
///
/// A chave vale para todos os modelos do mesmo provedor: a da Groq serve todos
/// os Whisper, a da MiniMax todos os modelos dela, e a `nvapi-` da NVIDIA todos
/// os modelos do NIM (é chave de conta, não de modelo — o botão "Get API Key"
/// de cada página de modelo entrega/rotaciona a mesma chave da conta).
///
/// `kind` separa transcrição de resumo: o mesmo host pode servir os dois com
/// credenciais diferentes (ex.: OpenAI Whisper vs GPT).
pub fn key_scope(kind: &str, endpoint_url: &str, _model: &str) -> String {
    format!("{kind}:{}", host_of(&endpoint_url.to_lowercase()))
}

// Transcrição (Groq/Whisper).
//
// Sem fallback para a chave única antiga: não há como saber a que provedor ela
// pertencia, e usar um palpite mandaria a credencial de um provedor para outro.
// Escopo vazio = sem chave; o usuário informa a chave uma vez por provedor.
pub fn set_api_key(endpoint_url: &str, model: &str, key: &str) -> Result<()> {
    set_key(&key_scope("stt", endpoint_url, model), key)
}
pub fn get_api_key(endpoint_url: &str, model: &str) -> Result<Option<String>> {
    get_key(&key_scope("stt", endpoint_url, model))
}
pub fn has_api_key(endpoint_url: &str, model: &str) -> bool {
    matches!(get_api_key(endpoint_url, model), Ok(Some(_)))
}

// Resumo (LLM).
pub fn set_summary_key(endpoint_url: &str, model: &str, key: &str) -> Result<()> {
    set_key(&key_scope("summary", endpoint_url, model), key)
}
pub fn get_summary_key(endpoint_url: &str, model: &str) -> Result<Option<String>> {
    get_key(&key_scope("summary", endpoint_url, model))
}
pub fn has_summary_key(endpoint_url: &str, model: &str) -> bool {
    matches!(get_summary_key(endpoint_url, model), Ok(Some(_)))
}

// Nota: a chave única antiga (antes das chaves por escopo) NÃO é migrada
// automaticamente. Não há registro de a qual provedor ela pertencia, e um
// palpite errado enviaria a credencial de um provedor para outro. As entradas
// antigas ficam órfãs no keychain, sem nunca serem lidas.

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
    fn nvidia_usa_a_mesma_chave_em_todos_os_modelos() {
        // A nvapi- é chave de conta: vale para todo o catálogo do NIM.
        let ep = "https://integrate.api.nvidia.com/v1/chat/completions";
        let a = key_scope("summary", ep, "minimaxai/minimax-m3");
        let b = key_scope("summary", ep, "deepseek-ai/deepseek-v4-pro");
        assert_eq!(a, b);
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
