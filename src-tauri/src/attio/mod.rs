//! Integração com o Attio (CRM): achar/criar meeting + subir a transcrição/resumo
//! como nota em cada participante, linkando a meeting.
//!
//! API v2, auth `Authorization: Bearer <chave>`. Endpoints:
//! - GET  /v2/meetings?ends_from=&starts_before=&timezone=  (candidatas por janela)
//! - POST /v2/meetings                                 (find-or-create)
//! - POST /v2/objects/people/records/query             (achar pessoa por email)
//! - POST /v2/notes                                    (criar nota)
//! A chave vem do keychain (nunca daqui).
//!
//! Nota: o filtro `participants` do GET /v2/meetings (endpoint beta) trava no
//! server do Attio. Filtramos por janela de tempo (que funciona) e casamos os
//! emails no cliente, sobre o campo `participants` de cada meeting.

use anyhow::{anyhow, bail, Result};
use serde::Serialize;
use serde_json::json;

const BASE: &str = "https://api.attio.com/v2";

#[derive(Serialize, Clone)]
pub struct AttioMeeting {
    pub meeting_id: String,
    pub title: String,
    pub start: Option<String>,
    pub end: Option<String>,
    /// Emails dos participantes (para o usuário conferir e p/ casar por email).
    pub participants: Vec<String>,
}

#[derive(Serialize, Clone)]
pub struct AttioCompany {
    pub record_id: String,
    pub name: String,
}

fn client() -> reqwest::blocking::Client {
    crate::net::client(30)
}

