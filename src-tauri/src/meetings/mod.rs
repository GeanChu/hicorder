//! Calendário via ICS (URL secreta do calendário; sem OAuth).
//! Busca a URL, parseia VEVENTs e extrai as próximas reuniões.
//!
//! Recorrência (RRULE) é expandida numa janela futura (crate `rrule`),
//! respeitando EXDATE. RECURRENCE-ID (ocorrências movidas) não é tratado.

use std::io::BufReader;
use std::str::FromStr;

use anyhow::{anyhow, Result};
use chrono::{NaiveDateTime, TimeZone, Utc};
use chrono_tz::Tz;
use ical::property::Property;
use ical::IcalParser;
use rrule::RRuleSet;
use serde::Serialize;

/// Até onde no futuro expandir reuniões recorrentes.
const HORIZON_DAYS: i64 = 60;

/// Domínios de serviços de videochamada — usados para distinguir o link da
/// call de um link qualquer (suporte, comentários) no LOCATION/DESCRIPTION.
const MEETING_HOSTS: &[&str] = &[
    "meet.google.com",
    "zoom.us",
    "teams.microsoft.com",
    "teams.live.com",
    "teams.live",
    "whereby.com",
    "meet.jit.si",
    "webex.com",
    "gotomeeting.com",
    "gotomeet.me",
    "bluejeans.com",
    "chime.aws",
    "around.co",
    "hangouts.google.com",
    "skype.com",
    "discord.gg",
    "vc.tandem.chat",
];

#[derive(Serialize, Clone)]
pub struct Meeting {
    pub uid: String,
    pub title: String,
    /// Início em unix ms (UTC).
    pub starts_at: i64,
    /// Fim em unix ms (UTC).
    pub ends_at: i64,
    /// Emails dos participantes (ATTENDEE mailto:).
    pub participants: Vec<String>,
    pub location: Option<String>,
    /// Link da call: propriedade URL, ou primeira URL em LOCATION/DESCRIPTION.
    pub link: Option<String>,
}

pub fn fetch_and_parse(ics_url: &str) -> Result<Vec<Meeting>> {
    let body = crate::net::client(30)
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
    let now = Utc::now().timestamp_millis();
    let horizon = now + HORIZON_DAYS * 86_400_000;
    let parser = IcalParser::new(BufReader::new(body.as_bytes()));
    let mut out = Vec::new();
    for cal in parser {
        let cal = cal.map_err(|e| anyhow!("ICS inválido: {e}"))?;
        for ev in cal.events {
            let mut uid = None;
            let mut title = None;
            let mut dtstart: Option<&Property> = None;
            let mut start = None;
            let mut end = None;
            let mut participants = Vec::new();
            let mut location = None;
            let mut url = None;
            let mut description = None;
            let mut x_conference = None;
            let mut rrule = None;
            let mut exdates: Vec<String> = Vec::new();
            for p in &ev.properties {
                match p.name.as_str() {
                    "UID" => uid = p.value.clone(),
                    "SUMMARY" => title = p.value.clone(),
                    "DTSTART" => {
                        dtstart = Some(p);
                        start = parse_dt(p);
                    }
                    "DTEND" => end = parse_dt(p),
                    "ATTENDEE" => {
                        if let Some(email) = attendee_email(p) {
                            participants.push(email);
                        }
                    }
                    "LOCATION" => location = p.value.clone().filter(|v| !v.trim().is_empty()),
                    "URL" => url = p.value.clone().filter(|v| !v.trim().is_empty()),
                    "DESCRIPTION" => description = p.value.clone(),
                    "X-GOOGLE-CONFERENCE" => x_conference = p.value.clone(),
                    "RRULE" => rrule = p.value.clone(),
                    "EXDATE" => {
                        if let Some(line) = ics_line(p) {
                            exdates.push(line);
                        }
                    }
                    _ => {}
                }
            }
            let (Some(uid), Some(starts_at)) = (uid, start) else {
                continue;
            };
            let ends_at = end.unwrap_or(starts_at + 3_600_000); // default 1h
            let duration = (ends_at - starts_at).max(0);
            let title = title.unwrap_or_else(|| "(sem título)".to_string());
            let link = pick_call_link(
                x_conference.as_deref(),
                url.as_deref(),
                location.as_deref(),
                description.as_deref(),
            );

            // Ocorrências: RRULE expandida na janela, ou a única data.
            let occurrences = match (&rrule, dtstart) {
                (Some(rule), Some(ds)) => {
                    expand_rrule(ds, rule, &exdates, now, horizon).unwrap_or_else(|| vec![starts_at])
                }
                _ => vec![starts_at],
            };

            for occ in occurrences {
                // uid único por ocorrência para não colidir no banco.
                let occ_uid = if rrule.is_some() {
                    format!("{uid}-{occ}")
                } else {
                    uid.clone()
                };
                out.push(Meeting {
                    uid: occ_uid,
                    title: title.clone(),
                    starts_at: occ,
                    ends_at: occ + duration,
                    participants: participants.clone(),
                    location: location.clone(),
                    link: link.clone(),
                });
            }
        }
    }
    Ok(out)
}

