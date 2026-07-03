# Arquitetura

## Visão geral
Aplicativo desktop único (Tauri 2). UI web (React) conversa com o backend Rust via comandos Tauri (IPC) e eventos. O backend captura áudio, faz encode, persiste em SQLite e chama as APIs de IA/CRM.

```
┌─────────────────────────────────────────────────┐
│  UI (Vite + React + TS) — src/App.tsx           │
│  Gravar · Agenda · Gravações · Transcrição ·    │
│  Configurações (provedores, chaves, logs)       │
└───────────────┬─────────────────────────────────┘
                │ comandos Tauri (IPC) + eventos
┌───────────────▼─────────────────────────────────┐
│  Backend Rust (src-tauri/src)                   │
│  audio/         mic (cpal) + sistema (WASAPI)   │
│  encode/        ffmpeg (resource) → Opus .webm  │
│  transcription/ OpenAiCompatible (multipart)    │
│  summary/       chat completions (OpenAI-like)  │
│  attio/         meetings + people + notes       │
│  meetings/      ICS fetch/parse (agenda)        │
│  scheduler/     auto-start/stop por agenda      │
│  tray.rs        bandeja + estado de gravação    │
│  storage/       SQLite                          │
│  settings/      chaves no keychain do SO        │
│  net.rs         HTTP client (IPv4-only, TLS SO) │
│  logs/          log persistente + humanize      │
│  migrate.rs     migração Call Recorder→Hicorder │
│  commands/      IPC para a UI                   │
└─────────────────────────────────────────────────┘
```

## Módulos

### audio/
Captura **mic** e **áudio do sistema** em faixas separadas (WAV) durante a gravação.
- `mic.rs` — captura via `cpal` numa thread dedicada (stream `!Send`).
- `loopback.rs` (Windows) — WASAPI loopback: dispositivo de render aberto com `Direction::Capture` compartilhado (flag `AUDCLNT_STREAMFLAGS_LOOPBACK`), crate `wasapi`.
- `recorder.rs` — sessão start/stop, tempo decorrido e nível para a UI.
- macOS (ScreenCaptureKit) e Linux (monitor source) estão no roadmap; nessas plataformas hoje grava só o mic.

### encode/
`ffmpeg` embutido como **resource** do bundle (baixado no CI de release; em dev usa o do PATH ou `CALLREC_FFMPEG`). Cada faixa WAV vira **Opus mono ~32 kbps em `.webm`**. Os WAVs são apagados após o encode.

### transcription/
Provedor `OpenAiCompatible`: multipart `file`+`model`+`language`+`response_format=verbose_json`, `Authorization: Bearer`. Retorna segmentos com timestamp; as faixas mic/sistema são transcritas separadamente e intercaladas com rótulos **Você**/**Participantes**. Default: Groq Whisper. `test_key()` valida a chave via `GET <base>/models`.

### summary/
Chat completions estilo OpenAI (system prompt de resumo em pt-BR). Default: MiniMax-M3. `test_key()` faz uma chamada mínima (1 token).

### attio/
API v2 do Attio com Bearer token:
- Busca de reuniões por **janela de tempo** (`ends_from`/`starts_before`/`timezone`) — o filtro `participants` do endpoint beta trava no servidor (ver ADR-007); o casamento por email é feito no cliente.
- `find_or_create_meeting`, `find_person_by_email` (people query), `create_note` (nota markdown por participante, vinculada à meeting).

### meetings/ + scheduler/
- `meetings/` — busca a URL ICS secreta do calendário e parseia VEVENTs (`ical`, `chrono-tz`). RRULE não é expandido (limitação v1).
- `scheduler/` — loop de 30s: inicia gravação de reunião habilitada no horário, notifica no fim previsto e para sozinho em fim+1h.

### storage/
SQLite (`rusqlite` bundled) em `app_data/callrec.db`:
- `recordings(id, path, system_path, created_at, duration_s, size_bytes)`
- `transcripts(recording_id, language, text, created_at)`
- `summaries(recording_id, text, created_at)`
- `settings(key, value)` — preferências não secretas (endpoints, modelos, ICS, email do Attio)
- `meetings(uid, title, starts_at, ends_at, record_enabled)`

### settings/
Chaves de API no **keychain do SO** (crate `keyring`): serviço `com.hicapital.hicorder`, usuários `transcription_api_key`, `summary_api_key`, `attio_api_key`. Fallback de migração lê o serviço antigo (`com.hicapital.callrecorder`) e copia.

### net.rs
Client `reqwest` compartilhado: TLS nativo do SO (compatível com inspeção HTTPS de antivírus), sem proxy do sistema, timeout explícito e **resolver DNS custom IPv4-only** — redes com IPv6 anunciado mas sem rota travavam a conexão até o timeout (ver ADR-008).

### logs/
Log persistente em `app_data/callrec.log` (rotação ~1MB): erros crus de API com timestamp/categoria — nunca chaves. `humanize()` converte o erro cru em mensagem para leigos exibida na UI; a UI tem "Ver logs"/"Limpar logs" nas Configurações.

### migrate.rs
Migração única e não destrutiva do identifier antigo (`com.hicapital.callrecorder`): copia a pasta de dados e corrige os paths absolutos das gravações no DB copiado. A pasta antiga permanece como backup.

## Frontend
`src/App.tsx` (tela única com abas) + `src/App.css` (design tokens). Abas: **Gravar** (botão + nível + tempo), **Agenda** (ICS + checkboxes), **Gravações** (lista + player + apagar), **Transcrição** (idioma, transcrever, copiar, resumo, envio ao Attio) e **Configurações** (selects de provedor/modelo por etapa, chaves com botão Testar, calendário, Attio, logs).

## Empacotamento
`tauri build` → `.msi`/NSIS (Windows), `.dmg` (macOS), `.AppImage`/`.deb` (Linux), gerados no CI (GitHub Actions, `release.yml`) com ffmpeg baixado por plataforma. Metadados de bundle preenchidos (publisher, copyright, descrição, homepage). Assinatura de código: em andamento via SignPath Foundation ([SIGNING.md](SIGNING.md)).
