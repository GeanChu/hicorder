//! Ícone na bandeja do sistema: indica gravação + start/stop + abrir janela.

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager};

use crate::audio::recorder::Recorder;
use crate::commands;

pub fn build_tray(app: &AppHandle) -> tauri::Result<()> {
    let toggle = MenuItem::with_id(app, "toggle", "Iniciar / parar gravação", true, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "Abrir Hicorder", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&toggle, &open, &quit])?;

    TrayIconBuilder::with_id("main")
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("Hicorder")
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

/// Atualiza tooltip e ícone do tray conforme o estado de gravação.
pub fn update_tray(app: &AppHandle) {
    let recording = app.state::<Recorder>().is_recording();
    if let Some(tray) = app.tray_by_id("main") {
        let _ = tray.set_tooltip(Some(if recording {
            "Hicorder — GRAVANDO"
        } else {
            "Hicorder"
        }));
        if let Some(base) = app.default_window_icon() {
            let _ = tray.set_icon(Some(tray_icon(base, recording)));
        }
    }
}

/// Ícone do tray; com uma bola vermelha no canto quando gravando.
fn tray_icon(base: &Image, recording: bool) -> Image<'static> {
    let (w, h) = (base.width(), base.height());
    let mut rgba = base.rgba().to_vec();
    if recording {
        let r = (w.min(h) as i32) / 3;
        let cx = w as i32 - r - 1;
        let cy = h as i32 - r - 1;
        for y in 0..h as i32 {
            for x in 0..w as i32 {
                let dx = x - cx;
                let dy = y - cy;
                if dx * dx + dy * dy <= r * r {
                    let idx = ((y as usize * w as usize) + x as usize) * 4;
                    rgba[idx] = 230; // R
                    rgba[idx + 1] = 30; // G
                    rgba[idx + 2] = 30; // B
                    rgba[idx + 3] = 255; // A
                }
            }
        }
    }
    Image::new_owned(rgba, w, h)
}

fn show_main(app: &AppHandle) {
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.show();
        let _ = win.unminimize();
        let _ = win.set_focus();
    }
}
