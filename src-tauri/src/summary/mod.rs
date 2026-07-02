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

const SYSTEM_PROMPT: &str = "Você resume reuniões em português do Brasil. A transcrição vem \
rotulada com \"Você\" (quem gravou) e \"Participantes\". Gere um resumo claro e conciso com: \
contexto, pontos principais, decisões tomadas e itens de ação (com responsável quando houver). \
Use tópicos curtos.";

pub fn summarize(cfg: &SummaryConfig, api_key: &str, transcript: &str) -> Result<String> {
    let body = serde_json::json!({
        "model": cfg.model,
        "messages": [
            { "role": "system", "content": SYSTEM_PROMPT },
            { "role": "user", "content": transcript }
        ]
    });

    let resp = reqwest::blocking::Client::builder()
        .use_native_tls()
        .no_proxy()
        .timeout(std::time::Duration::from_secs(180))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
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
    Ok(content.trim().to_string())
}
