//! Comandos Tauri (IPC) expostos à UI.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, State};

use crate::audio::recorder::{Recorder, RecordingInfo, RecordingResult};

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<String>, String> {
    crate::audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_recording(app: AppHandle, recorder: State<Recorder>) -> Result<RecordingInfo, String> {
    let dir = recordings_dir(&app).map_err(|e| e.to_string())?;
    recorder.start(dir, new_id()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn stop_recording(recorder: State<Recorder>) -> Result<RecordingResult, String> {
    recorder.stop().map_err(|e| e.to_string())
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

fn new_id() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("rec-{ms}")
}
