//! Integração com o Affinity (CRM): sobe a transcrição/resumo/anotações como
//! nota, ligada às pessoas e empresas escolhidas.
//!
//! API v1, auth HTTP Basic com usuário vazio e a chave como senha.
//! Endpoints usados:
//! - GET  /auth/whoami                    (valida a chave)
//! - GET  /persons?term=<email>           (acha a pessoa)
//! - GET  /organizations?term=<nome>      (acha a empresa)
//! - GET  /organizations/<id>             (nome da empresa)
//! - POST /notes                          (cria a nota)
//!
//! Diferença importante para o Attio: aqui UMA nota aceita várias pessoas e
//! várias empresas (`person_ids` / `organization_ids`), então não há duplicação.

use anyhow::{anyhow, bail, Result};
use serde_json::json;

use crate::attio::AttioCompany;

const BASE: &str = "https://api.affinity.co";

fn client() -> reqwest::blocking::Client {
    crate::net::client(30)
}

/// Requisição GET autenticada (Basic: usuário vazio, senha = chave).
fn get_json(key: &str, url: &str) -> Result<serde_json::Value> {
    let resp = client()
        .get(url)
        .basic_auth("", Some(key))
        .send()
        .map_err(|e| anyhow!("Affinity: falha na requisição: {e:?}"))?;
    parse(resp)
}

fn parse(resp: reqwest::blocking::Response) -> Result<serde_json::Value> {
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        bail!("Affinity retornou {status}: {text}");
    }
    if text.trim().is_empty() {
        return Ok(serde_json::Value::Null);
    }
    serde_json::from_str(&text).map_err(|e| anyhow!("Affinity: resposta não-JSON ({e}): {text}"))
}

/// Valida a chave: GET /auth/whoami (espera 200).
pub fn test_key(key: &str) -> Result<()> {
    let resp = client()
        .get(format!("{BASE}/auth/whoami"))
        .basic_auth("", Some(key))
        .send()
        .map_err(|e| anyhow!("falha na conexão: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().unwrap_or_default();
    bail!("Affinity retornou {status}: {body}");
}

/// Escapa o termo para a query string (o mesmo esquema usado no ICS).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Acha o id de uma pessoa pelo email. None se não existir.
pub fn find_person_by_email(key: &str, email: &str) -> Result<Option<i64>> {
    let url = format!("{BASE}/persons?term={}", urlencode(email.trim()));
    let json = get_json(key, &url)?;
    let alvo = email.trim().to_lowercase();
    let Some(arr) = json.get("persons").and_then(|p| p.as_array()) else {
        return Ok(None);
    };
    // Confere o email: o `term` do Affinity também casa por nome, e subir a
    // nota para a pessoa errada é pior do que não subir.
    for p in arr {
        let bate = p
            .get("emails")
            .and_then(|e| e.as_array())
            .map(|es| {
                es.iter()
                    .filter_map(|e| e.as_str())
                    .any(|e| e.to_lowercase() == alvo)
            })
            .unwrap_or(false)
            || p.get("primary_email")
                .and_then(|e| e.as_str())
                .map(|e| e.to_lowercase() == alvo)
                .unwrap_or(false);
        if bate {
            if let Some(id) = p.get("id").and_then(|i| i.as_i64()) {
                return Ok(Some(id));
            }
        }
    }
    Ok(None)
}

/// Nome de uma empresa pelo id.
fn company_name(key: &str, id: i64) -> Result<Option<String>> {
    let json = get_json(key, &format!("{BASE}/organizations/{id}"))?;
    Ok(json
        .get("name")
        .and_then(|n| n.as_str())
        .map(String::from)
        .or_else(|| {
            json.get("domain")
                .and_then(|d| d.as_str())
                .map(String::from)
        }))
}

/// Acha o id de uma empresa pelo nome (match exato primeiro, senão o 1º achado).
pub fn find_company_by_name(key: &str, name: &str) -> Result<Option<i64>> {
    let alvo = name.trim().to_lowercase();
    let url = format!("{BASE}/organizations?term={}", urlencode(name.trim()));
    let json = get_json(key, &url)?;
    let Some(arr) = json.get("organizations").and_then(|o| o.as_array()) else {
        return Ok(None);
    };
    for o in arr {
        if o.get("name")
            .and_then(|n| n.as_str())
            .map(|n| n.to_lowercase() == alvo)
            .unwrap_or(false)
        {
            if let Some(id) = o.get("id").and_then(|i| i.as_i64()) {
                return Ok(Some(id));
            }
        }
    }
    Ok(arr
        .first()
        .and_then(|o| o.get("id"))
        .and_then(|i| i.as_i64()))
}

/// Empresas (dedup) ligadas às pessoas dos emails informados, para o usuário
/// escolher quais entram na nota. Erros por-email não derrubam o conjunto.
pub fn companies_for_emails(key: &str, emails: &[String]) -> Result<Vec<AttioCompany>> {
    let mut ids: Vec<i64> = Vec::new();
    for e in emails {
        let url = format!("{BASE}/persons?term={}", urlencode(e.trim()));
        let Ok(json) = get_json(key, &url) else { continue };
        let Some(arr) = json.get("persons").and_then(|p| p.as_array()) else {
            continue;
        };
        for p in arr {
            if let Some(orgs) = p.get("organization_ids").and_then(|o| o.as_array()) {
                for o in orgs.iter().filter_map(|o| o.as_i64()) {
                    if !ids.contains(&o) {
                        ids.push(o);
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    for id in ids {
        let name = company_name(key, id)
            .ok()
            .flatten()
            .unwrap_or_else(|| "(empresa)".to_string());
        // record_id como string mantém o mesmo formato do Attio na UI.
        out.push(AttioCompany {
            record_id: id.to_string(),
            name,
        });
    }
    Ok(out)
}

/// Cria UMA nota ligada a todas as pessoas e empresas informadas.
/// Retorna o id da nota criada.
pub fn create_note(
    key: &str,
    person_ids: &[i64],
    organization_ids: &[i64],
    title: &str,
    content: &str,
) -> Result<String> {
    // O Affinity não tem campo de título na nota: vai como primeira linha.
    let body = json!({
        "content": format!("{title}\n\n{content}"),
        "person_ids": person_ids,
        "organization_ids": organization_ids,
    });
    let resp = client()
        .post(format!("{BASE}/notes"))
        .basic_auth("", Some(key))
        .json(&body)
        .send()
        .map_err(|e| anyhow!("Affinity: falha na requisição: {e:?}"))?;
    let json = parse(resp)?;
    json.get("id")
        .map(|i| i.to_string())
        .ok_or_else(|| anyhow!("Affinity: resposta sem id da nota: {json}"))
}
