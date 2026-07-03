mod attio;
mod audio;
mod commands;
mod encode;
mod logs;
mod meetings;
mod migrate;
mod net;
mod scheduler;
mod settings;
mod storage;
mod summary;
mod transcription;
mod tray;

use audio::recorder::Recorder;
use tauri::{Listener, Manager, WindowEvent};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .manage(Recorder::new())
        .setup(|app| {
            migrate::run(app.handle());
            tray::build_tray(app.handle())?;
            scheduler::spawn(app.handle().clone());

            // Atualiza a agenda no boot, se o ICS já estiver configurado.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                if let Ok(list) = commands::refresh_meetings(handle.clone()).await {
                    use tauri::Emitter;
                    let _ = handle.emit("meetings-refreshed", list);
                }
            });

            // Mantém o tray em sincronia com o estado de gravação.
            let handle = app.handle().clone();
            app.listen("recording-changed", move |_| tray::update_tray(&handle));

            // Fechar a janela minimiza pro tray (app segue rodando p/ auto-gravar).
            if let Some(win) = app.get_webview_window("main") {
                let w = win.clone();
                win.on_window_event(move |event| {
                    if let WindowEvent::CloseRequested { api, .. } = event {
                        api.prevent_close();
                        let _ = w.hide();
                    }
                });
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_input_devices,
            commands::start_recording,
            commands::stop_recording,
            commands::start_meeting_recording,
            commands::list_recordings,
            commands::delete_recording,
            commands::recording_level,
            commands::recording_status,
            commands::is_recording,
            commands::get_settings,
            commands::save_settings,
            commands::set_api_key,
            commands::set_summary_key,
            commands::transcribe,
            commands::get_transcript,
            commands::generate_summary,
            commands::get_summary,
            commands::refresh_meetings,
            commands::list_meetings,
            commands::set_meeting_record,
            commands::set_attio_key,
            commands::test_transcription_api,
            commands::test_summary_api,
            commands::test_attio_api,
            commands::get_logs,
            commands::clear_logs,
            commands::attio_find_meetings,
            commands::attio_upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
