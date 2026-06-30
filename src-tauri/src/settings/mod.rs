//! Preferências do app e segredos.
//!
//! Implementação nos PR5/PR6.
//! - Preferências (idioma padrão "pt-BR", "gravar todos") via tauri-plugin-store.
//! - Chave da API no keychain do SO (crate `keyring`), nunca em texto puro.
