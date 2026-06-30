# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Onde paramos
**PR0–PR5c + PR6 concluídos** (código verificado por `cargo check`; build do .exe/runtime pendente — ver Kaspersky abaixo). App grava mic + sistema (Windows) em **faixas separadas** → Opus `.webm` → SQLite; transcreve cada faixa e **intercala rotulando Você/Participantes**. Provedor HTTP configurável. Próximo: **PR6.5 (apagar gravação)** ou **PR7 (empacotar + sidecar + assinatura)**.

Commits: `git log`. PR6 na branch `pr6-two-track-transcription` (mergear).

### Transcrição — Groq (MiniMax NÃO tem STT)
Confirmado sondando endpoints: MiniMax só tem chat (MiniMax-M3) + TTS, **sem speech-to-text**. Default = **Groq Whisper** (`https://api.groq.com/openai/v1/audio/transcriptions`, `whisper-large-v3-turbo`, free tier; chave `gsk_...` em console.groq.com). Provedor OpenAiCompatible, configurável em Configurações. MiniMax-M3 reservado p/ feature futura de RESUMO. Ver [MINIMAX.md](MINIMAX.md). OBS: se o usuário já salvou config MiniMax antiga, precisa sobrescrever os 3 campos em Configurações pelos do Groq (o default só vale p/ DB novo).

### ⚠️ Kaspersky come os binários Rust (resolvido p/ compilar, atenção p/ distribuir)
Kaspersky dá **falso positivo** em `cargo.exe`/`rustc.exe` (e pode pegar o `.exe` final). Sintoma: build crasha (exit 0x40010005) ou `cargo.exe` some do toolchain. Fix: no Kaspersky, **Aplicativo Confiável** + exclusões curinga p/ `C:\Users\gfdch\.rustup\*`, `.cargo\*`, `AppData\Local\callrec-target\*`. Toolchain quebrado → `rustup toolchain uninstall stable-x86_64-pc-windows-msvc` + `install` (component add/--force NÃO re-extrai — metadados dizem "up to date"). **Verificar código com `cargo check`** (não emite .exe → Kaspersky não ataca) em vez de `cargo build`; build real do app = `npm run tauri dev`. Pode sobrar um `cargo.exe` zumbi segurando o lock do target → `npm run tauri dev` trava em "Blocking waiting for file lock"; **reboot** limpa. **Distribuir:** o .exe sem assinatura pode ser flagrado nos PCs do time → assinar (PR7) ou exclusão/política no Kaspersky deles + reportar FP em opentip.kaspersky.com.

### ⚠️ Dropbox + build artifacts
O repo está dentro do Dropbox. Isso trava o build (os error 32, arquivo em uso) porque o Dropbox sincroniza/bloqueia `target/`. Já marcamos `src-tauri/target` e `node_modules` como ignorados pelo Dropbox (stream NTFS `com.dropbox.ignored=1`). Em **outra máquina**, refazer:
```powershell
Set-Content -Path "src-tauri\target" -Stream com.dropbox.ignored -Value 1
Set-Content -Path "node_modules" -Stream com.dropbox.ignored -Value 1
```
Ideal a longo prazo: **mover o projeto para fora do Dropbox**. Se o build ainda travar (os error 32), buildar com o target fora do Dropbox:
```powershell
$env:CARGO_TARGET_DIR = "$env:LOCALAPPDATA\callrec-target"
npm run tauri dev
```

