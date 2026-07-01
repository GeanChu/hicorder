//! Ícone na bandeja do sistema: indica gravação + start/stop + abrir janela.

use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::audio::recorder::Recorder;
use crate::commands;

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Iniciar / parar gravação", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Abrir Call Recorder", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &open, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Call Recorder")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "toggle" => {
                let recording = app.state::<Recorder>().is_recording();
                let result = if recording {
                    commands::stop_recording_core(app).map(|_| ())
                } else {
                    commands::start_recording_core(app).map(|_| ())
                };
                if let Err(e) = result {
                    eprintln!("[tray] toggle falhou: {e}");
                }
                update_tray(app);
            }
            "open" => show_main(app),
            "quit" => app.exit(0),
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                show_main(tray.app_handle());
            }
        })
        .build(app)?;
    Ok(())
}

/// Atualiza o tooltip do tray conforme o estado de gravação.
pub fn update_tray(app: &AppHandle) {
    let recording = app.state::<Recorder>().is_recording();
    if let Some(tray) = app.tray_by_id("main") {
        let tip = if recording {
            "Call Recorder — GRAVANDO"
        } else {
            "Call Recorder"
        };
        let _ = tray.set_tooltip(Some(tip));
    }
}

fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
