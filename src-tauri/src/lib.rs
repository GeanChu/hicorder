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
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec!["--minimized"]),
        ))
        .manage(Recorder::new())
        .setup(|app| {
            migrate::run(app.handle());
            // Autoinicialização junto ao SO: ligada por padrão na 1ª execução.
            commands::ensure_autostart_default(app.handle());
            tray::build_tray(app.handle())?;
            scheduler::spawn(app.handle().clone());

            // Atualiza a agenda no boot, se habilitado e o ICS configurado.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let auto = commands::get_settings(handle.clone())
                    .map(|s| s.auto_sync_agenda)
                    .unwrap_or(true);
                if auto {
                    if let Ok(list) = commands::refresh_meetings(handle.clone()).await {
                        use tauri::Emitter;
                        let _ = handle.emit("meetings-refreshed", list);
                    }
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
                // Autoinicialização passa --minimized: começa escondido no tray.
                if std::env::args().any(|a| a == "--minimized") {
                    let _ = win.hide();
                }
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
            commands::rename_recording,
            commands::export_audio,
            commands::prepare_playback,
            commands::import_audio,
            commands::recording_level,
            commands::recording_status,
            commands::is_recording,
            commands::current_recording_id,
            commands::save_notes,
            commands::get_notes,
            commands::set_summary,
            commands::get_settings,
            commands::save_settings,
            commands::set_api_key,
            commands::set_summary_key,
            commands::transcribe,
            commands::get_transcript,
            commands::generate_summary,
            commands::default_summary_prompt,
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
            commands::log_client,
            commands::get_autostart,
            commands::set_autostart,
            commands::attio_find_meetings,
            commands::attio_upload,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
