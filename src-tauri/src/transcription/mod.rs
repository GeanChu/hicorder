//! Transcrição plugável. Trait `Transcriber` + provedor HTTP compatível com a
//! API OpenAI (`/audio/transcriptions`, multipart), que cobre OpenAI/Groq Whisper
//! e qualquer endpoint compatível. MiniMax: configurar a URL/modelo nas Configurações
//! (ver docs/MINIMAX.md). A chave vem do keychain (nunca daqui).

use std::path::Path;

use anyhow::{anyhow, bail, Result};
use serde::{Deserialize, Serialize};

/// Config não-secreta do provedor (persistida em SQLite). A chave fica no keychain.
#[derive(Serialize, Deserialize, Clone)]
pub struct TranscriptionConfig {
    /// URL completa do endpoint de transcrição (inclui query se o provedor exigir).
    pub endpoint_url: String,
    /// Nome do modelo enviado no campo `model`.
    pub model: String,
}

impl Default for TranscriptionConfig {
    fn default() -> Self {
        // MiniMax (global). Para a China, trocar em Configurações para api.minimaxi.com.
        Self {
            endpoint_url: "https://api.minimax.io/v1/audio/transcriptions".to_string(),
            model: "MiniMax-ASR".to_string(),
        }
    }
}

pub trait Transcriber {
    /// Transcreve o arquivo no idioma indicado (ex.: "pt"). Retorna o texto.
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<String>;
}

/// Provedor multipart compatível com a API OpenAI de transcrição.
pub struct OpenAiCompatible {
    pub endpoint_url: String,
    pub model: String,
    pub api_key: String,
}

impl Transcriber for OpenAiCompatible {
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<String> {
        let form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("language", language.to_string())
            .text("response_format", "json")
            .file("file", audio_path)
            .map_err(|e| anyhow!("falha ao anexar o áudio: {e}"))?;

        let resp = reqwest::blocking::Client::new()
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
        let text = json
            .get("text")
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("resposta sem campo 'text': {body}"))?;
        Ok(text.to_string())
    }
}
