//! Comandos Tauri (IPC) expostos à UI.

use std::cmp::Ordering;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State};

use crate::attio::{self, AttioCompany, AttioMeeting};
use crate::audio::recorder::{Recorder, RecordingInfo};
use crate::storage::{self, MeetingRow, RecordingRow, SummaryRow, TranscriptRow};
use crate::summary::{self, SummaryConfig};
use crate::transcription::{self, OpenAiCompatible, Transcriber, TranscriptionConfig};
use crate::{audio, encode, logs, meetings, settings};

#[derive(Serialize, Clone)]
pub struct AppSettings {
    pub default_language: String,
    pub endpoint_url: String,
    pub model: String,
    pub has_api_key: bool,
    // Resumo (MiniMax-M3) — opcional.
    pub summary_endpoint_url: String,
    pub summary_model: String,
    pub has_summary_key: bool,
    /// Prompt base usado em todos os resumos (editável nas Configurações).
    pub summary_prompt: String,
    // Calendário (ICS).
    pub ics_url: String,
    pub record_all: bool,
    // Attio (CRM).
    pub has_attio_key: bool,
    /// Email do usuário no Attio — filtra reuniões sugeridas às que ele participa.
    pub attio_user_email: String,
    /// Tema da UI: "system" | "light" | "dark".
    pub theme: String,
    /// Sincronizar a agenda automaticamente ao abrir o app.
    pub auto_sync_agenda: bool,
}

#[derive(Serialize, Clone)]
pub struct AttioUploadResult {
    pub meeting_id: String,
    pub notes_created: usize,
    pub missing_people: Vec<String>,
}

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<String>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_recording(app: AppHandle) -> Result<RecordingInfo, String> {
    logged(&app, "gravacao", start_recording_core(&app))
}

/// Para a gravação, mistura/encoda para Opus, persiste e retorna a linha.
/// O encode do ffmpeg pode demorar em reuniões longas, então roda numa thread
/// de blocking — se rodasse direto no comando (thread principal), a UI travaria
/// no "Processando".
#[tauri::command]
pub async fn stop_recording(app: AppHandle) -> Result<RecordingRow, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || logged(&app2, "gravacao", stop_recording_core(&app2)))
        .await
        .map_err(|e| e.to_string())?
}

/// Núcleo de iniciar — chamável pelo command, pelo tray e pelo scheduler.
pub fn start_recording_core(app: &AppHandle) -> Result<RecordingInfo, String> {
    let dir = recordings_dir(app).map_err(|e| e.to_string())?;
    let ffmpeg = resolve_ffmpeg(app);
    let info = app
        .state::<Recorder>()
        .start(ffmpeg, dir, new_id(), None, "Gravação manual".to_string())
        .map_err(|e| e.to_string())?;
    let _ = app.emit("recording-changed", true);
    Ok(info)
}

/// Inicia gravação vinculada a uma reunião (guarda o fim previsto p/ alerta/auto-stop).
pub fn start_recording_for_meeting_core(
    app: &AppHandle,
    meeting_end_ms: i64,
    title: &str,
) -> Result<RecordingInfo, String> {
    let dir = recordings_dir(app).map_err(|e| e.to_string())?;
    let title = if title.trim().is_empty() {
        "Reunião".to_string()
    } else {
        title.to_string()
    };
    let ffmpeg = resolve_ffmpeg(app);
    let info = app
        .state::<Recorder>()
        .start(ffmpeg, dir, new_id(), Some(meeting_end_ms), title)
        .map_err(|e| e.to_string())?;
    let _ = app.emit("recording-changed", true);
    Ok(info)
}

/// Inicia gravação de uma reunião a partir do toast de alerta.
#[tauri::command]
pub fn start_meeting_recording(
    app: AppHandle,
    end_ms: i64,
    title: String,
) -> Result<RecordingInfo, String> {
    logged(&app, "gravacao", start_recording_for_meeting_core(&app, end_ms, &title))
}

