//! Auto-gravação a partir da agenda + alertas de fim de reunião.
//!
//! Roda numa thread própria enquanto o app está aberto:
//! - auto-INICIA quando uma reunião habilitada está em andamento (uma vez por reunião);
//! - alerta no horário de FIM previsto (recomenda parar — parada é manual);
//! - AUTO-STOP se passar 1h do fim previsto.

use std::collections::HashSet;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_notification::NotificationExt;

use crate::audio::recorder::Recorder;
use crate::{commands, logs, meetings, storage};

/// Dispara o alerta/gravação a partir de 60s antes do início.
const LEAD_MS: i64 = 60_000;
/// Re-busca o ICS a cada N ticks (30s * 10 = 5 min).
const REFRESH_EVERY_TICKS: u32 = 10;

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
        // Pede permissão de notificação uma vez (main thread — exigência macOS).
        {
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = handle.notification().request_permission();
            });
        }
        let mut triggered: HashSet<String> = HashSet::new();
        let mut ticks: u32 = 0;
        loop {
            thread::sleep(Duration::from_secs(30));
            if ticks % REFRESH_EVERY_TICKS == 0 {
                refresh_ics(&app);
            }
            ticks = ticks.wrapping_add(1);
            tick(&app, &mut triggered);
        }
    });
}

/// Re-busca a agenda do ICS (se auto-sync ligado e URL configurada) para que
/// reuniões novas apareçam sem o usuário clicar em Atualizar.
fn refresh_ics(app: &AppHandle) {
    let Ok(conn) = commands::open_db(app) else {
        return;
    };
    let auto = storage::get_setting(&conn, "auto_sync_agenda")
        .ok()
        .flatten()
        .map(|v| v != "0")
        .unwrap_or(true);
    let ics = storage::get_setting(&conn, "ics_url").ok().flatten().unwrap_or_default();
    if !auto || ics.trim().is_empty() {
        return;
    }
    let record_all = storage::get_setting(&conn, "record_all")
        .ok()
        .flatten()
        .map(|v| v == "1")
        .unwrap_or(false);
    let parsed = match meetings::fetch_and_parse(&ics) {
        Ok(p) => p,
        Err(e) => {
            logs::log(app, "INFO", "agenda", &format!("refresh automático falhou: {e}"));
            return;
        }
    };
    for m in &parsed {
        let _ = storage::upsert_meeting(
            &conn,
            &m.uid,
            &m.title,
            m.starts_at,
            m.ends_at,
            record_all,
            &m.participants,
            m.location.as_deref(),
            m.link.as_deref(),
        );
    }
    let cutoff = now_ms() - 3_600_000;
    let _ = storage::prune_meetings(&conn, cutoff);
    if let Ok(list) = storage::list_meetings(&conn, cutoff) {
        let _ = app.emit("meetings-refreshed", list);
    }
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
        // Dispara a partir de 60s antes do início até o fim previsto.
        if now >= m.starts_at - LEAD_MS && now < m.ends_at && !triggered.contains(&m.uid) {
            triggered.insert(m.uid.clone());
            if record_all || m.record_enabled {
                if commands::start_recording_for_meeting_core(app, m.ends_at, &m.title).is_ok() {
                    logs::log(app, "INFO", "agenda", &format!("auto-gravação: {}", m.title));
                    notify(
                        app,
                        "Gravação iniciada automaticamente",
                        &format!("Reunião \"{}\" começou — gravação automática habilitada.", m.title),
                    );
                }
            } else {
                // Sem auto-gravação: notifica e abre a janela-toast com botão.
                logs::log(app, "INFO", "agenda", &format!("alerta de reunião: {}", m.title));
                notify(
                    app,
                    "Reunião começando",
                    &format!("\"{}\" está começando. Clique para gravar.", m.title),
                );
                show_meeting_toast(app, &m.title, m.ends_at);
            }
            break;
        }
    }
}

/// Janela pequena no canto inferior direito com botão "Iniciar gravação".
/// (Notificação nativa com botão não é confiável no Windows; janela própria é.)
///
/// A criação da janela/WebView precisa acontecer na thread principal — o
/// scheduler roda numa thread própria, então despacha via run_on_main_thread
/// (criar WebView2 fora da main thread falha com 0x80070057 / E_INVALIDARG).
fn show_meeting_toast(app: &AppHandle, title: &str, end_ms: i64) {
    let app_main = app.clone();
    let title = title.to_string();
    if let Err(e) = app.run_on_main_thread(move || build_toast(&app_main, &title, end_ms)) {
        logs::log(app, "ERRO", "agenda", &format!("run_on_main_thread falhou: {e}"));
    }
}

fn build_toast(app: &AppHandle, title: &str, end_ms: i64) {
    // Uma por vez: destrói toasts anteriores ainda abertos. `close()` é
    // assíncrono e o label continuava vivo, colidindo na criação seguinte
    // ("a webview with label `meeting-alert` already exists"); destroy() é
    // imediato e o label com timestamp garante unicidade mesmo assim.
    for (label, w) in app.webview_windows() {
        if label.starts_with("meeting-alert") {
            let _ = w.destroy();
        }
    }
    let url = format!(
        "index.html?alert=1&title={}&end={}",
        urlencode(title),
        end_ms
    );
    let (w, h) = (380.0, 140.0);
    let label = format!("meeting-alert-{}", now_ms());
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        label,
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
    match builder.build() {
        Err(e) => logs::log(app, "ERRO", "agenda", &format!("falha ao abrir a janela-toast: {e}")),
        Ok(win) => {
            // Rede de segurança: o toast não tem decoração nem taskbar, então
            // se o webview travar o usuário não consegue fechá-lo. Destrói
            // após 90s caso ainda exista (o normal é fechar antes via botão).
            let label = win.label().to_string();
            let app2 = app.clone();
            thread::spawn(move || {
                thread::sleep(Duration::from_secs(90));
                if let Some(w) = app2.get_webview_window(&label) {
                    let _ = w.destroy();
                }
            });
        }
    }
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

/// Envia uma notificação nativa. Despacha para a thread principal — no macOS
/// o centro de notificações exige main thread (o scheduler roda em outra).
fn notify(app: &AppHandle, title: &str, body: &str) {
    let handle = app.clone();
    let title = title.to_string();
    let body = body.to_string();
    let _ = app.run_on_main_thread(move || {
        let _ = handle.notification().builder().title(title).body(body).show();
    });
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
