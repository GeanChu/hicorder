//! Comandos Tauri (IPC) expostos à UI.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::audio::recorder::{Recorder, RecordingInfo};
use crate::storage::{self, MeetingRow, RecordingRow, SummaryRow, TranscriptRow};
use crate::summary::{self, SummaryConfig};
use crate::transcription::{OpenAiCompatible, Transcriber, TranscriptionConfig};
use crate::{audio, encode, meetings, settings};

#[derive(Serialize, Clone)]
pub struct AppSettings {
    pub default_language: String,
    pub endpoint_url: String,
    pub model: String,
    pub has_api_key: bool,
    // Resumo (MiniMax-M3) — opcional.
    pub summary_endpoint_url: String,
    pub summary_model: String,
    pub has_summary_key: bool,
    // Calendário (ICS).
    pub ics_url: String,
    pub record_all: bool,
}

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<String>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_recording(app: AppHandle) -> Result<RecordingInfo, String> {
    start_recording_core(&app)
}

/// Para a gravação, mistura/encoda para Opus, persiste e retorna a linha.
#[tauri::command]
pub fn stop_recording(app: AppHandle) -> Result<RecordingRow, String> {
    stop_recording_core(&app)
}

/// Núcleo de iniciar — chamável pelo command, pelo tray e pelo scheduler.
pub fn start_recording_core(app: &AppHandle) -> Result<RecordingInfo, String> {
    let dir = recordings_dir(app).map_err(|e| e.to_string())?;
    let info = app
        .state::<Recorder>()
        .start(dir, new_id())
        .map_err(|e| e.to_string())?;
    let _ = app.emit("recording-changed", true);
    Ok(info)
}