pub fn stop_recording_core(app: &AppHandle) -> Result<RecordingRow, String> {
    // As faixas já foram encodadas ao vivo para Opus/Ogg (mic.ogg / system.ogg).
    // Parar só fecha os pipes do ffmpeg — sem encode aqui, é quase instantâneo.
    // A faixa mixada para o player é gerada sob demanda no primeiro Play/Exportar.
    let res = app.state::<Recorder>().stop().map_err(|e| e.to_string())?;

    let mut size_bytes = std::fs::metadata(&res.mic_path).map(|m| m.len()).unwrap_or(0);
    if let Some(sp) = &res.system_path {
        size_bytes += std::fs::metadata(sp).map(|m| m.len()).unwrap_or(0);
    }

    let row = RecordingRow {
        id: res.id,
        title: res.title,
        path: res.mic_path, // mic.ogg = "Você"
        system_path: res.system_path, // system.ogg = "Participantes"
        created_at: now_ms(),
        duration_s: res.duration_s,
        size_bytes: size_bytes as i64,
    };

    let conn = open_db(app)?;
    storage::insert(&conn, &row).map_err(|e| e.to_string())?;
    let _ = app.emit("recording-changed", false);
    Ok(row)
}

/// Caminho do arquivo para reprodução/exportação: a faixa mixada (mic+sistema).
/// Gera `recording.<ext>` sob demanda (uma vez) e cacheia. Se não houver faixa
/// do sistema, usa a própria faixa do mic. Compatível com gravações antigas
/// (mic.webm + recording.webm já existente).
fn ensure_mixed(app: &AppHandle, recording_id: &str) -> Result<PathBuf, String> {
    let conn = open_db(app)?;
    let (mic, system) = storage::recording_paths(&conn, recording_id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "gravação não encontrada".to_string())?;
    let mic_path = PathBuf::from(&mic);

    // Sem faixa do sistema: reproduz o próprio mic.
    let Some(system) = system else {
        return Ok(mic_path);
    };

    let ext = mic_path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("ogg");
    let mixed = mic_path.with_file_name(format!("recording.{ext}"));
    if mixed.exists() {
        return Ok(mixed);
    }
    let ffmpeg = resolve_ffmpeg(app);
    encode::mix_to_opus(&ffmpeg, &mic, Some(&system), &mixed)
        .map_err(|e| fail(app, "gravacao", e.to_string()))?;
    Ok(mixed)
}

/// Prepara o arquivo de reprodução e devolve o caminho absoluto (a UI converte
/// com convertFileSrc). Mixa mic+sistema no primeiro uso.
#[tauri::command]
pub async fn prepare_playback(app: AppHandle, recording_id: String) -> Result<String, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        ensure_mixed(&app2, &recording_id).map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Exporta o áudio da gravação para `dest_path` (formato pela extensão).
#[tauri::command]
pub async fn export_audio(
    app: AppHandle,
    recording_id: String,
    dest_path: String,
) -> Result<(), String> {
    // Fonte = faixa mixada (mic + sistema), gerada sob demanda se preciso.
    let src = ensure_mixed(&app, &recording_id)?;
    let ffmpeg = resolve_ffmpeg(&app);
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        encode::transcode(&ffmpeg, &src, Path::new(&dest_path))
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| fail(&app2, "export", e.to_string()))
}

/// Importa um arquivo de áudio do usuário como uma gravação, convertendo para
/// o formato padrão (Opus/Ogg, faixa única). Vira "Upload Manual — <data/hora>".
#[tauri::command]
pub async fn import_audio(app: AppHandle, src_path: String) -> Result<RecordingRow, String> {
    let app2 = app.clone();
    tauri::async_runtime::spawn_blocking(move || logged(&app2, "import", import_audio_core(&app2, &src_path)))
        .await
        .map_err(|e| e.to_string())?
}

fn import_audio_core(app: &AppHandle, src: &str) -> Result<RecordingRow, String> {
    if !Path::new(src).exists() {
        return Err("arquivo não encontrado".to_string());
    }
    let id = new_id();
    let dir = recordings_dir(app).map_err(|e| e.to_string())?.join(&id);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let ffmpeg = resolve_ffmpeg(app);
    let mic = dir.join("mic.ogg");
    // Faixa única no formato padrão (mono 16k Opus). Sem faixa de sistema.
    encode::mix_to_opus(&ffmpeg, src, None, &mic)
        .map_err(|e| fail(app, "import", e.to_string()))?;

    let duration_s = encode::probe_duration(&ffmpeg, src).unwrap_or(0.0);
    let size_bytes = std::fs::metadata(&mic).map(|m| m.len()).unwrap_or(0) as i64;
    let when = chrono::Local::now().format("%d/%m/%Y %H:%M");

    let row = RecordingRow {
        id,
        title: format!("Upload Manual — {when}"),
        path: mic.to_string_lossy().into_owned(),
        system_path: None,
        created_at: now_ms(),
        duration_s,
        size_bytes,
    };
    let conn = open_db(app)?;
    storage::insert(&conn, &row).map_err(|e| e.to_string())?;
    let _ = app.emit("recording-changed", false);
    Ok(row)
}

