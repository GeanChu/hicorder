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
        .map_err(|e| anyhow!("falha na conexão: {}", crate::net::describe(&e)))?;
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
    // Nomes de legendadores que o Whisper cospe em silêncio (vistos em uso).
    "adriana zanotto",
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

/// Primeiras `n` palavras do texto normalizado (chave para agrupar variantes).
fn prefix_key(text: &str, n: usize) -> String {
    normalized(text)
        .split_whitespace()
        .take(n)
        .collect::<Vec<_>>()
        .join(" ")
}

/// Remove alucinações do Whisper de uma lista de segmentos de UMA faixa.
///
/// Sinais (combinados, para não apagar fala real):
/// 1. Substring de artefato de legenda → sempre descarta.
/// 2. Repetição: o modelo repete a mesma frase em quase todo o silêncio, mas
///    com pequenas variações no fim ("Legenda Adriana Zanotto", "Legenda
///    Adriana Zanotto E a"). Por isso a contagem é feita pelo PREFIXO (3
///    primeiras palavras): prefixo com ≥3 ocorrências ocupando ≥30% dos
///    segmentos da faixa é ruído — descarta todas as variantes dele. Fala real
///    numa reunião não repete o mesmo começo de frase a esse ponto.
/// 3. Confiança: indício de silêncio (no_speech_prob) + baixa confiança
///    (avg_logprob), quando o provedor manda essas métricas.
fn filter_hallucinations(raw: Vec<RawSeg>) -> Vec<TranscriptSegment> {
    let total = raw.len();
    // Frequência por prefixo — agrupa as variantes da mesma alucinação.
    let mut freq: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for s in &raw {
        let n = normalized(&s.text);
        // Frases longas são fala real; só prefixos de trechos curtos contam.
        if word_count(&n) <= 8 {
            *freq.entry(prefix_key(&s.text, 3)).or_insert(0) += 1;
        }
    }
    let repeated = |text: &str| -> bool {
        let c = freq.get(&prefix_key(text, 3)).copied().unwrap_or(0);
        c >= 3 && total > 0 && c * 100 >= total * 30
    };

    raw.into_iter()
        .filter(|s| {
            let lower = s.text.to_lowercase();
            let n = normalized(&s.text);
            // 1. Artefato de legenda.
            if ARTIFACT_SUBSTRINGS.iter().any(|a| lower.contains(a)) {
                return false;
            }
            // 2. Prefixo repetido dominando a faixa (pega as variantes).
            if word_count(&n) <= 8 && repeated(&s.text) {
                return false;
            }
            // 2b. Crédito de legenda curto ("Legenda Fulano de Tal"), mesmo
            // isolado. Trecho longo que apenas cita a palavra é preservado.
            if (n.starts_with("legenda ") || n.starts_with("legendas "))
                && word_count(&n) <= 5
            {
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
    fn descarta_zanotto_em_qualquer_variante() {
        // Match direto pelo nome: cobre qualquer sufixo, sem depender de repetição.
        let raw = vec![
            seg("Legenda Adriana Zanotto"),
            seg("Legenda Adriana Zanotto E a"),
            seg("legendas: adriana zanotto e aí"),
        ];
        assert!(filter_hallucinations(raw).is_empty());
    }

    #[test]
    fn descarta_variantes_do_mesmo_ruido() {
        // O modelo repete a frase com pequenas variações no fim — todas caem.
        let mut raw: Vec<RawSeg> = (0..4).map(|_| seg("Legenda Adriana Zanotto")).collect();
        raw.push(seg("Legenda Adriana Zanotto E a"));
        raw.push(seg("Legenda Adriana Zanotto E aí"));
        assert!(filter_hallucinations(raw).is_empty());
    }

    #[test]
    fn descarta_credito_de_legenda_isolado() {
        let raw = vec![
            seg("Legenda Adriana Zanotto"),
            seg("precisamos revisar o contrato antes de assinar"),
        ];
        let out = filter_hallucinations(raw);
        assert_eq!(out.len(), 1);
        assert!(out[0].text.starts_with("precisamos"));
    }

    #[test]
    fn mantem_frase_longa_que_cita_legenda() {
        // "legenda" em fala real (frase longa) não pode ser descartada.
        let raw = vec![seg("a legenda do gráfico ficou errada na página três")];
        assert_eq!(filter_hallucinations(raw).len(), 1);
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

    // ---- Divisão de áudio longo ----

    use super::{chunk_seconds, erro_de_tamanho, offset_segments, TranscriptSegment};

    const MB: u64 = 1024 * 1024;

    #[test]
    fn reconhece_o_erro_de_payload_grande() {
        // Mensagem real que o usuário recebeu.
        assert!(erro_de_tamanho(
            "provedor retornou 413 Payload Too Large: {\"error\":{\"code\":\"request_too_large\"}}"
        ));
        assert!(erro_de_tamanho("Request Entity Too Large"));
        // Não pode confundir com outros erros — dividir não resolveria 401/429.
        assert!(!erro_de_tamanho("provedor retornou 401: invalid api key"));
        assert!(!erro_de_tamanho("provedor retornou 429: rate limit"));
    }

    #[test]
    fn calcula_o_pedaco_pela_taxa_de_bits_real() {
        // 2h de Opus 32 kbps ≈ 28,8 MB → ~1h por pedaço de 15 MB.
        let duracao = 7200.0;
        let tamanho = 28 * MB;
        let s = chunk_seconds(tamanho, duracao, 15 * MB);
        assert!((3400..=3900).contains(&s), "esperava ~1h, veio {s}s");
    }

    #[test]
    fn audio_mais_pesado_gera_pedaco_mais_curto() {
        // Mesma duração, o dobro do tamanho → metade do tempo por pedaço.
        let leve = chunk_seconds(28 * MB, 7200.0, 15 * MB);
        let pesado = chunk_seconds(56 * MB, 7200.0, 15 * MB);
        assert!(pesado < leve, "leve={leve} pesado={pesado}");
    }

    #[test]
    fn sem_duracao_usa_o_padrao() {
        // probe_duration falhou: não dá para estimar, cai no valor fixo.
        assert_eq!(chunk_seconds(28 * MB, 0.0, 15 * MB), 30 * 60);
        assert_eq!(chunk_seconds(0, 7200.0, 15 * MB), 30 * 60);
    }

    #[test]
    fn duracao_do_pedaco_fica_dentro_dos_limites() {
        // Bitrate altíssimo não pode gerar centenas de pedaços...
        assert_eq!(chunk_seconds(10_000 * MB, 3600.0, 15 * MB), 60);
        // ...nem bitrate baixíssimo um pedaço gigante.
        assert_eq!(chunk_seconds(1, 100_000.0, 15 * MB), 3600);
    }

    #[test]
    fn desloca_os_tempos_para_a_linha_do_tempo_da_reuniao() {
        // O provedor devolve tempos relativos ao pedaço; o segundo pedaço
        // começa aos 30min, então 12s dele são 30min12s da reunião.
        let segs = vec![
            TranscriptSegment { start: 0.0, text: "a".into() },
            TranscriptSegment { start: 12.5, text: "b".into() },
        ];
        let out = offset_segments(segs, 1800.0);
        assert_eq!(out[0].start, 1800.0);
        assert_eq!(out[1].start, 1812.5);
        assert_eq!(out[1].text, "b");
    }

    #[test]
    fn deslocamento_zero_nao_altera_nada() {
        let segs = vec![TranscriptSegment { start: 7.0, text: "x".into() }];
        assert_eq!(offset_segments(segs, 0.0)[0].start, 7.0);
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

// ---- Reunião longa: divide, transcreve em partes e remonta ----

/// Acima disso o áudio é dividido antes de subir. OpenAI e Groq recusam em
/// 25 MB (`413 Payload Too Large`); 20 MB deixa margem para o overhead do
/// multipart. Provedor com teto menor cai no retry reativo de `transcribe_file`.
const MAX_UPLOAD_BYTES: u64 = 20 * 1024 * 1024;

/// Alvo de cada pedaço. Menor que o teto de propósito: a taxa de bits do Opus
/// é variável, então um pedaço pode sair maior que a estimativa.
const CHUNK_TARGET_BYTES: u64 = 15 * 1024 * 1024;

/// Usado quando não dá para medir a duração do arquivo (probe falhou).
const CHUNK_FALLBACK_SECS: u64 = 30 * 60;

/// O erro do provedor é de tamanho de payload?
fn erro_de_tamanho(e: &str) -> bool {
    let l = e.to_lowercase();
    l.contains("413") || l.contains("too large") || l.contains("request_too_large")
}

/// Duração de cada pedaço para caber em `target_bytes`, a partir do tamanho e
/// da duração reais do arquivo (ou seja, da taxa de bits medida, não suposta).
fn chunk_seconds(size_bytes: u64, duration_s: f64, target_bytes: u64) -> u64 {
    if duration_s <= 0.0 || size_bytes == 0 {
        return CHUNK_FALLBACK_SECS;
    }
    let bytes_por_s = size_bytes as f64 / duration_s;
    if bytes_por_s <= 0.0 {
        return CHUNK_FALLBACK_SECS;
    }
    let secs = (target_bytes as f64 / bytes_por_s) as u64;
    // Piso de 1 min evita gerar centenas de pedaços com áudio de bitrate alto;
    // teto de 1h evita um pedaço único grande demais se a conta der errado.
    secs.clamp(60, 3600)
}

/// Soma `offset_s` ao início de cada segmento — o provedor devolve tempos
/// relativos ao pedaço, e o transcrito final precisa da linha de tempo da
/// reunião inteira.
fn offset_segments(segs: Vec<TranscriptSegment>, offset_s: f64) -> Vec<TranscriptSegment> {
    segs.into_iter()
        .map(|s| TranscriptSegment {
            start: s.start + offset_s,
            text: s.text,
        })
        .collect()
}

/// Transcreve um arquivo, dividindo-o quando for grande demais para o provedor.
///
/// Duas portas de entrada para a divisão:
/// 1. **preventiva** — arquivo acima de `MAX_UPLOAD_BYTES`;
/// 2. **reativa** — o provedor respondeu 413 mesmo abaixo do limite (endpoint
///    personalizado pode ter teto menor).
///
/// Falha num pedaço não descarta os demais: entra um marcador visível no lugar.
/// Perder três horas de reunião porque o pedaço 4 de 6 falhou seria pior.
pub fn transcribe_file(
    provider: &OpenAiCompatible,
    ffmpeg: &str,
    audio_path: &Path,
    language: &str,
) -> Result<Vec<TranscriptSegment>> {
    let size = std::fs::metadata(audio_path).map(|m| m.len()).unwrap_or(0);

    if size <= MAX_UPLOAD_BYTES {
        match provider.transcribe(audio_path, language) {
            Ok(segs) => return Ok(segs),
            Err(e) if erro_de_tamanho(&e.to_string()) => {
                // Teto do provedor é menor que o nosso: divide e tenta de novo.
            }
            Err(e) => return Err(e),
        }
    }

    let duracao = crate::encode::probe_duration(ffmpeg, &audio_path.to_string_lossy()).unwrap_or(0.0);
    let chunk_secs = chunk_seconds(size, duracao, CHUNK_TARGET_BYTES);

    // Pasta temporária própria, apagada ao final. O nome inclui a gravação
    // (pasta-mãe) além da faixa: sem isso, transcrever duas gravações ao mesmo
    // tempo daria colisão — as duas faixas se chamam sempre "mic" e "system".
    let faixa = audio_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("audio");
    let gravacao = audio_path
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|s| s.to_str())
        .unwrap_or("rec");
    let dir = std::env::temp_dir().join(format!("hicorder-chunks-{gravacao}-{faixa}"));
    let _ = std::fs::remove_dir_all(&dir);

    let resultado = transcribe_em_partes(provider, ffmpeg, audio_path, language, &dir, chunk_secs);
    let _ = std::fs::remove_dir_all(&dir);
    resultado
}

fn transcribe_em_partes(
    provider: &OpenAiCompatible,
    ffmpeg: &str,
    audio_path: &Path,
    language: &str,
    dir: &Path,
    chunk_secs: u64,
) -> Result<Vec<TranscriptSegment>> {
    let partes = crate::encode::split_audio(ffmpeg, audio_path, dir, chunk_secs)?;

    let mut todos: Vec<TranscriptSegment> = Vec::new();
    let mut offset = 0.0f64;
    for (i, parte) in partes.iter().enumerate() {
        // Mede o pedaço em vez de assumir `chunk_secs`: com `-c copy` o corte
        // cai na borda de pacote, então a duração real varia alguns décimos.
        // Assumir o valor nominal acumularia erro a cada pedaço.
        let dur = crate::encode::probe_duration(ffmpeg, &parte.to_string_lossy())
            .unwrap_or(chunk_secs as f64);

        match provider.transcribe(parte, language) {
            Ok(segs) => todos.extend(offset_segments(segs, offset)),
            Err(e) => {
                // Marcador visível: melhor um buraco assinalado do que perder
                // a reunião inteira ou fingir que o trecho não existia.
                todos.push(TranscriptSegment {
                    start: offset,
                    text: format!(
                        "[trecho {} de {} não transcrito: {}]",
                        i + 1,
                        partes.len(),
                        e
                    ),
                });
            }
        }
        offset += dur;
    }

    if todos.is_empty() {
        bail!("nenhum trecho do áudio pôde ser transcrito");
    }
    Ok(todos)
}

/// Vocabulário de fábrica: jargão de VC/investimentos/fintech. Vira o campo
/// `prompt` do Whisper, que condiciona o modelo a reconhecer esses termos.
/// 40 termos: deixa folga dentro do teto de 224 tokens do `prompt` para o
/// usuário cadastrar os próprios (nomes de empresas, fundos, pessoas).
/// Prioriza jargão que o modelo erra — palavras comuns do português ficam de
/// fora porque ele já acerta.
pub const DEFAULT_VOCABULARY: &[&str] = &[
    "valuation", "cap table", "term sheet", "due diligence", "follow-on", "carry",
    "runway", "burn", "MRR", "ARR", "churn", "CAC", "LTV", "ticket médio",
    "unit economics", "EBITDA", "SAFE", "vesting", "cliff", "stock options",
    "pro rata", "liquidation preference", "tag along", "drag along", "earn-out",
    "seed", "series A", "pre-seed", "venture capital", "VC", "deal flow", "fintech",
    "PIX", "adquirência", "KYC", "BACEN", "CVM", "SaaS", "GMV", "PMF",
    "product market fit",
];

/// Vocabulário padrão como texto (uma linha, separado por vírgula).
pub fn default_vocabulary() -> String {
    DEFAULT_VOCABULARY.join(", ")
}

/// Provedor multipart compatível com a API OpenAI de transcrição.
#[derive(Clone)]
pub struct OpenAiCompatible {
    pub endpoint_url: String,
    pub model: String,
    pub api_key: String,
    /// Termos que o modelo deve reconhecer (nomes, siglas, jargão). Vai no
    /// campo `prompt`, limitado a 224 tokens pela API — o excedente é
    /// descartado silenciosamente pelo provedor.
    pub vocabulary: String,
}

impl Transcriber for OpenAiCompatible {
    fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Vec<TranscriptSegment>> {
        let mut form = reqwest::blocking::multipart::Form::new()
            .text("model", self.model.clone())
            .text("language", language.to_string())
            .text("response_format", "verbose_json")
            .file("file", audio_path)
            .map_err(|e| anyhow!("falha ao anexar o áudio: {e}"))?;
        if !self.vocabulary.trim().is_empty() {
            form = form.text("prompt", self.vocabulary.trim().to_string());
        }

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