pub fn stop_recording_core(app: &AppHandle) -> Result<RecordingRow, String> {
    let res = app.state::<Recorder>().stop().map_err(|e| e.to_string())?;
    let dir = Path::new(&res.mic_path)
        .parent()
        .ok_or_else(|| "caminho da gravação inválido".to_string())?;

    // Faixas separadas (Opus/.webm): mic = "Você", sistema = "Participantes".
    let mic_out = dir.join("mic.webm");
    encode::mix_to_opus(&res.mic_path, None, &mic_out).map_err(|e| e.to_string())?;

    let system_path = match &res.system_path {
        Some(sys) => {
            let so = dir.join("system.webm");
            match encode::mix_to_opus(sys, None, &so) {
                Ok(()) => Some(so.to_string_lossy().into_owned()),
                Err(e) => {
                    eprintln!("[encode] faixa do sistema falhou: {e}");
                    None
                }
            }
        }
        None => None,
    };

    // Faixa mixada (os dois lados juntos) só para reprodução. Best-effort.
    let mix_out = dir.join("recording.webm");
    let _ = encode::mix_to_opus(&res.mic_path, res.system_path.as_deref(), &mix_out);

    // Encode OK: remove os WAVs brutos.
    let _ = std::fs::remove_file(&res.mic_path);
    if let Some(sys) = &res.system_path {
        let _ = std::fs::remove_file(sys);
    }

    let mut size_bytes = std::fs::metadata(&mic_out).map(|m| m.len()).unwrap_or(0);
    if let Some(sp) = &system_path {
        size_bytes += std::fs::metadata(sp).map(|m| m.len()).unwrap_or(0);
    }
    size_bytes += std::fs::metadata(&mix_out).map(|m| m.len()).unwrap_or(0);

    let row = RecordingRow {
        id: res.id,
        path: mic_out.to_string_lossy().into_owned(),
        system_path,
        created_at: now_ms(),
        duration_s: res.duration_s,
        size_bytes: size_bytes as i64,
    };

    let conn = open_db(app)?;
    storage::insert(&conn, &row).map_err(|e| e.to_string())?;
    let _ = app.emit("recording-changed", false);
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
    let scfg = load_summary_config(&conn).map_err(|e| e.to_string())?;
    let ics_url = storage::get_setting(&conn, "ics_url")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let record_all = storage::get_setting(&conn, "record_all")
        .map_err(|e| e.to_string())?
        .map(|v| v == "1")
        .unwrap_or(false);
    Ok(AppSettings {
        default_language,
        endpoint_url: cfg.endpoint_url,
        model: cfg.model,
        has_api_key: settings::has_api_key(),
        summary_endpoint_url: scfg.endpoint_url,
        summary_model: scfg.model,
        has_summary_key: settings::has_summary_key(),
        ics_url,
        record_all,
    })
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    default_language: String,
    endpoint_url: String,
    model: String,
    summary_endpoint_url: String,
    summary_model: String,
    ics_url: String,
    record_all: bool,
) -> Result<(), String> {
    let conn = open_db(&app)?;
    storage::set_setting(&conn, "default_language", &default_language).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "endpoint_url", &endpoint_url).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "model", &model).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "summary_endpoint_url", &summary_endpoint_url)
        .map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "summary_model", &summary_model).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "ics_url", &ics_url).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "record_all", if record_all { "1" } else { "0" })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_api_key(key: String) -> Result<(), String> {
    settings::set_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_summary_key(key: String) -> Result<(), String> {
    settings::set_summary_key(&key).map_err(|e| e.to_string())
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
    let (mic_path, system_path, provider) = {
        let conn = open_db(&app)?;
        let (mic, sys) = storage::recording_paths(&conn, &recording_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "gravação não encontrada".to_string())?;
        let cfg = load_config(&conn).map_err(|e| e.to_string())?;
        let api_key = settings::get_api_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "configure a chave da API nas Configurações".to_string())?;
        (
            mic,
            sys,
            OpenAiCompatible {
                endpoint_url: cfg.endpoint_url,
                model: cfg.model,
                api_key,
            },
        )
    };

    // "Você" = microfone. HTTP bloqueante em thread de blocking (não trava a UI).
    let p_mic = provider.clone();
    let mic_for_http = mic_path.clone();
    let lang_mic = lang.clone();
    let mic_segs = tauri::async_runtime::spawn_blocking(move || {
        p_mic.transcribe(Path::new(&mic_for_http), &lang_mic)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    // "Participantes" = áudio do sistema (falha aqui não derruba a transcrição do mic).
    let sys_segs = if let Some(sp) = system_path {
        let p_sys = provider.clone();
        let lang_sys = lang.clone();
        match tauri::async_runtime::spawn_blocking(move || {
            p_sys.transcribe(Path::new(&sp), &lang_sys)
        })
        .await
        {
            Ok(Ok(segs)) => segs,
            Ok(Err(e)) => {
                eprintln!("[transcribe] sistema falhou: {e}");
                Vec::new()
            }
            Err(e) => {
                eprintln!("[transcribe] sistema panic: {e}");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Intercala por timestamp e rotula.
    let mut tagged: Vec<(f64, &'static str, String)> = Vec::new();
    for s in mic_segs {
        tagged.push((s.start, "Você", s.text));
    }
    for s in sys_segs {
        tagged.push((s.start, "Participantes", s.text));
    }
    tagged.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    let text = tagged
        .iter()
        .map(|(start, label, txt)| format!("[{}] {}: {}", fmt_timestamp(*start), label, txt))
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err("transcrição vazia".to_string());
    }

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

#[tauri::command]
pub fn delete_recording(app: AppHandle, recording_id: String) -> Result<(), String> {
    let conn = open_db(&app)?;
    let paths = storage::recording_paths(&conn, &recording_id).map_err(|e| e.to_string())?;
    storage::delete_recording(&conn, &recording_id).map_err(|e| e.to_string())?;
    // Apaga a pasta da gravação (mic.webm + system.webm).
    if let Some((mic, _sys)) = paths {
        if let Some(dir) = Path::new(&mic).parent() {
            let _ = std::fs::remove_dir_all(dir);
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn generate_summary(app: AppHandle, recording_id: String) -> Result<SummaryRow, String> {
    let (transcript_text, cfg, api_key) = {
        let conn = open_db(&app)?;
        let t = storage::get_transcript(&conn, &recording_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "transcreva a gravação antes de resumir".to_string())?;
        let cfg = load_summary_config(&conn).map_err(|e| e.to_string())?;
        let api_key = settings::get_summary_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "configure a chave do Resumo (MiniMax) nas Configurações".to_string())?;
        (t.text, cfg, api_key)
    };

    let text = tauri::async_runtime::spawn_blocking(move || {
        summary::summarize(&cfg, &api_key, &transcript_text)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())?;

    let row = SummaryRow {
        recording_id,
        text,
        created_at: now_ms(),
    };
    let conn = open_db(&app)?;
    storage::upsert_summary(&conn, &row).map_err(|e| e.to_string())?;
    Ok(row)
}

#[tauri::command]
pub fn get_summary(app: AppHandle, recording_id: String) -> Result<Option<SummaryRow>, String> {
    let conn = open_db(&app)?;
    storage::get_summary(&conn, &recording_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn refresh_meetings(app: AppHandle) -> Result<Vec<MeetingRow>, String> {
    let (ics_url, record_all) = {
        let conn = open_db(&app)?;
        let url = storage::get_setting(&conn, "ics_url")
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        let ra = storage::get_setting(&conn, "record_all")
            .map_err(|e| e.to_string())?
            .map(|v| v == "1")
            .unwrap_or(false);
        (url, ra)
    };
    if ics_url.trim().is_empty() {
        return Err("configure a URL do calendário (ICS) nas Configurações".to_string());
    }

    let parsed = tauri::async_runtime::spawn_blocking(move || meetings::fetch_and_parse(&ics_url))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())?;

    let conn = open_db(&app)?;
    for m in &parsed {
        storage::upsert_meeting(&conn, &m.uid, &m.title, m.starts_at, m.ends_at, record_all)
            .map_err(|e| e.to_string())?;
    }
    let cutoff = now_ms() - 3_600_000; // mantém até 1h após o fim
    storage::prune_meetings(&conn, cutoff).map_err(|e| e.to_string())?;
    storage::list_meetings(&conn, cutoff).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_meetings(app: AppHandle) -> Result<Vec<MeetingRow>, String> {
    let conn = open_db(&app)?;
    storage::list_meetings(&conn, now_ms() - 3_600_000).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_meeting_record(app: AppHandle, uid: String, enabled: bool) -> Result<(), String> {
    let conn = open_db(&app)?;
    storage::set_meeting_record(&conn, &uid, enabled).map_err(|e| e.to_string())
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

fn load_summary_config(conn: &rusqlite::Connection) -> anyhow::Result<SummaryConfig> {
    let d = SummaryConfig::default();
    Ok(SummaryConfig {
        endpoint_url: storage::get_setting(conn, "summary_endpoint_url")?.unwrap_or(d.endpoint_url),
        model: storage::get_setting(conn, "summary_model")?.unwrap_or(d.model),
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

fn fmt_timestamp(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}
