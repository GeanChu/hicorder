mod audio;
mod commands;
mod encode;
mod settings;
mod storage;
mod transcription;

use audio::recorder::Recorder;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(Recorder::new())
        .invoke_handler(tauri::generate_handler![
            commands::list_input_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::recording_level,
            commands::is_recording,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
