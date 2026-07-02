//! Integração com o Attio (CRM): achar/criar meeting + subir a transcrição/resumo
//! como nota em cada participante, linkando a meeting.
//!
//! API v2, auth `Authorization: Bearer <chave>`. Endpoints:
//! - GET  /v2/meetings?participants=<email>            (listar candidatas)
//! - POST /v2/meetings                                 (find-or-create)
//! - POST /v2/objects/people/records/query             (achar pessoa por email)
//! - POST /v2/notes                                    (criar nota)
//! A chave vem do keychain (nunca daqui).

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
}

fn client() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .use_native_tls()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
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
    Some(AttioMeeting {
        meeting_id,
        title,
        start,
        end,
    })
}

/// Lista meetings que têm ao menos um dos emails como participante.
pub fn list_meetings(key: &str, emails: &[String]) -> Result<Vec<AttioMeeting>> {
    let joined = emails.join(",");
    let mut params: Vec<(&str, &str)> = vec![("limit", "25")];
    if !joined.is_empty() {
        params.push(("participants", joined.as_str()));
    }
    let url = reqwest::Url::parse_with_params(&format!("{BASE}/meetings"), &params)
        .map_err(|e| anyhow!("Attio: URL inválida: {e}"))?;
    let json = get_json(key, url)?;
    let arr = json
        .get("data")
        .and_then(|d| d.as_array())
        .cloned()
        .unwrap_or_default();
    Ok(arr.iter().filter_map(meeting_from_value).collect())
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
    let body = json!({
        "data": {
            "title": title,
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