### Como testar (PR2b + PR4, Windows)
1. Garanta o ffmpeg no PATH (já instalado via winget; abra um terminal novo).
2. `npm run tauri dev` → tocar um áudio/vídeo (ou entrar numa call) → Gravar → Parar.
3. Esperado: em `...\recordings\<id>\` fica um `recording.ogg` pequeno (os WAVs são apagados após o encode). A gravação aparece na aba **Gravações** com data/duração/tamanho e **continua lá após reabrir** o app (SQLite em `app_data\callrec.db`).
4. Se o `.ogg` não tiver o áudio do sistema, é o loopback (PR2b) — confira o `system.wav` antes do encode comentando a remoção, ou rode com áudio tocando.

### Descoberta importante (PR2)
meetily **não** implementa loopback de áudio do sistema em Windows/Linux — só macOS (CoreAudio); os outros caminhos fazem `bail!("not yet implemented")` (ver `capture/system.rs` do clone em scratchpad). Logo, o loopback Win/Linux é nosso. Plano: crate `wasapi` (0.23) no Windows; `.monitor` source via `cpal` no Linux; ScreenCaptureKit no macOS (PR3).

### Como testar o PR2a
`npm run tauri dev` → aba Gravar → Gravar/Parar. Arquivo em
`%APPDATA%\com.hicapital.callrecorder\recordings\<id>\mic.wav` (Win).
Conferir que o WAV tem áudio do microfone.

### Módulos criados (src-tauri/src/audio)
- `wav.rs` (WavSink, hound) · `mic.rs` (captura cpal numa thread, stream `!Send` fica na thread) · `recorder.rs` (sessão start/stop + nível) · `mod.rs` (RecordedTrack).
- Comandos em `commands/mod.rs`; estado `Recorder` via `.manage()` no `lib.rs`.
- Nível: polling (`recording_level`) — simples; trocar por eventos se quiser.

## Estado do ambiente (máquina atual, Windows) — atualizado
- git 2.54 ✅ | node v23.10.0 ✅ | npm 10.9.2 ✅
- **Rust stable (x86_64-pc-windows-msvc): INSTALADO** ✅ (via winget; `~/.cargo/bin` — pode não estar no PATH de shells antigos; abrir novo terminal)
- **VS 2022 Build Tools (VC.Tools + Win11 SDK): INSTALADO** ✅ (`link.exe` ok; `cargo build` compila)
- WebView2 ✅ (149.x)
- **ffmpeg: NÃO instalado** ❌ — necessário só no PR4 (dev). Em prod vai como sidecar.
- winget e choco disponíveis para instalar deps.

## Como rodar agora
```bash
npm install          # já feito; lockfile versionado
npm run tauri dev    # compila Rust + abre a janela do app
```
(Primeira compilação Rust ~4 min; depois é incremental.)

## Próximo passo imediato — escolher
**PR6 — Apagar gravação** [não precisa de credencial]: comando `delete_recording(id)` em `commands` (apaga o arquivo `.ogg` + linha em `recordings` + transcript; helper `storage::delete`), com confirmação na UI (botão na aba Gravações). A tela de Configurações já existe (idioma, endpoint, modelo, chave) — falta o toggle "gravar todos" (placeholder, vira fase 2).

**PR7 — Empacotar** [pra "executável fácil"]: ffmpeg como **sidecar** do Tauri (externalBin `binaries/ffmpeg-<triple>`), pra prod não depender de ffmpeg no PATH (hoje `encode` usa o do PATH/`CALLREC_FFMPEG`). Gerar instaladores (.msi/NSIS, .dmg, .AppImage) não assinados + instruções.

**PR2c (Linux) / PR3 (macOS)** — áudio do sistema nas outras plataformas, quando houver acesso a elas.

## Reuso do meetily (MIT)
Repo: https://github.com/Zackriya-Solutions/meetily — pasta `frontend/src-tauri/src/audio/`.
Portar no PR2/PR3:
- `audio/capture/` (WASAPI/CoreAudio/PulseAudio), `audio/devices/`
- `audio/level_monitor.rs`, `audio/incremental_saver.rs`, `audio/recording_manager.rs`
- `audio/ffmpeg_mixer.rs` (adaptar saída para Opus)

NÃO portar: `whisper_engine/`, `parakeet_engine/`, `llama-helper/`, `ollama/` (transcrição local — não usamos; vamos de API).
Manter atribuição MIT no [NOTICE](../NOTICE).

## Pendências do usuário (bloqueiam PR5)
- Fornecer **endpoint ASR + chave da MiniMax** (ver [MINIMAX.md](MINIMAX.md)). Se não houver ASR, decidir fallback (Whisper).

## Decisões já tomadas (não reabrir sem motivo)
Tauri 2; macOS=ScreenCaptureKit; transcrição plugável (MiniMax default); v1 sem assinatura; áudio Opus `.ogg`; ffmpeg sidecar. Detalhe em [DECISIONS.md](DECISIONS.md).

## Regras do projeto
- Versões fixadas; lockfiles versionados e intocáveis.
- Antes de instalar pacote novo: checar data de publicação (>7 dias) e alertas (socket.dev/osv.dev).
- Commits frequentes e documentados. Nunca commitar `.env`, chaves, ou código quebrado.
- Atualizar este arquivo ao fim de cada sessão.
