//! Transcrição plugável. Provedor `OpenAiCompatible` (multipart, Bearer) —
//! cobre Groq/OpenAI Whisper e qualquer endpoint compatível. Default = Groq.
//! Retorna segmentos com timestamp (via `verbose_json`) para intercalar faixas.
//! A chave vem do keychain (nunca daqui). Ver docs/MINIMAX.md.

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// Config não-secreta do provedor (persistida em SQLite). A chave fica no keychain.
#[derive(Serialize, Deserialize, Clone)]
pub struct TranscriptionConfig {
    /// URL completa do endpoint de transcrição.
    pub endpoint_url: String,
    /// Nome do modelo enviado no campo `model`.
    pub model: String,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        // Groq Whisper (OpenAI-compatível, free tier). MiniMax NÃO tem STT.
        Self {
            endpoint_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3-turbo".to_string(),
        }
    }
}

/// Um trecho transcrito com o instante de início (segundos).
pub struct TranscriptSegment {
    pub start: f64,
    pub text: String,
}

pub trait Transcriber {
    /// Transcreve o arquivo no idioma indicado (ex.: "pt"), em segmentos.
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Vec<TranscriptSegment>>;
}

/// Provedor multipart compatível com a API OpenAI de transcrição.
#[derive(Clone)]
pub struct OpenAiCompatible {
    pub endpoint_url: String,
    pub model: String,
    pub api_key: String,
}

impl Transcriber for OpenAiCompatible {
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Vec<TranscriptSegment>> {
        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("language", language.to_string())
            .text("response_format", "verbose_json")
            .file("file", audio_path)
            .map_err(|e| anyhow!("falha ao anexar o áudio: {e}"))?;

        let resp = reqwest::blocking::Client::builder()
            .use_native_tls()
            .no_proxy()
            .timeout(std::time::Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new())
            .post(&self.endpoint_url)
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .map_err(|e| anyhow!("falha na requisição ao provedor: {e}"))?;

        let status = resp.status();
        let body = resp.text().unwrap_or_default();
        if !status.is_success() {
            bail!("provedor retornou {status}: {body}");
        }

        let json: serde_json::Value = serde_json::from_str(&body)
            .map_err(|e| anyhow!("resposta não-JSON ({e}): {body}"))?;

        // verbose_json: array "segments" com start/text.
        if let Some(segs) = json.get("segments").and_then(|s| s.as_array()) {
            let mut out = Vec::new();
            for s in segs {
                let start = s.get("start").and_then(|v| v.as_f64()).unwrap_or(0.0);
                let text = s
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    out.push(TranscriptSegment { start, text });
                }
            }
            if !out.is_empty() {
                return Ok(out);
            }
        }

        // Fallback: só o campo `text` como um único segmento.
        let text = json
            .get("text")
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string();
        if text.is_empty() {
            bail!("resposta sem texto: {body}");
        }
        Ok(vec![TranscriptSegment { start: 0.0, text }])
    }
}
