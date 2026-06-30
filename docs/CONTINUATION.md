# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Onde paramos
**PR0, PR1, PR2a, PR2b concluídos** (PR2b = código compilando; falta teste em chamada real). App grava **microfone + áudio do sistema (Windows)**. Próximo: **PR2c (Linux monitor source)** ou pular para **PR4 (encode Opus + storage)**.

Commits: `git log`. Branch `main` contém até PR2a; PR2b na branch `pr2b-system-audio-windows` (mergear).

### ⚠️ Dropbox + build artifacts
O repo está dentro do Dropbox. Isso trava o build (os error 32, arquivo em uso) porque o Dropbox sincroniza/bloqueia `target/`. Já marcamos `src-tauri/target` e `node_modules` como ignorados pelo Dropbox (stream NTFS `com.dropbox.ignored=1`). Em **outra máquina**, refazer:
```powershell
Set-Content -Path "src-tauri\target" -Stream com.dropbox.ignored -Value 1
Set-Content -Path "node_modules" -Stream com.dropbox.ignored -Value 1
```
Ideal a longo prazo: mover o projeto para fora do Dropbox.

### Como testar o PR2b (Windows)
`npm run tauri dev` → tocar um áudio/vídeo (ou entrar numa call) → Gravar/Parar.
Em `...\recordings\<id>\` devem existir `mic.wav` (sua voz) e `system.wav` (o que saiu na caixa). Se `system.wav` falhar, a app mantém só o `mic.wav`.

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
**Opção A — PR2c (áudio do sistema no Linux)**: em `audio/system.rs`, no bloco `#[cfg(target_os="linux")]`, abrir o `.monitor` do sink padrão via `cpal` (aparece como device de entrada cujo nome contém "monitor"). Mesmo padrão de thread do `mic.rs`. Sem teste em Linux nesta máquina (Windows) — fazer em ambiente Linux.

**Opção B — PR4 (encode + storage)** [recomendado se o foco é Windows/Mac]: ffmpeg sidecar; mix `mic.wav` + `system.wav` (`amix`) → **Opus `.ogg`** mono ~32 kbps; SQLite (`recordings`); persistir a lista (hoje só vive em memória no front). Instalar ffmpeg (dev): `choco install ffmpeg` ou `winget install Gyan.FFmpeg`.

Atenção: mic e sistema têm sample rates diferentes (mic = nativo do device; system = 48 kHz). O `amix` do ffmpeg resample no PR4.

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
