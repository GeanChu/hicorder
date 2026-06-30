# Transcrição — MiniMax e camada de provedor

## Situação
O usuário quer transcrever com a **MiniMax** (tem chave de assinatura). Porém **a existência de um endpoint público de speech-to-text (ASR) da MiniMax não foi confirmada** nesta sessão — a busca web estava indisponível no ambiente. A MiniMax é conhecida principalmente por LLM e text-to-speech (TTS); STT precisa ser verificado.

Por isso a transcrição é uma **camada plugável**: o resto do app não depende do provedor específico.

## Trait
```rust
#[async_trait]
pub trait Transcriber {
    /// Transcreve o arquivo de áudio no idioma indicado (ex.: "pt-BR").
    async fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Transcript>;
}

pub struct Transcript {
    pub text: String,
    pub language: String,
    pub provider: String,
}
```

Implementações:
- `MiniMaxTranscriber` (padrão) — preencher endpoint/auth quando confirmado.
- `OpenAIWhisperTranscriber` (fallback) — `POST /v1/audio/transcriptions`, aceita `.ogg`/Opus, ótimo pt-BR.

## A confirmar com o usuário / verificar
1. **Endpoint ASR da MiniMax**: URL, nome do modelo, método (multipart upload? URL? streaming?).
2. **Auth**: formato do header da chave (`Authorization: Bearer ...`? `api-key`?). MiniMax costuma usar `Authorization: Bearer <key>` + `GroupId`.
3. **Formatos aceitos** e limite de tamanho/duração (confirmar se aceita Opus `.ogg`; se não, encode alternativo, ex.: mp3/wav, ou enviar em chunks).
4. **Idiomas / seleção de idioma** (confirmar pt-BR e o código esperado: `pt`, `pt-BR`, etc.).
5. **Preço/limites**.

> Quando o usuário fornecer o endpoint e a chave, preencher `MiniMaxTranscriber` no PR5. Se a MiniMax não tiver ASR utilizável, ativar o fallback (Whisper) sem mudar a UI.

## Segurança
- Chave guardada no **keychain do SO** (crate `keyring`), nunca no SQLite, em `.env` versionado, ou logada.
- Nunca logar o conteúdo do áudio nem a chave, mesmo em modo DEBUG.
- Avisar o usuário (na UI) que a transcrição envia o áudio para o provedor de IA configurado.
