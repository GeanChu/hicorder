# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Onde paramos
**PR0, PR1, PR2a concluídos.** App Tauri com captura de **microfone** funcionando (compila; testar rodando). Próximo: **PR2b (áudio do sistema no Windows, WASAPI loopback)**.

Commits: `git log`. Branch `main` contém PR0+PR1+PR2a.

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

## Próximo passo imediato (PR2b — áudio do sistema no Windows)
1. Adicionar dep só-Windows no `Cargo.toml`:
   ```toml
   [target.'cfg(windows)'.dependencies]
   wasapi = "0.23.0"
   ```
   (Conferir a API real lendo o exemplo de loopback em `~/.cargo/registry/src/.../wasapi-0.23.0/examples/`.)
2. Criar `src-tauri/src/audio/system.rs`:
   - `#[cfg(windows)]` captura loopback do device de render padrão → `Vec<f32>` → WAV `system.wav`, no mesmo padrão de thread do `mic.rs`.
   - `#[cfg(target_os="linux")]` / `#[cfg(target_os="macos")]` stubs (PR2c/PR3).
3. No `recorder.rs`: spawnar também a captura de sistema; guardar `system_handle`; em `stop()` retornar `system_path`.
4. Atenção: mic e sistema podem ter sample rates diferentes — manter faixas separadas; o mix vem no PR4 (ffmpeg).
5. Validar numa chamada real que `system.wav` tem a voz dos outros. Commit; atualizar este arquivo e o ROADMAP.

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