/// Renomeia uma gravação.
#[tauri::command]
pub fn rename_recording(app: AppHandle, recording_id: String, title: String) -> Result<(), String> {
    let t = title.trim();
    if t.is_empty() {
        return Err("o nome não pode ser vazio".to_string());
    }
    let r = open_db(&app)
        .and_then(|conn| storage::rename_recording(&conn, &recording_id, t).map_err(|e| e.to_string()));
    logged(&app, "gravacao", r)
}

#[tauri::command]
pub fn list_recordings(app: AppHandle) -> Result<Vec<RecordingRow>, String> {
    let r = open_db(&app).and_then(|conn| storage::list(&conn).map_err(|e| e.to_string()));
    logged(&app, "gravacao", r)
}

#[tauri::command]
pub fn get_settings(app: AppHandle) -> Result<AppSettings, String> {
    let conn = open_db(&app)?;
    let cfg = load_config(&conn).map_err(|e| e.to_string())?;
    let default_language = storage::get_setting(&conn, "default_language")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "pt".to_string());
    let scfg = load_summary_config(&conn).map_err(|e| e.to_string())?;
    let summary_prompt = storage::get_setting(&conn, "summary_prompt")
        .map_err(|e| e.to_string())?
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| summary::default_prompt().to_string());
    let ics_url = storage::get_setting(&conn, "ics_url")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let record_all = storage::get_setting(&conn, "record_all")
        .map_err(|e| e.to_string())?
        .map(|v| v == "1")
        .unwrap_or(false);
    let attio_user_email = storage::get_setting(&conn, "attio_user_email")
        .map_err(|e| e.to_string())?
        .unwrap_or_default();
    let theme = storage::get_setting(&conn, "theme")
        .map_err(|e| e.to_string())?
        .unwrap_or_else(|| "system".to_string());
    // Default habilitado: só desliga se estiver explicitamente "0".
    let auto_sync_agenda = storage::get_setting(&conn, "auto_sync_agenda")
        .map_err(|e| e.to_string())?
        .map(|v| v != "0")
        .unwrap_or(true);
    Ok(AppSettings {
        default_language,
        endpoint_url: cfg.endpoint_url,
        model: cfg.model,
        has_api_key: settings::has_api_key(),
        summary_endpoint_url: scfg.endpoint_url,
        summary_model: scfg.model,
        has_summary_key: settings::has_summary_key(),
        summary_prompt,
        ics_url,
        record_all,
        has_attio_key: settings::has_attio_key(),
        attio_user_email,
        theme,
        auto_sync_agenda,
    })
}

#[tauri::command]
pub fn save_settings(
    app: AppHandle,
    default_language: String,
    endpoint_url: String,
    model: String,
    summary_endpoint_url: String,
    summary_model: String,
    summary_prompt: String,
    ics_url: String,
    record_all: bool,
    attio_user_email: String,
    theme: String,
    auto_sync_agenda: bool,
) -> Result<(), String> {
    let conn = open_db(&app)?;
    storage::set_setting(&conn, "default_language", &default_language).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "endpoint_url", &endpoint_url).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "model", &model).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "summary_endpoint_url", &summary_endpoint_url)
        .map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "summary_model", &summary_model).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "summary_prompt", summary_prompt.trim()).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "ics_url", &ics_url).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "record_all", if record_all { "1" } else { "0" })
        .map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "attio_user_email", attio_user_email.trim())
        .map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "theme", &theme).map_err(|e| e.to_string())?;
    storage::set_setting(&conn, "auto_sync_agenda", if auto_sync_agenda { "1" } else { "0" })
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn set_api_key(key: String) -> Result<(), String> {
    settings::set_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_summary_key(key: String) -> Result<(), String> {
    settings::set_summary_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_attio_key(key: String) -> Result<(), String> {
    settings::set_attio_key(&key).map_err(|e| e.to_string())
}

/// Registra o erro cru no log e devolve uma mensagem amigável ao usuário.
fn fail(app: &AppHandle, category: &str, raw: String) -> String {
    logs::log(app, "ERRO", category, &raw);
    logs::humanize(&raw)
}

/// Loga o erro (se houver) e devolve o Result inalterado. Usado no boundary
/// dos comandos que já têm mensagem clara (gravação, agenda) para que tudo
/// caia no callrec.log, não só os caminhos de IA/CRM.
fn logged<T>(app: &AppHandle, category: &str, r: Result<T, String>) -> Result<T, String> {
    if let Err(e) = &r {
        logs::log(app, "ERRO", category, e);
    }
    r
}

/// Testa a chave/endpoint da transcrição. `key` opcional (usa o keychain se vazio).
#[tauri::command]
pub async fn test_transcription_api(
    app: AppHandle,
    endpoint_url: String,
    key: Option<String>,
) -> Result<String, String> {
    let api_key = match key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k,
        None => settings::get_api_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Nenhuma chave de transcrição configurada.".to_string())?,
    };
    tauri::async_runtime::spawn_blocking(move || transcription::test_key(&endpoint_url, &api_key))
        .await
        .map_err(|e| e.to_string())?
        .map(|_| "Transcrição: conexão e chave OK.".to_string())
        .map_err(|e| fail(&app, "transcricao", e.to_string()))
}

/// Testa a chave/endpoint/modelo do resumo. `key` opcional (usa o keychain se vazio).
#[tauri::command]
pub async fn test_summary_api(
    app: AppHandle,
    endpoint_url: String,
    model: String,
    key: Option<String>,
) -> Result<String, String> {
    let api_key = match key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k,
        None => settings::get_summary_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Nenhuma chave de resumo configurada.".to_string())?,
    };
    let cfg = SummaryConfig { endpoint_url, model };
    tauri::async_runtime::spawn_blocking(move || summary::test_key(&cfg, &api_key))
        .await
        .map_err(|e| e.to_string())?
        .map(|_| "Resumo: conexão, chave e modelo OK.".to_string())
        .map_err(|e| fail(&app, "resumo", e.to_string()))
}

