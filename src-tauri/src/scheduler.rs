//! Auto-gravação a partir da agenda + alertas de fim de reunião.
//!
//! Roda numa thread própria enquanto o app está aberto:
//! - auto-INICIA quando uma reunião habilitada está em andamento (uma vez por reunião);
//! - alerta no horário de FIM previsto (recomenda parar — parada é manual);
//! - AUTO-STOP se passar 1h do fim previsto.

use std::collections::HashSet;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::audio::recorder::Recorder;
use crate::{commands, storage};

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        let mut triggered: HashSet<String> = HashSet::new();
        loop {
            thread::sleep(Duration::from_secs(30));
            tick(&app, &mut triggered);
        }
    });
}

fn tick(app: &AppHandle, triggered: &mut HashSet<String>) {
    let now = now_ms();
    let recorder = app.state::<Recorder>();

    if recorder.is_recording() {
        if recorder.should_alert_end(now) {
            notify(
                app,
                "Reunião terminou",
                "A reunião marcada chegou ao fim. Recomendado parar a gravação.",
            );
        }
        if recorder.should_auto_stop(now) {
            let _ = commands::stop_recording_core(app);
            notify(
                app,
                "Gravação encerrada",
                "Passou 1h do fim da reunião — a gravação foi parada automaticamente.",
            );
        }
        return;
    }

    // Não está gravando: procura reunião em andamento ainda não tratada.
    let conn = match commands::open_db(app) {
        Ok(c) => c,
        Err(_) => return,
    };
    // "Gravar todas" liga a gravação automática para qualquer reunião.
    let record_all = storage::get_setting(&conn, "record_all")
        .ok()
        .flatten()
        .map(|v| v == "1")
        .unwrap_or(false);
    let meetings = storage::list_meetings(&conn, now - 3_600_000).unwrap_or_default();
    for m in meetings {
        if now >= m.starts_at && now < m.ends_at && !triggered.contains(&m.uid) {
            triggered.insert(m.uid.clone());
            if record_all || m.record_enabled {
                if commands::start_recording_for_meeting_core(app, m.ends_at).is_ok() {
                    notify(
                        app,
                        "Gravação iniciada automaticamente",
                        &format!("Reunião \"{}\" começou — gravação automática habilitada.", m.title),
                    );
                }
            } else {
                // Sem auto-gravação: notifica e abre um toast com botão de iniciar.
                notify(
                    app,
                    "Reunião começando",
                    &format!("\"{}\" está começando. Abra o Hicorder para gravar.", m.title),
                );
                show_meeting_toast(app, &m.title, m.ends_at);
            }
            break;
        }
    }
}

/// Janela pequena no canto inferior direito com botão "Iniciar gravação".
/// (Notificação nativa com botão não é confiável no Windows; janela própria é.)
fn show_meeting_toast(app: &AppHandle, title: &str, end_ms: i64) {
    // Uma por vez: fecha a anterior se ainda estiver aberta.
    if let Some(w) = app.get_webview_window("meeting-alert") {
        let _ = w.close();
    }
    let url = format!(
        "index.html?alert=1&title={}&end={}",
        urlencode(title),
        end_ms
    );
    let (w, h) = (380.0, 140.0);
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "meeting-alert",
        tauri::WebviewUrl::App(url.into()),
    )
    .title("Hicorder")
    .inner_size(w, h)
    .resizable(false)
    .decorations(false)
    .always_on_top(true)
    .skip_taskbar(true);

    // Posiciona no canto inferior direito do monitor principal.
    if let Ok(Some(mon)) = app.primary_monitor() {
        let size = mon.size();
        let scale = mon.scale_factor();
        let x = size.width as f64 / scale - w - 16.0;
        let y = size.height as f64 / scale - h - 64.0;
        builder = builder.position(x, y);
    }
    let _ = builder.build();
}

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn notify(app: &AppHandle, title: &str, body: &str) {
    let _ = app.notification().builder().title(title).body(body).show();
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
