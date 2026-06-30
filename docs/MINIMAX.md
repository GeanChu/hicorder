# Transcrição — provedores

## Conclusão sobre a MiniMax
**MiniMax NÃO tem speech-to-text.** Verificado sondando os endpoints (`curl` sem auth, 2026-06-30):

| Endpoint | Resultado |
|---|---|
| `POST /v1/chat/completions` | 401 (existe — LLM MiniMax-M3) |
| `POST /v1/t2a_v2` (TTS) | 200 (existe — text→audio) |
| `POST /v1/audio/transcriptions` + 14 variações de ASR | **todas 404** |

A MiniMax oferece **chat (MiniMax-M3)** e **TTS**, mas não áudio→texto. O "OpenAI-Compatible Protocol" da doc da MiniMax é o **chat** (`/v1/chat/completions` com `MiniMax-M3`), não transcrição. A doc que sugeria `/v1/audio/transcriptions` estava errada. A chave `sk-cp` é válida, mas não serve para transcrever.

> **MiniMax-M3 fica reservado para a feature futura de RESUMO da reunião** (rodar o LLM sobre a transcrição), via `/v1/chat/completions` + `MiniMax-M3` + Bearer sk-cp. Aí a sk-cp é útil.

## Provedor de STT usado (default: Groq)
O provedor `OpenAiCompatible` (`transcription/mod.rs`) é agnóstico — endpoint/modelo/chave configuráveis em Configurações. Default de fábrica:

| Provedor | Endpoint | Modelo | Chave |
|---|---|---|---|
| **Groq Whisper** (default, grátis) | `https://api.groq.com/openai/v1/audio/transcriptions` | `whisper-large-v3-turbo` (ou `whisper-large-v3` p/ +precisão) | grátis em console.groq.com |
| OpenAI Whisper (pago) | `https://api.openai.com/v1/audio/transcriptions` | `whisper-1` | platform.openai.com |

Groq aceita `webm` (e mp3/m4a/wav/ogg/flac/mp4/mpeg/mpga). Resposta `{"text": ...}` com `response_format=json` — o provedor lê `text`. Auth `Authorization: Bearer <chave>`.

## Configurar no app
Configurações → Endpoint = (Groq, já é o default), Modelo = `whisper-large-v3-turbo`, Chave = sua chave Groq → Salvar. Aba Transcrição → escolher gravação + idioma (pt) → Transcrever → Copiar.

> Se você JÁ salvou config antiga (MiniMax) antes: sobrescreva os campos Endpoint/Modelo/Chave em Configurações com os valores do Groq e Salve. O default só vale para instalações novas.

## Segurança
- Chave no keychain do SO; nunca logar áudio nem chave.
- A transcrição envia o áudio para o provedor configurado (avisar o usuário na UI).
