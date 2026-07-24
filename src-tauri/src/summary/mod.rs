//! Resumo da reunião via chat completions (OpenAI-compatível). Default: MiniMax-M3.
//! Opcional — só roda se o usuário configurar chave/endpoint em Configurações.
//! Usa a Subscription Key sk-cp da MiniMax (Bearer). A chave vem do keychain.

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct SummaryConfig {
    pub endpoint_url: String,
    pub model: String,
}

impl Default for SummaryConfig {
    fn default() -> Self {
        // MiniMax-M3 via chat completions (global; China = api.minimaxi.com).
        Self {
            endpoint_url: "https://api.minimax.io/v1/chat/completions".to_string(),
            model: "MiniMax-M3".to_string(),
        }
    }
}

pub const DEFAULT_SUMMARY_PROMPT: &str = "Você resume reuniões em português do Brasil. A transcrição vem \
rotulada com \"Você\" (quem gravou) e \"Participantes\". Gere um resumo claro e conciso com: \
contexto, pontos principais, decisões tomadas e itens de ação (com responsável quando houver). \
Use tópicos curtos. Quando houver \"Anotações manuais\" de quem gravou, use-as para enriquecer, \
corrigir e dar mais clareza ao resumo — elas têm prioridade sobre a transcrição em caso de \
conflito, pois foram escritas por uma pessoa presente na reunião.";

/// Prompt base de fábrica (para a UI oferecer "restaurar padrão").
pub fn default_prompt() -> &'static str {
    DEFAULT_SUMMARY_PROMPT
}

/// Valida a chave/endpoint/modelo com um chat completions mínimo. Espera 200.
///
/// `max_tokens` pequeno mas não 1: modelos de raciocínio (deepseek-v4-pro,
/// minimax-m3) gastam tokens pensando antes de responder e alguns provedores
/// devolvem 400 quando o teto não permite nenhuma saída útil.
pub fn test_key(cfg: &SummaryConfig, api_key: &str) -> Result<()> {
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [{ "role": "user", "content": "ping" }],
        "max_tokens": 16
    });
    let resp = crate::net::client(20)
        .post(&cfg.endpoint_url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .map_err(|e| anyhow!("falha na conexão: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let raw = resp.text().unwrap_or_default();
    bail!("provedor retornou {status}: {raw}");
}

pub fn summarize(
    cfg: &SummaryConfig,
    api_key: &str,
    transcript: &str,
    notes: Option<&str>,
    system_prompt: &str,
) -> Result<String> {
    let system_prompt = if system_prompt.trim().is_empty() {
        DEFAULT_SUMMARY_PROMPT
    } else {
        system_prompt
    };
    let user_content = match notes.map(str::trim).filter(|n| !n.is_empty()) {
        Some(n) => format!(
            "Transcrição da reunião:\n{transcript}\n\n---\nAnotações manuais de quem gravou:\n{n}"
        ),
        None => transcript.to_string(),
    };
    let mut body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": system_prompt },
            { "role": "user", "content": user_content }
        ]
    });
    // NVIDIA NIM tem um `max_tokens` padrão baixo e corta o resumo no meio.
    // Só para esse endpoint — outros provedores (ex.: o4-mini da OpenAI) rejeitam
    // ou interpretam `max_tokens` de forma diferente.
    //
    // O teto de saída varia por modelo (a própria NVIDIA sugere 8192 para o
    // minimax-m3 e 16384 para o deepseek-v4-pro); pedir acima do teto volta 400.
    // Folga suficiente: o resumo em si dá ~1k tokens, o resto é o raciocínio.
    if cfg.endpoint_url.contains("integrate.api.nvidia.com") {
        let cap = if cfg.model.to_lowercase().contains("minimax") {
            8192
        } else {
            16384
        };
        body["max_tokens"] = serde_json::json!(cap);
    }

    let resp = crate::net::client(180)
        .post(&cfg.endpoint_url)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .map_err(|e| anyhow!("falha na requisição de resumo: {e}"))?;

    let status = resp.status();
    let raw = resp.text().unwrap_or_default();
    if !status.is_success() {
        bail!("provedor de resumo retornou {status}: {raw}");
    }

    let json: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| anyhow!("resposta não-JSON ({e}): {raw}"))?;
    let content = json
        .get("choices")
        .and_then(|c| c.get(0))
        .and_then(|c| c.get("message"))
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_str())
        .ok_or_else(|| anyhow!("resposta sem choices[0].message.content: {raw}"))?;

    let mut out = strip_reasoning(content);
    // `finish_reason: "length"` = bateu no teto de tokens e o texto veio cortado.
    // Marca no proprio resumo para o corte nao passar despercebido (o usuário
    // pode editar o resumo e/ou refazer com outro modelo).
    let truncated = json
        .pointer("/choices/0/finish_reason")
        .and_then(|f| f.as_str())
        .map(|f| f == "length")
        .unwrap_or(false);
    if truncated {
        out.push_str("\n\n[Resumo truncado: o modelo atingiu o limite de tokens.]");
    }
    Ok(out)
}

/// Remove o raciocínio interno de modelos "reasoning" (MiniMax-M3, etc.):
/// blocos `<think>...</think>` e qualquer texto antes de um `</think>` solto.
fn strip_reasoning(content: &str) -> String {
    let mut s = content.to_string();

    // Remove pares <think>...</think> (case-insensitive, multi-linha).
    loop {
        let lower = s.to_lowercase();
        let (Some(open), Some(close)) = (lower.find("<think>"), lower.find("</think>")) else {
            break;
        };
        if close < open {
            break; // </think> antes de <think>: tratado abaixo.
        }
        s.replace_range(open..close + "</think>".len(), "");
    }

    // Fecha-tag órfã (abertura implícita): fica só o que vem depois da última.
    let lower = s.to_lowercase();
    if let Some(pos) = lower.rfind("</think>") {
        s = s[pos + "</think>".len()..].to_string();
    }

    s.trim().to_string()
}