/// Expande uma RRULE na janela [now-1h, horizon], respeitando EXDATE.
/// Retorna os instantes de início (unix ms) ou None se a regra não parsear.
fn expand_rrule(
    dtstart: &Property,
    rrule_value: &str,
    exdates: &[String],
    now_ms: i64,
    horizon_ms: i64,
) -> Option<Vec<i64>> {
    let mut input = dtstart_line(dtstart)?;
    input.push_str("\nRRULE:");
    input.push_str(rrule_value.trim());
    for ex in exdates {
        input.push('\n');
        input.push_str(ex);
    }

    let set = RRuleSet::from_str(&input).ok()?;
    let after = Utc
        .timestamp_millis_opt(now_ms - 3_600_000)
        .single()?
        .with_timezone(&rrule::Tz::UTC);
    let before = Utc
        .timestamp_millis_opt(horizon_ms)
        .single()?
        .with_timezone(&rrule::Tz::UTC);
    let result = set.after(after).before(before).all(200);
    let occ: Vec<i64> = result
        .dates
        .iter()
        .map(|d| d.timestamp_millis())
        .collect();
    if occ.is_empty() {
        None
    } else {
        Some(occ)
    }
}

/// Reconstrói a linha `DTSTART...` no formato iCal para alimentar o parser de RRULE.
fn dtstart_line(p: &Property) -> Option<String> {
    let value = p.value.as_ref()?;
    if let Some(tzid) = tzid_param(p) {
        Some(format!("DTSTART;TZID={tzid}:{value}"))
    } else if value.len() == 8 && value.chars().all(|c| c.is_ascii_digit()) {
        Some(format!("DTSTART;VALUE=DATE:{value}"))
    } else {
        Some(format!("DTSTART:{value}")) // inclui o sufixo Z se houver
    }
}

/// Reconstrói uma linha de propriedade (ex.: EXDATE) preservando o TZID.
fn ics_line(p: &Property) -> Option<String> {
    let value = p.value.as_ref()?;
    if let Some(tzid) = tzid_param(p) {
        Some(format!("{};TZID={tzid}:{value}", p.name))
    } else {
        Some(format!("{}:{value}", p.name))
    }
}

/// Escolhe o link da videochamada. Prioriza X-GOOGLE-CONFERENCE e URLs de
/// domínios de call conhecidos; só então cai para o LOCATION-como-URL ou a
/// propriedade URL. Nunca chuta um link solto da DESCRIPTION (pode ser suporte).
fn pick_call_link(
    x_conference: Option<&str>,
    url: Option<&str>,
    location: Option<&str>,
    description: Option<&str>,
) -> Option<String> {
    // 1. Link de conferência explícito do Google.
    if let Some(x) = x_conference {
        if let Some(u) = first_url(x) {
            return Some(u);
        }
    }
    // 2. Qualquer URL de domínio de call conhecido, em qualquer campo.
    let mut candidates: Vec<String> = Vec::new();
    if let Some(u) = url {
        candidates.extend(all_urls(u));
    }
    if let Some(l) = location {
        candidates.extend(all_urls(l));
    }
    if let Some(d) = description {
        candidates.extend(all_urls(d));
    }
    if let Some(u) = candidates.iter().find(|u| is_meeting_url(u)) {
        return Some(u.clone());
    }
    // 3. LOCATION que é uma URL (comum guardar o link da call ali).
    if let Some(l) = location {
        if let Some(u) = first_url(l) {
            return Some(u);
        }
    }
    // 4. Propriedade URL do evento.
    url.and_then(first_url)
}

