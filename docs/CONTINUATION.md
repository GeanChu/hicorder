# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Onde paramos
**PR0 concluído**: repo git inicializado, planejamento e documentação escritos. **Nenhum código de app ainda** — começa no PR1.

Commits relevantes: ver `git log`. Branch atual: `main`.

## Estado do ambiente (máquina atual, Windows)
Verificado nesta sessão:
- git 2.54 ✅
- node v23.10.0 ✅ (atenção: Tauri recomenda LTS 20/22; v23 deve funcionar, validar no PR1)
- npm 10.9.2 ✅
- **Rust (cargo/rustc): NÃO instalado** ❌ — instalar no PR1 (https://rustup.rs)
- **ffmpeg: NÃO instalado** ❌ — para dev; em produção vai como sidecar empacotado
- pnpm: não instalado (opcional; usamos npm)

## Próximo passo imediato (PR1)
1. Instalar Rust: `rustup` (toolchain stable). No Windows, instalar também o "Microsoft C++ Build Tools" (MSVC) e o WebView2 (já vem no Win11).
2. Scaffold dentro deste repo:
   ```bash
   npm create tauri-app@latest
   # template: TypeScript / React / Vite
   ```
   Ajustar para que o app fique na raiz do repo (não criar subpasta extra desnecessária).
3. Criar os módulos backend como stubs: `audio/`, `encode/`, `transcription/`, `storage/`, `settings/`, `commands/`.
4. Esqueleto das 4 telas (Gravar, Gravações, Transcrição, Configurações), vazias.
5. Validar `npm run tauri dev` abre janela.
6. Commit + atualizar este arquivo + marcar PR1 no ROADMAP.

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