/// Valida a chave: GET /v2/meetings?limit=1 (espera 200). Erro traz status+corpo.
pub fn test_key(key: &str) -> Result<()> {
    let resp = client()
        .get(format!("{BASE}/meetings?limit=1"))
        .bearer_auth(key)
        .send()
        .map_err(|e| anyhow!("falha na conexão: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().unwrap_or_default();
    bail!("Attio retornou {status}: {body}");
}

fn get_json(key: &str, url: reqwest::Url) -> Result<serde_json::Value> {
    let resp = client()
        .get(url)
        .bearer_auth(key)
        .send()
        .map_err(|e| anyhow!("Attio: falha na requisição: {e:?}"))?;
    parse(resp)
}

fn post_json(key: &str, url: &str, body: &serde_json::Value) -> Result<serde_json::Value> {
    let resp = client()
        .post(url)
        .bearer_auth(key)
        .json(body)
        .send()
        .map_err(|e| anyhow!("Attio: falha na requisição: {e:?}"))?;
    parse(resp)
}

fn parse(resp: reqwest::blocking::Response) -> Result<serde_json::Value> {
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        bail!("Attio retornou {status}: {text}");
    }
    serde_json::from_str(&text).map_err(|e| anyhow!("Attio: resposta não-JSON ({e}): {text}"))
}

fn meeting_from_value(v: &serde_json::Value) -> Option<AttioMeeting> {
    let meeting_id = v
        .pointer("/id/meeting_id")
        .and_then(|x| x.as_str())
        .map(String::from)?;
    let title = v
        .get("title")
        .and_then(|x| x.as_str())
        .unwrap_or("(sem título)")
        .to_string();
    let start = v
        .pointer("/start/datetime")
        .and_then(|x| x.as_str())
        .map(String::from);
    let end = v
        .pointer("/end/datetime")
        .and_then(|x| x.as_str())
        .map(String::from);
    let participants = v
        .get("participants")
        .and_then(|p| p.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|p| {
                    p.get("email_address")
                        .and_then(|e| e.as_str())
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default();
    Some(AttioMeeting {
        meeting_id,
        title,
        start,
        end,
        participants,
    })
}

/// Lista meetings numa janela de tempo. Se `emails` for informado, filtra no
/// cliente para as que têm ao menos um dos emails como participante (com
/// fallback: se nenhuma casar, devolve todas da janela p/ o usuário escolher).
///
/// `ends_from` e `starts_before` são ISO-8601; a meeting entra se sobrepõe a
/// janela [ends_from, starts_before).
pub fn list_meetings(
    key: &str,
    ends_from: &str,
    starts_before: &str,
    timezone: &str,
    user_email: Option<&str>,
    emails: &[String],
) -> Result<Vec<AttioMeeting>> {
    let params: Vec<(&str, &str)> = vec![
        ("limit", "50"),
        ("sort", "start_asc"),
        ("ends_from", ends_from),
        ("starts_before", starts_before),
        ("timezone", timezone),
    ];
    let url = reqwest::Url::parse_with_params(&format!("{BASE}/meetings"), &params)
        .map_err(|e| anyhow!("Attio: URL inválida: {e}"))?;
    let json = get_json(key, url)?;
    let arr = json
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    let mut all: Vec<AttioMeeting> = arr.iter().filter_map(meeting_from_value).collect();

    // Filtro forte: só reuniões onde o usuário do Attio participa.
    if let Some(u) = user_email.map(str::to_lowercase).filter(|u| !u.is_empty()) {
        all.retain(|m| m.participants.iter().any(|p| p.to_lowercase() == u));
    }

    if emails.is_empty() {
        return Ok(all);
    }
    let wanted: Vec<String> = emails.iter().map(|e| e.to_lowercase()).collect();
    let matched: Vec<AttioMeeting> = all
        .iter()
        .filter(|m| {
            m.participants
                .iter()
                .any(|p| wanted.contains(&p.to_lowercase()))
        })
        .cloned()
        .collect();
    Ok(if matched.is_empty() { all } else { matched })
}

/// Acha ou cria uma meeting a partir de título/horário/participantes. Retorna o meeting_id.
pub fn find_or_create_meeting(
    key: &str,
    title: &str,
    start_iso: &str,
    end_iso: &str,
    timezone: &str,
    emails: &[String],
) -> Result<String> {
    let participants: Vec<serde_json::Value> = emails
        .iter()
        .enumerate()
        .map(|(i, e)| {
            json!({ "email_address": e, "is_organizer": i == 0, "status": "accepted" })
        })
        .collect();
    // A API de meetings (beta) exige `description` (string) e `external_ref`
    // (referência externa única). O external_ref é estável por (início+título),
    // então re-subir a mesma reunião reaproveita a existente em vez de duplicar.
    let external_ref = format!("hicorder:{start_iso}|{title}");
    let body = json!({
        "data": {
            "title": title,
            "description": title,
            "external_ref": external_ref,
            "start": { "datetime": start_iso, "timezone": timezone },
            "end": { "datetime": end_iso, "timezone": timezone },
            "is_all_day": false,
            "participants": participants
        }
    });
    let json = post_json(key, &format!("{BASE}/meetings"), &body)?;
    json.pointer("/data/id/meeting_id")
        .and_then(|x| x.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow!("Attio: resposta sem meeting_id: {json}"))
}

/// Acha o record_id de uma pessoa pelo email. None se não existir.
pub fn find_person_by_email(key: &str, email: &str) -> Result<Option<String>> {
    let body = json!({ "filter": { "email_addresses": email }, "limit": 1 });
    let json = post_json(
        key,
        &format!("{BASE}/objects/people/records/query"),
        &body,
    )?;
    Ok(json
        .pointer("/data/0/id/record_id")
        .and_then(|x| x.as_str())
        .map(String::from))
}

/// record_id da empresa vinculada a uma pessoa (atributo `company`). None se não houver.
fn person_company_id(key: &str, email: &str) -> Result<Option<String>> {
    let body = json!({ "filter": { "email_addresses": email }, "limit": 1 });
    let json = post_json(key, &format!("{BASE}/objects/people/records/query"), &body)?;
    Ok(json
        .pointer("/data/0/values/company/0/target_record_id")
        .and_then(|x| x.as_str())
        .map(String::from))
}

/// Nome de uma empresa pelo record_id. None se não achar/sem nome.
fn company_name(key: &str, record_id: &str) -> Result<Option<String>> {
    let url = reqwest::Url::parse(&format!("{BASE}/objects/companies/records/{record_id}"))
        .map_err(|e| anyhow!("Attio: URL inválida: {e}"))?;
    let json = get_json(key, url)?;
    Ok(json
        .pointer("/data/values/name/0/value")
        .and_then(|x| x.as_str())
        .map(String::from))
}

/// Empresas (dedup) vinculadas às pessoas de uma lista de emails, para o usuário
/// escolher quais também recebem a nota. Erros por-email não derrubam o conjunto.
pub fn companies_for_emails(key: &str, emails: &[String]) -> Result<Vec<AttioCompany>> {
    let mut ids: Vec<String> = Vec::new();
    for e in emails {
        if let Ok(Some(cid)) = person_company_id(key, e) {
            if !ids.contains(&cid) {
                ids.push(cid);
            }
        }
    }
    let mut out = Vec::new();
    for id in ids {
        let name = company_name(key, &id)
            .ok()
            .flatten()
            .unwrap_or_else(|| "(empresa)".to_string());
        out.push(AttioCompany { record_id: id, name });
    }
    Ok(out)
}

/// Cria uma nota num record pai, linkando a meeting.
pub fn create_note(
    key: &str,
    parent_object: &str,
    parent_record_id: &str,
    meeting_id: &str,
    title: &str,
    content_markdown: &str,
) -> Result<String> {
    let body = json!({
        "data": {
            "parent_object": parent_object,
            "parent_record_id": parent_record_id,
            "meeting_id": meeting_id,
            "title": title,
            "format": "markdown",
            "content": content_markdown
        }
    });
    let json = post_json(key, &format!("{BASE}/notes"), &body)?;
    json.pointer("/data/id/note_id")
        .and_then(|x| x.as_str())
        .map(String::from)
        .ok_or_else(|| anyhow!("Attio: resposta sem note_id: {json}"))
}
