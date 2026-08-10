//! Auto-gravação a partir da agenda + alertas de fim de reunião.
//!
//! Roda numa thread própria enquanto o app está aberto:
//! - auto-INICIA quando uma reunião habilitada está em andamento (uma vez por reunião);
//! - alerta no horário de FIM previsto (recomenda parar — parada é manual);
//! - AUTO-STOP se passar 1h do fim previsto.
//!
//! **Um canal só de alerta**: a janela-toast. Antes cada evento disparava também
//! uma notificação nativa, o que avisava a mesma coisa duas vezes e ainda por
//! cima no canal sem botão — a notificação nativa do Windows não leva ação
//! confiável, que é justamente por que o toast existe.

use std::collections::HashSet;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Emitter, Manager};

use crate::audio::recorder::Recorder;
use crate::{commands, logs, meetings, storage};

/// Dispara o alerta/gravação a partir de 60s antes do início.
const LEAD_MS: i64 = 60_000;
/// Re-busca o ICS a cada N ticks (30s * 10 = 5 min).
const REFRESH_EVERY_TICKS: u32 = 10;

pub fn spawn(app: AppHandle) {
    thread::spawn(move || {
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
    let user_email = storage::get_setting(&conn, "attio_user_email")
        .ok()
        .flatten()
        .unwrap_or_default();
    let me = if user_email.trim().is_empty() {
        None
    } else {
        Some(user_email.trim())
    };
    // Falha de rede/servidor sai aqui, ANTES da reconciliação: apagar a agenda
    // por causa de um ICS que respondeu 500 seria pior que a reunião fantasma.
    let parsed = match meetings::fetch_and_parse(&ics, me) {
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
    // Reunião apagada no calendário sai daqui também no refresh automático.
    let uids: Vec<String> = parsed.iter().map(|m| m.uid.clone()).collect();
    if let Ok(n) = storage::prune_missing_meetings(&conn, &uids, cutoff) {
        if n > 0 {
            logs::log(
                app,
                "INFO",
                "agenda",
                &format!("{n} reunião(ões) removida(s): não estão mais no calendário"),
            );
        }
    }
    if let Ok(list) = storage::list_meetings(&conn, cutoff) {
        let _ = app.emit("meetings-refreshed", list);
    }
}

fn tick(app: &AppHandle, triggered: &mut HashSet<String>) {
    let now = now_ms();
    let recorder = app.state::<Recorder>();

    if recorder.is_recording() {
        // Lembrete de hora em hora: a gravação pode ter sido esquecida ligada.
        if let Some(hours) = recorder.should_alert_running() {
            logs::log(app, "INFO", "gravacao", &format!("lembrete: gravando há {hours}h"));
            show_toast(app, "recording", "", hours as i64, None);
        }
        if recorder.should_alert_end(now) {
            logs::log(app, "INFO", "agenda", "fim previsto da reunião");
            show_toast(app, "meeting-end", "", 0, None);
        }
        if recorder.should_auto_stop(now) {
            let _ = commands::stop_recording_core(app);
            show_toast(
                app,
                "stopped",
                "Passou 1h do fim da reunião — a gravação foi parada automaticamente.",
                0,
                None,
            );
            return;
        }
        // Auto-stop global por tempo (Configurações). 0 = desligado.
        let limit_min = commands::open_db(app)
            .ok()
            .and_then(|c| storage::get_setting(&c, "auto_stop_minutes").ok().flatten())
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120);
        if limit_min > 0 && recorder.elapsed_secs() >= limit_min * 60 {
            let _ = commands::stop_recording_core(app);
            let label = fmt_duration(limit_min);
            logs::log(app, "INFO", "gravacao", &format!("auto-stop por tempo: {label}"));
            show_toast(
                app,
                "stopped",
                &format!("Limite de {label} atingido — a gravação foi parada automaticamente."),
                0,
                None,
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
                    // Já está gravando: o toast serve para avisar e dar o
                    // atalho de entrar na call.
                    show_toast(app, "recording-started", &m.title, m.ends_at, m.link.as_deref());
                }
            } else {
                logs::log(app, "INFO", "agenda", &format!("alerta de reunião: {}", m.title));
                show_toast(app, "meeting", &m.title, m.ends_at, m.link.as_deref());
            }
            break;
        }
    }
}

/// Janela pequena no canto inferior direito — o **único** canal de alerta do
/// app. Notificação nativa com botão não é confiável no Windows, e é o botão
/// que importa aqui (gravar, parar, entrar na call).
///
/// A criação da janela/WebView precisa acontecer na thread principal — o
/// scheduler roda numa thread própria, então despacha via run_on_main_thread
/// (criar WebView2 fora da main thread falha com 0x80070057 / E_INVALIDARG).
///
/// `kind`:
/// - `meeting`           — reunião começando, sem auto-gravação
/// - `recording-started` — auto-gravação começou
/// - `recording`         — lembrete horário de gravação em andamento
/// - `meeting-end`       — passou do fim previsto e ainda está gravando
/// - `stopped`           — gravação encerrada automaticamente (só informa)
///
/// `value`: fim previsto (unix ms) nos alertas de reunião; horas gravadas em
/// `recording`. `link`: URL da call, quando a agenda tiver uma.
fn show_toast(app: &AppHandle, kind: &str, title: &str, value: i64, link: Option<&str>) {
    let app_main = app.clone();
    let kind = kind.to_string();
    let title = title.to_string();
    let link = link.unwrap_or_default().to_string();
    if let Err(e) =
        app.run_on_main_thread(move || build_toast(&app_main, &kind, &title, value, &link))
    {
        logs::log(app, "ERRO", "agenda", &format!("run_on_main_thread falhou: {e}"));
    }
}

fn build_toast(app: &AppHandle, kind: &str, title: &str, value: i64, link: &str) {
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
        "index.html?alert=1&kind={kind}&title={}&end={value}&link={}",
        urlencode(title),
        urlencode(link),
    );
    // Altura fixa: nenhum toast passa de dois botões, e o corpo de duas linhas
    // (motivo do auto-stop) cabe nesses 140px — os dois casos foram medidos.
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

/// "45min", "2h", "1h30" para o texto do aviso.
fn fmt_duration(minutes: u64) -> String {
    if minutes < 60 {
        format!("{minutes}min")
    } else if minutes % 60 == 0 {
        format!("{}h", minutes / 60)
    } else {
        format!("{}h{}", minutes / 60, minutes % 60)
    }
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}