/// Testa a chave do Attio. `key` opcional (usa o keychain se vazio).
#[tauri::command]
pub async fn test_attio_api(app: AppHandle, key: Option<String>) -> Result<String, String> {
    let api_key = match key.filter(|k| !k.trim().is_empty()) {
        Some(k) => k,
        None => settings::get_attio_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "Nenhuma chave do Attio configurada.".to_string())?,
    };
    tauri::async_runtime::spawn_blocking(move || attio::test_key(&api_key))
        .await
        .map_err(|e| e.to_string())?
        .map(|_| "Attio: conexão e chave OK.".to_string())
        .map_err(|e| fail(&app, "attio", e.to_string()))
}

/// Liga a autoinicialização por padrão na primeira execução (uma única vez).
pub fn ensure_autostart_default(app: &AppHandle) {
    use tauri_plugin_autostart::ManagerExt;
    let Ok(conn) = open_db(app) else {
        return;
    };
    let al = app.autolaunch();
    let already = storage::get_setting(&conn, "autostart_init")
        .ok()
        .flatten()
        .is_some();

    // Primeira execução (ou tentativa anterior que falhou): liga por padrão.
    // Só marca como inicializado quando o enable dá certo — assim uma falha
    // é re-tentada no próximo boot em vez de ficar travada.
    if !already {
        match al.enable() {
            Ok(()) => {
                let _ = storage::set_setting(&conn, "autostart_init", "1");
                logs::log(app, "INFO", "autostart", "habilitado por padrão");
            }
            Err(e) => logs::log(app, "ERRO", "autostart", &format!("falha ao habilitar: {e}")),
        }
    }

    // Diagnóstico: registra o estado real no log a cada boot.
    match al.is_enabled() {
        Ok(on) => logs::log(
            app,
            "INFO",
            "autostart",
            if on { "estado: ligado" } else { "estado: desligado" },
        ),
        Err(e) => logs::log(app, "INFO", "autostart", &format!("is_enabled erro: {e}")),
    }
}