fn is_meeting_url(u: &str) -> bool {
    let lu = u.to_lowercase();
    MEETING_HOSTS.iter().any(|h| lu.contains(h))
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

/// Extrai o email de um ATTENDEE ("mailto:x@y.com", case-insensitive).
fn attendee_email(p: &Property) -> Option<String> {
    let v = p.value.as_ref()?;
    let lower = v.to_lowercase();
    let email = lower.strip_prefix("mailto:").unwrap_or(&lower).trim().to_string();
    if email.contains('@') {
        Some(email)
    } else {
        None
    }
}

/// Primeira URL http(s) dentro de um texto.
fn first_url(text: &str) -> Option<String> {
    all_urls(text).into_iter().next()
}

/// Todas as URLs http(s) num texto (ICS escapa vírgulas/quebras com `\`).
fn all_urls(text: &str) -> Vec<String> {
    let unescaped = text.replace("\\,", ",").replace("\\n", " ").replace("\\;", ";");
    let mut urls = Vec::new();
    let mut rest = unescaped.as_str();
    while let Some(pos) = rest.find("https://").or_else(|| rest.find("http://")) {
        let sub = &rest[pos..];
        let end = sub
            .find(|c: char| c.is_whitespace() || c == '<' || c == '>' || c == '"' || c == ',')
            .unwrap_or(sub.len());
        let url = sub[..end].trim_end_matches(['.', ',', ';', ')']).to_string();
        if !url.is_empty() {
            urls.push(url);
        }
        rest = &sub[end..];
    }
    urls
}

fn tzid_param(p: &Property) -> Option<String> {
    let params = p.params.as_ref()?;
    params
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case("TZID"))
        .and_then(|(_, v)| v.first().cloned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expande_rrule_semanal_em_varias_ocorrencias() {
        // Semanal às segundas desde 2020 → várias ocorrências na janela futura.
        let ics = "BEGIN:VCALENDAR\r\nBEGIN:VEVENT\r\nUID:rec1\r\nSUMMARY:Semanal\r\n\
DTSTART:20200106T130000Z\r\nDTEND:20200106T140000Z\r\nRRULE:FREQ=WEEKLY;BYDAY=MO\r\n\
END:VEVENT\r\nEND:VCALENDAR\r\n";
        let ms = parse_ics(ics).unwrap();
        assert!(ms.len() > 1, "esperava várias ocorrências, veio {}", ms.len());
        let now = Utc::now().timestamp_millis();
        assert!(ms.iter().all(|m| m.ends_at >= now - 3_600_000));
        // Duração preservada (1h).
        assert!(ms.iter().all(|m| m.ends_at - m.starts_at == 3_600_000));
    }

    #[test]
    fn link_prefere_dominio_de_call() {
        let l = pick_call_link(
            None,
            None,
            Some("https://meet.google.com/abc-defg-hij"),
            Some("Suporte: https://support.exemplo.com/ticket/1"),
        );
        assert_eq!(l.as_deref(), Some("https://meet.google.com/abc-defg-hij"));
    }

    #[test]
    fn link_ignora_url_solta_da_descricao() {
        // Sem domínio de call e sem URL em LOCATION: não chuta o link de comentários.
        let l = pick_call_link(None, None, None, Some("Comentários: https://coment.exemplo.com/x"));
        assert_eq!(l, None);
    }

    #[test]
    fn link_usa_x_google_conference() {
        let l = pick_call_link(Some("https://meet.google.com/xyz-1234-abc"), None, None, None);
        assert_eq!(l.as_deref(), Some("https://meet.google.com/xyz-1234-abc"));
    }
}
