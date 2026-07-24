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
        // large-v3 (não o turbo): alucina menos em trechos de silêncio.
        Self {
            endpoint_url: "https://api.groq.com/openai/v1/audio/transcriptions".to_string(),
            model: "whisper-large-v3".to_string(),
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

/// Valida a chave/endpoint sem enviar áudio: GET `<base>/models` (espera 200).
/// Deriva a base trocando `/audio/transcriptions` por `/models`.
pub fn test_key(endpoint_url: &str, api_key: &str) -> Result<()> {
    let models_url = if endpoint_url.contains("/audio/transcriptions") {
        endpoint_url.replace("/audio/transcriptions", "/models")
    } else {
        endpoint_url.to_string()
    };
    let resp = crate::net::client(20)
        .get(&models_url)
        .bearer_auth(api_key)
        .send()
        .map_err(|e| anyhow!("falha na conexão: {e}"))?;
    let status = resp.status();
    if status.is_success() {
        return Ok(());
    }
    let body = resp.text().unwrap_or_default();
    bail!("provedor retornou {status}: {body}");
}

/// Segmento cru antes da filtragem (com as métricas do verbose_json).
struct RawSeg {
    start: f64,
    text: String,
    no_speech_prob: f64,
    avg_logprob: f64,
}

/// Substrings que só aparecem por contaminação do treino do Whisper (créditos
/// de legenda de YouTube etc.) — nunca são fala real de reunião. Case-insensitive.
const ARTIFACT_SUBSTRINGS: &[&str] = &[
    "amara.org",
    "legendas pela comunidade",
    "legendado pela comunidade",
    "legenda pela comunidade",
    "inscreva-se no canal",
    "subtitles by",
    "subtitles by the amara",
    "thanks for watching",
    "subscribe to",
];

/// Muletas curtas que o Whisper repete em silêncio.
const FILLER_HALLUCINATIONS: &[&str] = &[
    "e aí", "e ai", "aí", "obrigado", "obrigada", "muito obrigado", "valeu", "tchau", "até logo",
    "até mais", "inscreva-se", "thank you", "subscribe", "bye", "you",
];

/// Texto sem pontuação nas bordas e em minúsculas.
fn normalized(text: &str) -> String {
    text.trim()
        .trim_matches(|c: char| c.is_ascii_punctuation() || c == '…' || c.is_whitespace())
        .to_lowercase()
}

fn word_count(text: &str) -> usize {
    text.split_whitespace().filter(|w| !w.is_empty()).count()
}

/// Remove alucinações do Whisper de uma lista de segmentos de UMA faixa.
///
/// Sinais (combinados, para não apagar fala real):
/// 1. Substring de artefato de legenda → sempre descarta.
/// 2. Repetição: uma frase curta (≤6 palavras) que aparece ≥3 vezes E ocupa
///    ≥40% dos segmentos da faixa é o modelo "preenchendo" silêncio — descarta
///    todas as ocorrências. Fala real numa reunião é variada.
/// 3. Confiança: indício de silêncio (no_speech_prob) + baixa confiança
///    (avg_logprob), quando o provedor manda essas métricas.
fn filter_hallucinations(raw: Vec<RawSeg>) -> Vec<TranscriptSegment> {
    let total = raw.len();
    // Frequência das frases curtas normalizadas.
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for s in &raw {
        let n = normalized(&s.text);
        if word_count(&n) <= 6 {
            *freq.entry(n).or_insert(0) += 1;
        }
    }
    let repeated = |n: &str| -> bool {
        let c = freq.get(n).copied().unwrap_or(0);
        c >= 3 && total > 0 && c * 100 >= total * 40
    };

    raw.into_iter()
        .filter(|s| {
            let lower = s.text.to_lowercase();
            let n = normalized(&s.text);
            // 1. Artefato de legenda.
            if ARTIFACT_SUBSTRINGS.iter().any(|a| lower.contains(a)) {
                return false;
            }
            // 2. Frase curta repetida dominando a faixa.
            if word_count(&n) <= 6 && repeated(&n) {
                return false;
            }
            // 3. Silêncio + baixa confiança (métricas do provedor).
            if s.no_speech_prob > 0.6 && s.avg_logprob < -0.4 {
                return false;
            }
            // Muleta conhecida com indício de silêncio.
            if s.no_speech_prob > 0.5 && FILLER_HALLUCINATIONS.contains(&n.as_str()) {
                return false;
            }
            true
        })
        .map(|s| TranscriptSegment {
            start: s.start,
            text: s.text,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{filter_hallucinations, RawSeg};

    fn seg(t: &str) -> RawSeg {
        RawSeg { start: 0.0, text: t.into(), no_speech_prob: 0.0, avg_logprob: 0.0 }
    }

    #[test]
    fn descarta_frase_repetida_dominando_faixa() {
        // Faixa muda: "E aí" em quase todo segmento (sem métricas confiáveis).
        let raw: Vec<RawSeg> = (0..8).map(|_| seg("E aí")).collect();
        assert!(filter_hallucinations(raw).is_empty());
    }

    #[test]
    fn descarta_credito_de_legenda_repetido() {
        let raw: Vec<RawSeg> = (0..5).map(|_| seg("Legenda Adriana Zanotto")).collect();
        assert!(filter_hallucinations(raw).is_empty());
    }

    #[test]
    fn descarta_artefato_amara_mesmo_sem_repetir() {
        let raw = vec![seg("Legendas pela comunidade Amara.org")];
        assert!(filter_hallucinations(raw).is_empty());
    }

    #[test]
    fn mantem_fala_real_variada() {
        let raw = vec![
            seg("vamos fechar o valuation na semana que vem"),
            seg("perfeito, alinho com o time"),
            seg("E aí"), // "E aí" real, isolado, não domina → fica
        ];
        assert_eq!(filter_hallucinations(raw).len(), 3);
    }

    #[test]
    fn descarta_silencio_baixa_confianca() {
        let raw = vec![RawSeg {
            start: 0.0,
            text: "E aí".into(),
            no_speech_prob: 0.9,
            avg_logprob: -0.8,
        }];
        assert!(filter_hallucinations(raw).is_empty());
    }
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

        let resp = crate::net::client(180)
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
            let raw: Vec<RawSeg> = segs
                .iter()
                .filter_map(|s| {
                    let text = s.get("text").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if text.is_empty() {
                        return None;
                    }
                    Some(RawSeg {
                        start: s.get("start").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        text,
                        no_speech_prob: s.get("no_speech_prob").and_then(|v| v.as_f64()).unwrap_or(0.0),
                        avg_logprob: s.get("avg_logprob").and_then(|v| v.as_f64()).unwrap_or(0.0),
                    })
                })
                .collect();
            // `segments` presente = resposta válida; devolve o filtrado mesmo se
            // vazio (faixa era só silêncio/alucinação, não é erro).
            if !raw.is_empty() {
                return Ok(filter_hallucinations(raw));
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