/// Estado atual da autoinicialização com o SO.
#[tauri::command]
pub fn get_autostart(app: AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

/// Liga/desliga a autoinicialização com o SO.
#[tauri::command]
pub fn set_autostart(app: AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    let r = if enabled { al.enable() } else { al.disable() };
    r.map_err(|e| e.to_string())
}

/// Registra no log um erro reportado pela UI (ex.: falha do player, catch
/// de uma ação no frontend). Assim o callrec.log cobre também erros que só
/// acontecem no lado do webview.
#[tauri::command]
pub fn log_client(app: AppHandle, category: String, message: String) {
    let cat = if category.trim().is_empty() { "ui" } else { category.trim() };
    logs::log(&app, "ERRO", cat, &message);
}

/// Devolve o conteúdo do log persistente (para troubleshooting).
#[tauri::command]
pub fn get_logs(app: AppHandle) -> Result<String, String> {
    Ok(logs::read(&app))
}

/// Limpa o log persistente.
#[tauri::command]
pub fn clear_logs(app: AppHandle) -> Result<(), String> {
    logs::clear(&app);
    Ok(())
}

/// Lista meetings do Attio numa janela de tempo, casando emails no cliente.
#[tauri::command]
pub async fn attio_find_meetings(
    app: AppHandle,
    ends_from: String,
    starts_before: String,
    timezone: String,
    emails: Vec<String>,
) -> Result<Vec<AttioMeeting>, String> {
    let key = settings::get_attio_key()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "configure a chave do Attio nas Configurações".to_string())?;
    let user_email = {
        let conn = open_db(&app)?;
        storage::get_setting(&conn, "attio_user_email")
            .map_err(|e| e.to_string())?
            .unwrap_or_default()
    };
    tauri::async_runtime::spawn_blocking(move || {
        let ue = if user_email.trim().is_empty() {
            None
        } else {
            Some(user_email.trim())
        };
        attio::list_meetings(&key, &ends_from, &starts_before, &timezone, ue, &emails)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| fail(&app, "attio", e.to_string()))
}

/// Empresas vinculadas aos participantes (por email) para o usuário escolher
/// quais também recebem a nota.
#[tauri::command]
pub async fn attio_meeting_companies(
    app: AppHandle,
    emails: Vec<String>,
) -> Result<Vec<AttioCompany>, String> {
    let key = settings::get_attio_key()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "configure a chave do Attio nas Configurações".to_string())?;
    tauri::async_runtime::spawn_blocking(move || attio::companies_for_emails(&key, &emails))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| fail(&app, "attio", e.to_string()))
}

