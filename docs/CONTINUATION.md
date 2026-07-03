# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Estado atual (2026-07-03, v0.2.0 "Hicorder")

**Funcional e testado pelo usuário (Windows):** gravação mic + sistema (WASAPI loopback) em faixas separadas → Opus `.webm` → SQLite; player embutido; transcrição em 2 faixas intercaladas ("Você"/"Participantes") com provedor selecionável (default Groq Whisper); resumo com provedor selecionável (default MiniMax-M3); agenda ICS com auto-start no horário, alerta no fim e auto-stop em fim+1h; tray com bolinha vermelha; upload ao Attio (busca reunião por horário → sugere participantes → nota por pessoa); teste de chave por provedor; mensagens de erro amigáveis + log persistente.

**Nesta sessão:** rename completo para **Hicorder** (identifier `com.hicapital.hicorder`, migração não destrutiva de dados/keychain em `migrate.rs` e `settings/`), logo novo (waveform, `tauri icon`), metadados de bundle p/ AV, revisão de toda a documentação, preparação SignPath ([SIGNING.md](SIGNING.md), SECURITY.md).

## Pendências (em ordem sugerida)

1. **SignPath Foundation** — usuário precisa submeter a aplicação (passos em [SIGNING.md](SIGNING.md)). Depois: integrar o passo de assinatura no `release.yml`.
2. **Renomear o repo GitHub** para `hicorder` (Settings → rename; redirects automáticos) e atualizar `homepage` no `tauri.conf.json` + links nos docs.
3. **Release v0.2.0** — tag `v0.2.0` → workflow `release` gera instaladores (draft). Testar instalador Windows com migração (dados do Call Recorder devem aparecer no Hicorder).
4. **PR2c — Linux system audio** (monitor source via cpal) e **PR3 — macOS ScreenCaptureKit**.
5. Reportar falso positivo do executável ao Kaspersky (opentip.kaspersky.com) e Microsoft quando houver release assinado ou final.

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
