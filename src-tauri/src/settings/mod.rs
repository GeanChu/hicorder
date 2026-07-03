//! Segredos no keychain do SO (crate `keyring`). Nunca em texto puro, nunca logado.
//!
//! As preferências não-secretas (idioma padrão, URL/modelo do provedor) ficam no
//! SQLite (ver `storage`).

use anyhow::{anyhow, Result};
use keyring::Entry;

const SERVICE: &str = "com.hicapital.hicorder";
/// Nome antigo (Call Recorder) — mantido para migrar chaves já salvas.
const OLD_SERVICE: &str = "com.hicapital.callrecorder";
const TRANSCRIPTION_KEY: &str = "transcription_api_key";
const SUMMARY_KEY: &str = "summary_api_key";
const ATTIO_KEY: &str = "attio_api_key";

fn entry(user: &str) -> Result<Entry> {
    Entry::new(SERVICE, user).map_err(|e| anyhow!("keychain: {e}"))
}

fn set_key(user: &str, key: &str) -> Result<()> {
    entry(user)?.set_password(key).map_err(|e| anyhow!("keychain: {e}"))
}

fn get_key(user: &str) -> Result<Option<String>> {
    match entry(user)?.get_password() {
        Ok(p) => Ok(Some(p)),
        Err(keyring::Error::NoEntry) => migrate_old_key(user),
        Err(e) => Err(anyhow!("keychain: {e}")),
    }
}

/// Migração preguiçosa: se a chave só existe no serviço antigo, copia para o
/// novo e passa a usá-lo. A entrada antiga é mantida (rollback possível).
fn migrate_old_key(user: &str) -> Result<Option<String>> {
    let old = Entry::new(OLD_SERVICE, user).map_err(|e| anyhow!("keychain: {e}"))?;
    match old.get_password() {
        Ok(p) => {
            let _ = set_key(user, &p);
            Ok(Some(p))
        }
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(anyhow!("keychain: {e}")),
    }
}

// Transcrição (Groq/Whisper).
pub fn set_api_key(key: &str) -> Result<()> {
    set_key(TRANSCRIPTION_KEY, key)
}
pub fn get_api_key() -> Result<Option<String>> {
    get_key(TRANSCRIPTION_KEY)
}
pub fn has_api_key() -> bool {
    matches!(get_api_key(), Ok(Some(_)))
}

// Resumo (MiniMax-M3, sk-cp).
pub fn set_summary_key(key: &str) -> Result<()> {
    set_key(SUMMARY_KEY, key)
}
pub fn get_summary_key() -> Result<Option<String>> {
    get_key(SUMMARY_KEY)
}
pub fn has_summary_key() -> bool {
    matches!(get_summary_key(), Ok(Some(_)))
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
