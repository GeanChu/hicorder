//! Transcrição plugável. Trait `Transcriber` + provedores (MiniMax padrão).
//!
//! Implementação no PR5. Ver docs/MINIMAX.md.
//! Idioma é parâmetro por chamada (padrão "pt-BR").
//!
//! ```ignore
//! #[async_trait]
//! pub trait Transcriber {
//!     async fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Transcript>;
//! }
//! ```
//! Impls previstas: MiniMaxTranscriber (padrão), OpenAIWhisperTranscriber (fallback).
