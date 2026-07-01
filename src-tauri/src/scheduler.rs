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

    // Não está gravando: procura uma reunião habilitada em andamento, ainda não disparada.
    let conn = match commands::open_db(app) {
        Ok(c) => c,
        Err(_) => return,
    };
    let meetings = storage::list_meetings(&conn, now - 3_600_000).unwrap_or_default();
    for m in meetings {
        if m.record_enabled && now >= m.starts_at && now < m.ends_at && !triggered.contains(&m.uid)
        {
            triggered.insert(m.uid.clone());
            if commands::start_recording_for_meeting_core(app, m.ends_at).is_ok() {
                notify(app, "Gravação iniciada", &format!("Reunião \"{}\" começou.", m.title));
            }
            break;
        }
    }
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
