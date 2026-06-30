# Transcrição — MiniMax (confirmado)

## Resumo
MiniMax STT é **OpenAI-compatível**. O provedor `OpenAiCompatible` (`transcription/mod.rs`) funciona direto. Defaults de fábrica já apontam pra MiniMax.

| Item | Valor |
|------|-------|
| Endpoint (global) | `https://api.minimax.io/v1/audio/transcriptions` |
| Endpoint (China) | `https://api.minimaxi.com/v1/audio/transcriptions` |
| Modelo | `MiniMax-ASR` (ou omitir = auto) |
| Auth | `Authorization: Bearer sk-cp-...` (Subscription Key do Token Plan) |
| Formatos de áudio | mp3, mp4, m4a, wav, mpga, **webm** (⚠️ **não** aceita `.ogg`/opus puro) |
| Resposta | `json` (default, tem `text`), `verbose_json` (com `segments`), `text`, `srt`, `vtt` |
| Idioma | ISO code (`pt`, `en`, `zh`); opcional |
| Quota | deduz do Token Plan (5h rolling + semanal) |

## Como o app usa
- Grava e encoda em **Opus dentro de `.webm`** (`recording.webm`) — leve e aceito pelo endpoint (é o que navegador/Open WebUI mandam).
- `transcribe` envia multipart `file` + `model` + `language` + `response_format=json`, header `Authorization: Bearer <sk-cp>`; lê `text` do JSON.
- A chave (sk-cp) fica no **keychain** do SO; endpoint/modelo são editáveis em Configurações.

## ⚠️ Região (pegadinha do sk-cp)
A sk-cp tem região embutida. Chave **global** só funciona em `api.minimax.io`; chave **China** só em `api.minimaxi.com`. Endpoint errado = **401**. Se tomar 401, trocar o endpoint em Configurações para `api.minimaxi.com`.

Checar a região da chave:
```powershell
curl.exe https://api.minimax.io/v1/models -H "Authorization: Bearer sk-cp-..."
# 401 → tentar https://api.minimaxi.com/v1/models
```

## Teste rápido (valida a chave sem o app)
```powershell
curl.exe -X POST "https://api.minimax.io/v1/audio/transcriptions" `
  -H "Authorization: Bearer sk-cp-SUA_CHAVE" `
  -F "file=@C:\caminho\chamada.mp3" `
  -F "model=MiniMax-ASR" -F "language=pt"
# { "text": "..." } = OK
```

## Configurar no app
Configurações → Endpoint = `https://api.minimax.io/v1/audio/transcriptions` (default), Modelo = `MiniMax-ASR` (default), Chave = sua `sk-cp` → Salvar. Aba Transcrição → escolher gravação + idioma (pt) → Transcrever.

## Segurança
- Chave no keychain; nunca logar áudio nem chave.
- Avisar o usuário (UI) que a transcrição envia o áudio para a MiniMax.
