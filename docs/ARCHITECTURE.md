# Arquitetura

## Visão geral
Aplicativo desktop único (Tauri 2). UI web (React) conversa com backend Rust via comandos Tauri (IPC). Backend faz captura de áudio, encode, storage e chamadas à API de transcrição.

```
┌───────────────────────────────────────────────┐
│  UI (Vite + React + TS)                         │
│  - Gravar (botão + medidor de nível)            │
│  - Lista de gravações                           │
│  - Transcrição (idioma, copiar)                 │
│  - Configurações                                │
└───────────────┬───────────────────────────────┘
                │ comandos Tauri (IPC) + eventos
┌───────────────▼───────────────────────────────┐
│  Backend Rust (src-tauri)                       │
│  audio/         captura mic + sistema por SO    │
│  encode/        ffmpeg sidecar → Opus (.ogg)    │
│  transcription/ trait Transcriber + MiniMax     │
│  storage/       SQLite (gravações, transcrições)│
│  settings/      store + keychain (chave API)    │
│  commands/      IPC para a UI                   │
└───────────────┬───────────────────────────────┘
                │
        ffmpeg (sidecar empacotado)   API MiniMax (HTTPS)
```

## Módulos do backend

### audio/
Portado/adaptado do meetily (`frontend/src-tauri/src/audio/`, MIT). Responsável por capturar **microfone** e **áudio do sistema** simultaneamente e entregar PCM para encode.

- `capture/` — abstração + implementações por SO.
- `devices/` — enumeração de dispositivos (mics, saídas, monitores).
- `level_monitor.rs` — níveis em tempo real para o medidor da UI.
- `incremental_saver.rs` — grava em disco em chunks durante a captura (não estoura memória em reuniões longas).
- `recording_manager.rs` — máquina de estados start/stop, fallback de dispositivo.
- Mix mic+sistema via ffmpeg (`amix`), adaptado de `ffmpeg_mixer.rs`.

Bibliotecas: `cpal` (I/O de áudio), `windows-rs` (WASAPI loopback no Windows), bindings CoreAudio + ponte ScreenCaptureKit (macOS), `ebur128`/`nnnoiseless` opcionais (normalização/ruído).

### Captura por sistema operacional
| SO | Microfone | Áudio do sistema | Setup do usuário |
|----|-----------|------------------|------------------|
| Windows | WASAPI via `cpal` | WASAPI **loopback** | Nenhum |
| Linux | PulseAudio/PipeWire via `cpal` | **monitor source** do sink | Nenhum |
| macOS 13+ | CoreAudio/`cpal` | **ScreenCaptureKit** | Conceder permissão de gravação de tela |
| macOS <13 | CoreAudio/`cpal` | — (só microfone) | Nenhum |

macOS é o caso difícil: não há loopback nativo antigo. Usamos ScreenCaptureKit (13+); em versões antigas, v1 grava só o microfone.

### encode/
ffmpeg empacotado como **sidecar** do Tauri (binário por plataforma baixado no build, como o meetily faz no `build.rs`). Encode final: **Opus mono ~32 kbps em container `.ogg`**, 16 kHz. Leve (~7–15 MB/hora) e aceito pela maioria das APIs de ASR. Parâmetros configuráveis.

### transcription/
```rust
#[async_trait]
trait Transcriber {
    async fn transcribe(&self, audio_path: &Path, language: &str) -> Result<Transcript>;
}
```
- `MiniMaxTranscriber` (padrão) — HTTP via `reqwest`, multipart do arquivo `.ogg`, `language` por chamada (padrão `pt-BR`).
- Fallback configurável (ex.: `OpenAIWhisperTranscriber`) caso a MiniMax não atenda. Ver [MINIMAX.md](MINIMAX.md).

### storage/
SQLite (via `sqlx` ou `rusqlite`). Tabelas mínimas:
- `recordings(id, title, path, created_at, duration_s, size_bytes)`
- `transcripts(id, recording_id, language, text, provider, created_at)`
- (fase 2) `meetings(id, source, external_id, title, starts_at, record_enabled)`

### settings/
- Preferências em `tauri-plugin-store` (idioma padrão = `pt-BR`, toggle "gravar todos" — placeholder até fase 2).
- **Chave da API no keychain do SO** (crate `keyring`), nunca em texto puro nem no SQLite.

## Frontend
Vite + React + TS (mais leve que o Next.js do meetily). Telas mínimas, foco em clareza:
1. **Gravar** — botão grande Gravar/Parar, medidor de nível, tempo decorrido.
2. **Gravações** — lista com data/duração/tamanho; ações: Transcrever, Copiar, Apagar.
3. **Transcrição** — seletor de idioma (padrão pt-BR), texto, botão Copiar.
4. **Configurações** — idioma padrão, chave da API (vai pro keychain), toggle "gravar todos" (fase 2).

## Empacotamento
Instaladores por SO: `.msi`/NSIS (Windows), `.dmg` (macOS), `.AppImage`/`.deb` (Linux). v1 **não assinada** — instruções de liberação (right-click→Open no Mac, "Executar assim mesmo" no SmartScreen). Auto-update opcional via `tauri-plugin-updater`.

## Fase 2 — sincronização com agenda
- Conectores: Google Calendar / Outlook / ICS-CalDAV.
- UI: lista de reuniões com **checkbox por reunião** para habilitar gravação.
- Auto-gravar: ao iniciar uma reunião habilitada (horário do evento + app de reunião ativo), iniciar gravação automaticamente.
- Configuração "gravar todas" como padrão (marca todas as reuniões novas).
