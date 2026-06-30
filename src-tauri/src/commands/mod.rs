//! Comandos Tauri (IPC) expostos à UI.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Manager, State};

use crate::audio::recorder::{Recorder, RecordingInfo};
use crate::storage::{self, RecordingRow, TranscriptRow};
use crate::transcription::{OpenAiCompatible, Transcriber, TranscriptionConfig};
use crate::{audio, encode, settings};

#[derive(Serialize, Clone)]
pub struct AppSettings {
    pub default_language: String,
    pub endpoint_url: String,
    pub model: String,
    pub has_api_key: bool,
}

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<String>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_recording(app: AppHandle, recorder: State<Recorder>) -> Result<RecordingInfo, String> {
    let dir = recordings_dir(&app).map_err(|e| e.to_string())?;
    recorder.start(dir, new_id()).map_err(|e| e.to_string())
}

/// Para a gravação, mistura/encoda para Opus, persiste e retorna a linha.
#[tauri::command]
pub fn stop_recording(app: AppHandle, recorder: State<Recorder>) -> Result<RecordingRow, String> {
    let res = recorder.stop().map_err(|e| e.to_string())?;

    let dir = Path::new(&res.mic_path)
        .parent()
        .ok_or_else(|| "caminho da gravação inválido".to_string())?;
    // webm/opus: leve e aceito pelo endpoint OpenAI-compat da MiniMax (.ogg não é aceito).
    let out = dir.join("recording.webm");

    encode::mix_to_opus(&res.mic_path, res.system_path.as_deref(), &out).map_err(|e| e.to_string())?;

    // Encode OK: limpa os WAVs brutos (mantém só o .ogg leve).
    let _ = std::fs::remove_file(&res.mic_path);
    if let Some(sys) = &res.system_path {
        let _ = std::fs::remove_file(sys);
    }

    let size_bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) as i64;
    let row = RecordingRow {
        id: res.id,
        path: out.to_string_lossy().into_owned(),
        created_at: now_ms(),
        duration_s: res.duration_s,
        size_bytes,
    };

    let conn = storage::open(&db_path(&app).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    storage::insert(&conn, &row).map_err(|e| e.to_string())?;
    Ok(row)
}

#[tauri::command]
pub fn list_recordings(app: AppHandle) -> Result<Vec<RecordingRow>, String> {
    let conn = open_db(&app)?;
    storage::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    let conn = open_db(&app)?;
    let cfg = load_config(&conn).map_err(|e| e.to_string())?;
    let default_language = storage::get_setting(&conn, "default_language")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "pt".to_string());
    Ok(AppSettings {
        default_language,
        endpoint_url: cfg.endpoint_url,
        model: cfg.model,
        has_api_key: settings::has_api_key(),
    })
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    default_language: String,
    endpoint_url: String,
    model: String,
) -> Result<(), String> {
    let conn = open_db(&app)?;
    storage::set_setting(&conn, "default_language", &default_language).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "endpoint_url", &endpoint_url).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "model", &model).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_api_key(key: String) -> Result<(), String> {
    settings::set_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn transcribe(
    app: AppHandle,
    recording_id: String,
    language: String,
) -> Result<TranscriptRow, String> {
    let lang = if language.trim().is_empty() {
        "pt".to_string()
    } else {
        language
    };

    // Leituras síncronas no SQLite, sem segurar a conexão durante o await.
    let (path, provider) = {
        let conn = open_db(&app)?;
        let path = storage::recording_path(&conn, &recording_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "gravação não encontrada".to_string())?;
        let cfg = load_config(&conn).map_err(|e| e.to_string())?;
        let api_key = settings::get_api_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "configure a chave da API nas Configurações".to_string())?;
        (
            path,
            OpenAiCompatible {
                endpoint_url: cfg.endpoint_url,
                model: cfg.model,
                api_key,
            },
        )
    };

    // HTTP bloqueante numa thread de blocking (não trava a UI, não aninha runtime).
    let lang_http = lang.clone();
    let text = tauri::async_runtime::spawn_blocking(move || {
        provider.transcribe(Path::new(&path), &lang_http)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let row = TranscriptRow {
        recording_id,
        language: lang,
        text,
        created_at: now_ms(),
    };
    let conn = open_db(&app)?;
    storage::upsert_transcript(&conn, &row).map_err(|e| e.to_string())?;
    Ok(row)
}

#[tauri::command]
pub fn get_transcript(app: AppHandle, recording_id: String) -> Result<Option<TranscriptRow>, String> {
    let conn = open_db(&app)?;
    storage::get_transcript(&conn, &recording_id).map_err(|e| e.to_string())
}

fn open_db(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    storage::open(&db_path(app).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn load_config(conn: &rusqlite::Connection) -> anyhow::Result<TranscriptionConfig> {
    let d = TranscriptionConfig::default();
    Ok(TranscriptionConfig {
        endpoint_url: storage::get_setting(conn, "endpoint_url")?.unwrap_or(d.endpoint_url),
        model: storage::get_setting(conn, "model")?.unwrap_or(d.model),
    })
}

#[tauri::command]
pub fn recording_level(recorder: State<Recorder>) -> f32 {
    recorder.level()
}

#[tauri::command]
pub fn is_recording(recorder: State<Recorder>) -> bool {
    recorder.is_recording()
}

fn recordings_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?.join("recordings");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn db_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let base = app.path().app_data_dir()?;
    std::fs::create_dir_all(&base)?;
    Ok(base.join("callrec.db"))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id() -> String {
    format!("rec-{}", now_ms())
}
