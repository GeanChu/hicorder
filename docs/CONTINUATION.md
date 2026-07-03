# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Estado atual (2026-07-03, v0.2.3 "Hicorder", publicado)

**Funcional e testado pelo usuário (Windows, incl. instalador em máquina limpa):** gravação mic + sistema (WASAPI loopback) em faixas separadas → Opus `.webm` → SQLite; player embutido; exportar áudio (MP3/WAV/OGG via ffmpeg); transcrição em 2 faixas intercaladas em formato **chat** ("Você" à direita / "Participantes" à esquerda) com provedor selecionável (default Groq Whisper); resumo com provedor selecionável (default MiniMax-M3, com remoção do `<think>`); agenda ICS (participantes/local/link, botão "Entrar na call", destaque da reunião atual, "Iniciar Gravação" por reunião, "Agendar Gravação") com auto-start, alerta e auto-stop; **autoinicialização com o SO** (ligada por padrão, toggle em Config); tray; upload ao Attio; tema claro/escuro; teste de chave por provedor; **log persistente cobrindo todas as fontes de erro**.

**Repo** renomeado para `GeanChu/hicorder` (público). Releases via CI (draft → publicar com `gh release edit --draft=false --latest`). Ver [[release-state]] na memória.

### Correções de build instalado (achadas em máquina limpa, v0.2.2/0.2.3)
- ffmpeg empacotado não era achado (glob `resources/*` põe em `resources/ffmpeg[.exe]`, não na raiz) → `resolve_ffmpeg` tenta os dois caminhos + PATH.
- macOS/Linux: `ensure_executable()` dá `chmod 0755` no ffmpeg empacotado (Tauri pode dropar o `+x`).
- "database is locked": `storage::open` liga WAL + `busy_timeout(10s)`.
- Log passou a cobrir gravação/agenda/player/UI (helpers `logged()` + comando `log_client`).

## Pendências (em ordem sugerida)

1. **SignPath Foundation** — usuário precisa submeter a aplicação manualmente (form é embed cross-origin + reCAPTCHA, resiste a automação; valores prontos + README já tem a atribuição). Depois: integrar o passo de assinatura no `release.yml` ([SIGNING.md](SIGNING.md)).
2. **PR2c — Linux system audio** (monitor source via cpal) e **PR3 — macOS ScreenCaptureKit** (hoje Mac/Linux gravam só o mic).
3. Reportar falso positivo do instalador ao Kaspersky (opentip.kaspersky.com) e Microsoft — melhor depois de assinar (o hash muda ao assinar).
4. **Notarização macOS** (Gatekeeper barra app/ffmpeg não notarizados na 1ª execução) — exige Apple Developer pago.

## Gotchas de ambiente (máquina do usuário, Windows)

- **Kaspersky** dá falso positivo em `cargo.exe`/`rustc.exe` e pode atacar o `.exe` final. Exclusões já configuradas (`.rustup\*`, `.cargo\*`, target). Verificar código com `cargo check` (não emite .exe); build real = `npm run tauri dev`. Toolchain quebrado → `rustup toolchain uninstall/install`.
- **Dropbox** trava `target/` (os error 32). `.cargo/config.toml` local (não commitado) move o target para `%LOCALAPPDATA%\callrec-target`. Em outra máquina, recriar esse arquivo ou marcar `target/`/`node_modules` como ignorados no Dropbox.
- **Rede do usuário tem IPv6 sem rota** — por isso o `net.rs` força IPv4 (ADR-008). Não remover sem testar.
- **Attio**: filtro `participants` do GET /v2/meetings trava o servidor (ADR-007). Não voltar a usá-lo.

## Regras do projeto

- Versões fixadas; lockfiles versionados e intocáveis.
- Antes de instalar pacote novo: checar data de publicação (>7 dias) e alertas (socket.dev/osv.dev).
- Commits frequentes e documentados. Nunca commitar `.env`, chaves, ou código quebrado.
- Chaves de API só no keychain; nunca em logs (nem no `callrec.log`).
- Atualizar este arquivo ao fim de cada sessão.