/// Sobe a transcrição ou o resumo como nota em cada participante, linkando a meeting.
/// Se `meeting_id` for None, faz find-or-create com título/horário/emails.
#[tauri::command]
pub async fn attio_upload(
    app: AppHandle,
    recording_id: String,
    kind: String,
    meeting_id: Option<String>,
    title: String,
    start_iso: String,
    end_iso: String,
    timezone: String,
    emails: Vec<String>,
    company_ids: Vec<String>,
) -> Result<AttioUploadResult, String> {
    let content = {
        let conn = open_db(&app)?;
        match kind.as_str() {
            "summary" => storage::get_summary(&conn, &recording_id)
                .map_err(|e| e.to_string())?
                .map(|s| s.text)
                .ok_or_else(|| "gere o resumo antes de subir".to_string())?,
            "notes" => storage::get_notes(&conn, &recording_id)
                .map_err(|e| e.to_string())?
                .filter(|n| !n.trim().is_empty())
                .ok_or_else(|| "escreva anotações antes de subir".to_string())?,
            _ => storage::get_transcript(&conn, &recording_id)
                .map_err(|e| e.to_string())?
                .map(|t| t.text)
                .ok_or_else(|| "transcreva antes de subir".to_string())?,
        }
    };
    let key = settings::get_attio_key()
        .map_err(|e| e.to_string())?
        .ok_or_else(|| "configure a chave do Attio nas Configurações".to_string())?;
    let kind_label = match kind.as_str() {
        "summary" => "Resumo",
        "notes" => "Anotações",
        _ => "Transcrição",
    };
    let note_title = format!("{title} — {kind_label} (Hicorder)");

    tauri::async_runtime::spawn_blocking(move || -> Result<AttioUploadResult, String> {
        let mid = match meeting_id {
            Some(m) => m,
            None => attio::find_or_create_meeting(
                &key, &title, &start_iso, &end_iso, &timezone, &emails,
            )
            .map_err(|e| e.to_string())?,
        };
        let mut notes_created = 0usize;
        let mut missing = Vec::new();
        for e in &emails {
            match attio::find_person_by_email(&key, e).map_err(|er| er.to_string())? {
                Some(pid) => {
                    attio::create_note(&key, "people", &pid, &mid, &note_title, &content)
                        .map_err(|er| er.to_string())?;
                    notes_created += 1;
                }
                None => missing.push(e.clone()),
            }
        }
        // Notas também nas empresas selecionadas (record_id já resolvido no cliente).
        for cid in &company_ids {
            attio::create_note(&key, "companies", cid, &mid, &note_title, &content)
                .map_err(|er| er.to_string())?;
            notes_created += 1;
        }
        Ok(AttioUploadResult {
            meeting_id: mid,
            notes_created,
            missing_people: missing,
        })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|raw| fail(&app, "attio", raw))
}

#[tauri::command]
pub async fn transcribe(
    app: AppHandle,
    recording_id: String,
    language: String,
) -> Result<TranscriptRow, String> {
    let lang = if language.trim().is_empty() {
        "pt".to_string()
    } else {
        language
    };

    // Leituras síncronas no SQLite, sem segurar a conexão durante o await.
    let (mic_path, system_path, provider) = {
        let conn = open_db(&app)?;
        let (mic, sys) = storage::recording_paths(&conn, &recording_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "gravação não encontrada".to_string())?;
        let cfg = load_config(&conn).map_err(|e| e.to_string())?;
        let api_key = settings::get_api_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "configure a chave da API nas Configurações".to_string())?;
        (
            mic,
            sys,
            OpenAiCompatible {
                endpoint_url: cfg.endpoint_url,
                model: cfg.model,
                api_key,
            },
        )
    };

    // "Você" = microfone. HTTP bloqueante em thread de blocking (não trava a UI).
    let p_mic = provider.clone();
    let mic_for_http = mic_path.clone();
    let lang_mic = lang.clone();
    let mic_segs = tauri::async_runtime::spawn_blocking(move || {
        p_mic.transcribe(Path::new(&mic_for_http), &lang_mic)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| fail(&app, "transcricao", e.to_string()))?;

    // "Participantes" = áudio do sistema (falha aqui não derruba a transcrição do mic).
    let sys_segs = if let Some(sp) = system_path {
        let p_sys = provider.clone();
        let lang_sys = lang.clone();
        match tauri::async_runtime::spawn_blocking(move || {
            p_sys.transcribe(Path::new(&sp), &lang_sys)
        })
        .await
        {
            Ok(Ok(segs)) => segs,
            Ok(Err(e)) => {
                logs::log(&app, "INFO", "transcricao", &format!("faixa do sistema falhou: {e}"));
                Vec::new()
            }
            Err(e) => {
                logs::log(&app, "INFO", "transcricao", &format!("faixa do sistema panic: {e}"));
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    // Intercala por timestamp e rotula.
    let mut tagged: Vec<(f64, &'static str, String)> = Vec::new();
    for s in mic_segs {
        tagged.push((s.start, "Você", s.text));
    }
    for s in sys_segs {
        tagged.push((s.start, "Participantes", s.text));
    }
    tagged.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(Ordering::Equal));

    let text = tagged
        .iter()
        .map(|(start, label, txt)| format!("[{}] {}: {}", fmt_timestamp(*start), label, txt))
        .collect::<Vec<_>>()
        .join("\n");

    if text.is_empty() {
        return Err("transcrição vazia".to_string());
    }

    let row = TranscriptRow {
        recording_id,
        language: lang,
        text,
        created_at: now_ms(),
    };
    let conn = open_db(&app)?;
    storage::upsert_transcript(&conn, &row).map_err(|e| e.to_string())?;
    Ok(row)
}

#[tauri::command]
pub fn get_transcript(app: AppHandle, recording_id: String) -> Result<Option<TranscriptRow>, String> {
    let r = open_db(&app)
        .and_then(|conn| storage::get_transcript(&conn, &recording_id).map_err(|e| e.to_string()));
    logged(&app, "transcricao", r)
}

#[tauri::command]
pub fn delete_recording(app: AppHandle, recording_id: String) -> Result<(), String> {
    let r = (|| {
        let conn = open_db(&app)?;
        let paths = storage::recording_paths(&conn, &recording_id).map_err(|e| e.to_string())?;
        storage::delete_recording(&conn, &recording_id).map_err(|e| e.to_string())?;
        // Apaga a pasta da gravação (mic.webm + system.webm).
        if let Some((mic, _sys)) = paths {
            if let Some(dir) = Path::new(&mic).parent() {
                let _ = std::fs::remove_dir_all(dir);
            }
        }
        Ok(())
    })();
    logged(&app, "gravacao", r)
}

/// Gera o resumo. `prompt` opcional sobrescreve o prompt base só nesta chamada
/// (edição do prompt de um resumo específico na aba Gravações). Vazio/None usa
/// o prompt base salvo nas Configurações (ou o padrão de fábrica).
#[tauri::command]
pub async fn generate_summary(
    app: AppHandle,
    recording_id: String,
    prompt: Option<String>,
) -> Result<SummaryRow, String> {
    let (transcript_text, notes, cfg, api_key, system_prompt) = {
        let conn = open_db(&app)?;
        let t = storage::get_transcript(&conn, &recording_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "transcreva a gravação antes de resumir".to_string())?;
        let notes = storage::get_notes(&conn, &recording_id).map_err(|e| e.to_string())?;
        let cfg = load_summary_config(&conn).map_err(|e| e.to_string())?;
        let api_key = settings::get_summary_key()
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "configure a chave do Resumo (MiniMax) nas Configurações".to_string())?;
        let system_prompt = prompt
            .map(|p| p.trim().to_string())
            .filter(|p| !p.is_empty())
            .or_else(|| {
                storage::get_setting(&conn, "summary_prompt")
                    .ok()
                    .flatten()
                    .filter(|s| !s.trim().is_empty())
            })
            .unwrap_or_else(|| summary::default_prompt().to_string());
        (t.text, notes, cfg, api_key, system_prompt)
    };

    let text = tauri::async_runtime::spawn_blocking(move || {
        summary::summarize(&cfg, &api_key, &transcript_text, notes.as_deref(), &system_prompt)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| fail(&app, "resumo", e.to_string()))?;

    let row = SummaryRow {
        recording_id,
        text,
        created_at: now_ms(),
    };
    let conn = open_db(&app)?;
    storage::upsert_summary(&conn, &row).map_err(|e| e.to_string())?;
    Ok(row)
}

/// Prompt base de fábrica do resumo (para "restaurar padrão" na UI).
#[tauri::command]
pub fn default_summary_prompt() -> String {
    summary::default_prompt().to_string()
}

#[tauri::command]
pub fn get_summary(app: AppHandle, recording_id: String) -> Result<Option<SummaryRow>, String> {
    let r = open_db(&app)
        .and_then(|conn| storage::get_summary(&conn, &recording_id).map_err(|e| e.to_string()));
    logged(&app, "resumo", r)
}

#[tauri::command]
pub async fn refresh_meetings(app: AppHandle) -> Result<Vec<MeetingRow>, String> {
    let (ics_url, record_all) = {
        let conn = open_db(&app)?;
        let url = storage::get_setting(&conn, "ics_url")
            .map_err(|e| e.to_string())?
            .unwrap_or_default();
        let ra = storage::get_setting(&conn, "record_all")
            .map_err(|e| e.to_string())?
            .map(|v| v == "1")
            .unwrap_or(false);
        (url, ra)
    };
    if ics_url.trim().is_empty() {
        // Config faltando, não é erro de execução — não vai pro log.
        return Err("configure a URL do calendário (ICS) nas Configurações".to_string());
    }

    let work = async {
        let parsed =
            tauri::async_runtime::spawn_blocking(move || meetings::fetch_and_parse(&ics_url))
                .await
                .map_err(|e| e.to_string())?
                .map_err(|e| e.to_string())?;

        let conn = open_db(&app)?;
        for m in &parsed {
            storage::upsert_meeting(
                &conn,
                &m.uid,
                &m.title,
                m.starts_at,
                m.ends_at,
                record_all,
                &m.participants,
                m.location.as_deref(),
                m.link.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        }
        let cutoff = now_ms() - 3_600_000; // mantém até 1h após o fim
        storage::prune_meetings(&conn, cutoff).map_err(|e| e.to_string())?;
        storage::list_meetings(&conn, cutoff).map_err(|e| e.to_string())
    }
    .await;
    logged(&app, "agenda", work)
}

#[tauri::command]
pub fn list_meetings(app: AppHandle) -> Result<Vec<MeetingRow>, String> {
    let r = open_db(&app)
        .and_then(|conn| storage::list_meetings(&conn, now_ms() - 3_600_000).map_err(|e| e.to_string()));
    logged(&app, "agenda", r)
}

#[tauri::command]
pub fn set_meeting_record(app: AppHandle, uid: String, enabled: bool) -> Result<(), String> {
    let r = open_db(&app)
        .and_then(|conn| storage::set_meeting_record(&conn, &uid, enabled).map_err(|e| e.to_string()));
    logged(&app, "agenda", r)
}

pub(crate) fn open_db(app: &AppHandle) -> Result<rusqlite::Connection, String> {
    storage::open(&db_path(app).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

fn load_config(conn: &rusqlite::Connection) -> anyhow::Result<TranscriptionConfig> {
    let d = TranscriptionConfig::default();
    Ok(TranscriptionConfig {
        endpoint_url: storage::get_setting(conn, "endpoint_url")?.unwrap_or(d.endpoint_url),
        model: storage::get_setting(conn, "model")?.unwrap_or(d.model),
    })
}

fn load_summary_config(conn: &rusqlite::Connection) -> anyhow::Result<SummaryConfig> {
    let d = SummaryConfig::default();
    Ok(SummaryConfig {
        endpoint_url: storage::get_setting(conn, "summary_endpoint_url")?.unwrap_or(d.endpoint_url),
        model: storage::get_setting(conn, "summary_model")?.unwrap_or(d.model),
    })
}

#[tauri::command]
pub fn recording_level(recorder: State<Recorder>) -> f32 {
    recorder.level()
}

#[derive(Serialize, Clone)]
pub struct RecordingStatus {
    pub recording: bool,
    pub elapsed_s: f64,
    pub level: f32,
}

#[tauri::command]
pub fn recording_status(recorder: State<Recorder>) -> RecordingStatus {
    let (recording, elapsed_s, level) = recorder.status();
    RecordingStatus {
        recording,
        elapsed_s,
        level,
    }
}

#[tauri::command]
pub fn is_recording(recorder: State<Recorder>) -> bool {
    recorder.is_recording()
}

/// Id da gravação em andamento (para o painel de anotações ao vivo vincular as
/// notas à gravação correta). None quando não há gravação.
#[tauri::command]
pub fn current_recording_id(recorder: State<Recorder>) -> Option<String> {
    recorder.current_id()
}

/// Salva (ou atualiza) as anotações manuais de uma gravação. Usado tanto pelo
/// painel ao vivo (autosave durante a reunião) quanto pela edição em Gravações.
#[tauri::command]
pub fn save_notes(app: AppHandle, recording_id: String, notes: String) -> Result<(), String> {
    let r = open_db(&app)
        .and_then(|conn| storage::upsert_notes(&conn, &recording_id, &notes, now_ms()).map_err(|e| e.to_string()));
    logged(&app, "anotacoes", r)
}

/// Lê as anotações manuais de uma gravação (None se ainda não houver).
#[tauri::command]
pub fn get_notes(app: AppHandle, recording_id: String) -> Result<Option<String>, String> {
    let r = open_db(&app)
        .and_then(|conn| storage::get_notes(&conn, &recording_id).map_err(|e| e.to_string()));
    logged(&app, "anotacoes", r)
}

/// Salva uma edição manual do resumo feita pelo usuário.
#[tauri::command]
pub fn set_summary(app: AppHandle, recording_id: String, text: String) -> Result<(), String> {
    let row = SummaryRow {
        recording_id,
        text,
        created_at: now_ms(),
    };
    let r = open_db(&app)
        .and_then(|conn| storage::upsert_summary(&conn, &row).map_err(|e| e.to_string()));
    logged(&app, "resumo", r)
}

fn recordings_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?.join("recordings");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn db_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let base = app.path().app_data_dir()?;
    std::fs::create_dir_all(&base)?;
    Ok(base.join("callrec.db"))
}

/// Caminho do ffmpeg: resource empacotado (prod) ou PATH/CALLREC_FFMPEG (dev).
fn resolve_ffmpeg(app: &AppHandle) -> String {
    let bin = if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" };
    // O glob `resources/*` do bundle preserva a subpasta, então o binário
    // fica em `<resource_dir>/resources/ffmpeg[.exe]`. Tenta esse caminho e,
    // por robustez, também a raiz do resource dir.
    for cand in [format!("resources/{bin}"), bin.to_string()] {
        if let Ok(p) = app.path().resolve(&cand, tauri::path::BaseDirectory::Resource) {
            if p.exists() {
                ensure_executable(&p);
                return p.to_string_lossy().into_owned();
            }
        }
    }
    std::env::var("CALLREC_FFMPEG").unwrap_or_else(|_| "ffmpeg".to_string())
}

/// macOS/Linux: garante o bit de executável no ffmpeg empacotado. O Tauri
/// trata resources como dados e pode não preservar o `+x`, o que faria a
/// execução falhar com "permission denied" mesmo achando o binário.
#[cfg(unix)]
fn ensure_executable(p: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(p) {
        let mut perms = meta.permissions();
        if perms.mode() & 0o111 == 0 {
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(p, perms);
        }
    }
}

#[cfg(not(unix))]
fn ensure_executable(_p: &Path) {}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id() -> String {
    format!("rec-{}", now_ms())
}

fn fmt_timestamp(secs: f64) -> String {
    let s = secs.max(0.0) as u64;
    format!("{:02}:{:02}", s / 60, s % 60)
}
