//! Calendário via ICS (URL secreta do calendário; sem OAuth).
//! Busca a URL, parseia VEVENTs e extrai as próximas reuniões.
//!
//! Limitação v1: recorrência (RRULE) não é expandida — eventos recorrentes
//! aparecem só na primeira ocorrência.

use std::io::BufReader;

use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use ical::property::Property;
use ical::IcalParser;
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct Meeting {
    pub uid: String,
    pub title: String,
    /// Início em unix ms (UTC).
    pub starts_at: i64,
    /// Fim em unix ms (UTC).
    pub ends_at: i64,
}

pub fn fetch_and_parse(ics_url: &str) -> Result<Vec<Meeting>> {
    let body = reqwest::blocking::Client::builder()
        .use_native_tls()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
        .get(ics_url)
        .send()
        .map_err(|e| anyhow!("falha ao buscar o ICS: {e}"))?
        .error_for_status()
        .map_err(|e| anyhow!("o ICS retornou erro HTTP: {e}"))?
        .text()
        .map_err(|e| anyhow!("falha ao ler o ICS: {e}"))?;
    parse_ics(&body)
}

pub fn parse_ics(body: &str) -> Result<Vec<Meeting>> {
    let parser = IcalParser::new(BufReader::new(body.as_bytes()));
    let mut out = Vec::new();
    for cal in parser {
        let cal = cal.map_err(|e| anyhow!("ICS inválido: {e}"))?;
        for ev in cal.events {
            let mut uid = None;
            let mut title = None;
            let mut start = None;
            let mut end = None;
            for p in &ev.properties {
                match p.name.as_str() {
                    "UID" => uid = p.value.clone(),
                    "SUMMARY" => title = p.value.clone(),
                    "DTSTART" => start = parse_dt(p),
                    "DTEND" => end = parse_dt(p),
                    _ => {}
                }
            }
            if let (Some(uid), Some(starts_at)) = (uid, start) {
                let ends_at = end.unwrap_or(starts_at + 3_600_000); // default 1h
                out.push(Meeting {
                    uid,
                    title: title.unwrap_or_else(|| "(sem título)".to_string()),
                    starts_at,
                    ends_at,
                });
            }
        }
    }
    Ok(out)
}

/// Converte DTSTART/DTEND para unix ms (UTC). Trata sufixo Z (UTC), TZID e naive.
fn parse_dt(p: &Property) -> Option<i64> {
    let value = p.value.as_ref()?;

    // All-day (VALUE=DATE, 8 dígitos): meia-noite UTC.
    if value.len() == 8 && value.chars().all(|c| c.is_ascii_digit()) {
        let naive =
            NaiveDateTime::parse_from_str(&format!("{value}T000000"), "%Y%m%dT%H%M%S").ok()?;
        return Some(naive.and_utc().timestamp_millis());
    }

    // UTC explícito (sufixo Z).
    if let Some(stripped) = value.strip_suffix('Z') {
        let naive = NaiveDateTime::parse_from_str(stripped, "%Y%m%dT%H%M%S").ok()?;
        return Some(naive.and_utc().timestamp_millis());
    }

    let naive = NaiveDateTime::parse_from_str(value, "%Y%m%dT%H%M%S").ok()?;

    // Com TZID: converte do fuso para UTC.
    if let Some(tzid) = tzid_param(p) {
        if let Ok(tz) = tzid.parse::<Tz>() {
            if let Some(dt) = tz.from_local_datetime(&naive).single() {
                return Some(dt.with_timezone(&Utc).timestamp_millis());
            }
        }
    }

    // Floating (sem fuso): aproxima como UTC.
    Some(naive.and_utc().timestamp_millis())
}

fn tzid_param(p: &Property) -> Option<String> {
    let params = p.params.as_ref()?;
    params
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("TZID"))
        .and_then(|(_, v)| v.first().cloned())
}
