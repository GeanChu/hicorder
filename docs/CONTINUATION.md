# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Onde paramos
**PR0 e PR1 concluídos.** Repo com docs + app Tauri scaffoldado e compilando. Próximo: **PR2 (captura de áudio)**.

Commits: `git log`. Branch `main` contém PR0+PR1 (PR1 mergeado de `pr1-scaffold`).

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

## Próximo passo imediato (PR2 — captura de áudio Windows + Linux)
1. Portar de meetily (MIT) o módulo `frontend/src-tauri/src/audio/` para `src-tauri/src/audio/`:
   - `capture/` (WASAPI loopback no Win, PulseAudio/PipeWire monitor no Linux), `devices/`.
   - `level_monitor.rs`, `incremental_saver.rs`, `recording_manager.rs`.
2. Adicionar deps no `Cargo.toml`: `cpal`, `windows`/`windows-rs` (loopback Win), etc. — fixar versões; checar data/alertas antes (regra do projeto).
3. Comandos Tauri `start_recording`/`stop_recording`; eventos de nível para a UI.
4. Salvar WAV bruto. Validar numa chamada real que o WAV tem mic + áudio do sistema.
5. Commit por etapa; atualizar este arquivo e o ROADMAP.

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
