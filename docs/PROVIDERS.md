# Provedores de IA

O Hicorder fala dois protocolos, ambos estilo OpenAI. Qualquer endpoint compatível funciona (opção "Personalizado" nas Configurações).

## Transcrição (speech-to-text)
Protocolo: `POST <endpoint>` multipart (`file`, `model`, `language`, `response_format=verbose_json`), `Authorization: Bearer <chave>`. O app transcreve as faixas mic/sistema separadamente e intercala com rótulos "Você"/"Participantes".

| Provedor | Endpoint | Modelos no select | Chave |
|---|---|---|---|
| **Groq** (default, free tier) | `https://api.groq.com/openai/v1/audio/transcriptions` | `whisper-large-v3-turbo`, `whisper-large-v3`, `distil-whisper-large-v3-en` | console.groq.com/keys |
| OpenAI | `https://api.openai.com/v1/audio/transcriptions` | `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe` | platform.openai.com/api-keys |
| Fireworks AI | `https://api.fireworks.ai/inference/v1/audio/transcriptions` | `whisper-v3`, `whisper-v3-turbo` | fireworks.ai → API Keys |

O áudio enviado é Opus em `.webm` (aceito pelos três).

## Resumo (LLM)
Protocolo: `POST <endpoint>` JSON chat completions (`model`, `messages`), `Authorization: Bearer <chave>`. System prompt de resumo em pt-BR embutido no app.

| Provedor | Endpoint | Modelos no select | Chave |
|---|---|---|---|
| OpenAI | `https://api.openai.com/v1/chat/completions` | `gpt-4o-mini`, `gpt-4o`, `gpt-4.1`, ... | platform.openai.com/api-keys |
| Claude (Anthropic) | `https://api.anthropic.com/v1/chat/completions` (camada compat OpenAI) | `claude-3-5-sonnet-latest`, ... | console.anthropic.com/settings/keys |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions` | `gemini-2.0-flash`, ... | aistudio.google.com/apikey |
| MiniMax (Subscription sk-cp) | `https://api.minimax.io/v1/chat/completions` | `MiniMax-M3`, `MiniMax-Text-01` | Subscription Key da conta |
| MiniMax (API) | idem | idem | platform.minimax.io → API Keys |

## Teste de chave
Botão "Testar" ao lado de cada chave nas Configurações:
- Transcrição: `GET <base>/models` (valida chave sem enviar áudio).
- Resumo: chat completions mínimo de 1 token (valida chave, endpoint e modelo).
- Attio: `GET /v2/meetings?limit=1`.

Erros aparecem em linguagem simples; o erro cru fica no log persistente (Configurações → Ver logs).

## Nota histórica: MiniMax não tem STT
Verificado em 2026-06-30 sondando endpoints: a MiniMax oferece chat (`/v1/chat/completions`, MiniMax-M3) e TTS (`/v1/t2a_v2`), mas **nenhum** endpoint de transcrição (14 variações testadas, todas 404). Por isso o default de transcrição é Groq e a MiniMax aparece só como provedor de resumo.

## Segurança
- Chaves no keychain do SO; nunca em texto puro, nunca em logs.
- A transcrição/resumo envia áudio/texto apenas ao provedor configurado.
