//! Comandos Tauri (IPC) expostos à UI.
//!
//! Stubs no PR1; comandos reais (start/stop_recording, transcribe, delete...) nos PRs seguintes.

/// Verificação de IPC. Substituído por comandos reais nos próximos PRs.
#[tauri::command]
pub fn ping(name: &str) -> String {
    format!("Call Recorder OK — olá, {name}")
}
