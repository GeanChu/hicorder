# Continuação / Handoff

Documento para a próxima sessão saber exatamente onde paramos e como seguir.

## Estado atual (2026-08-11, v0.2.51)

**Mudou o fluxo de trabalho.** O **Smart App Control** do Windows 11 ligou nesta máquina (é irreversível uma vez desligado, então ficou ligado) e bloqueia qualquer executável recém-compilado sem assinatura — `cargo build`, `cargo test` e `tauri dev` não rodam mais localmente (`os error 4551`). O ciclo agora é: branch → push → PR → a **CI compila e testa** nos 3 SOs → merge. O `npm run build` do frontend continua funcionando local (é TypeScript, não gera executável), então UI ainda dá para inspecionar no navegador.

**v0.2.51:**
- **`cargo test` na CI** — antes ela só compilava. Na primeira execução já achou um bug publicado: o filtro que separa o Claude Desktop do Claude Code só funcionava no Windows (`path.components()` não reconhece `\` fora dele, e no macOS o Desktop mora em `Claude.app`). Estava nas v0.2.48–0.2.50.
- **Áudio longo é dividido em partes** — reunião acima de ~1h45 dava `413 Payload Too Large` (Opus 32 kbps ≈ 14,4 MB/h contra o teto de 25 MB dos provedores). Divide preventivamente acima de 20 MB e reativamente em qualquer 413; timestamps deslocados para a linha do tempo da reunião; parte que falha vira marcador visível em vez de derrubar tudo.
- **Attio: deduplicação passou a ser nossa** (ADR-014) — o `external_ref` foi deprecado e o POST passou a criar sempre uma meeting nova. Os outros dois itens do comunicado (call recordings) não nos tocam.

**Novo na sessão anterior (v0.2.48–0.2.50):**
- **Alerta em um canal só** (v0.2.49): a notificação nativa foi removida (plugin inclusive) e todo aviso virou janela-toast, que é a que tem botão. Cinco tipos, novo botão "Entrar na call", e um × para dispensar — a janela não tem decoração nem barra de tarefas, então sem o × um toast sem ação ficaria preso na tela.
- **Agenda deixa de mostrar reunião fantasma** (v0.2.50): reunião apagada no Google continuava listada, e se estivesse marcada para gravar o scheduler iniciava a gravação de uma reunião inexistente. Duas causas: a sincronização era só aditiva (a única remoção era por tempo) e `STATUS:CANCELLED` era ignorado. Agora o ICS é autoritativo — com as travas de não apagar em falha de fetch nem em feed vazio — e eventos cancelados/recusados-por-mim são descartados no parser.

**Da sessão anterior (v0.2.48):**
- **Resumo pelo Claude Code local** (ADR-013): provedor que executa o CLI `claude --print` da máquina em vez de HTTP; único sem chave de API. Botão "Testar instalação" valida binário + versão + chamada real. Testado pelo usuário no app compilado.
- **Fix de segurança**: a URL do ICS (que é credencial — quem tem o link lê a agenda) caía crua no `callrec.log`. `redact()` agora mascara trechos de caminho de URL com 16+ caracteres, com teste travando os 7 endpoints legítimos.
- **Ambiente de dev reinstalado do zero** nesta máquina: Node 24.18.1, Rust stable-MSVC, VS Build Tools 17.14 (MSVC 14.44 + Windows SDK 10.0.26100), ffmpeg 8.1.2, gh 2.97.

**Armadilhas descobertas nesta sessão (não repetir):**
- `cargo build --release` **não embute o frontend** — quem decide dev vs. produção é o CLI do Tauri. O binário sai apontando para `localhost:1420` e a janela mostra `ERR_CONNECTION_REFUSED`. Para gerar executável testável: `npm run tauri build -- --no-bundle`.
- **Claude Desktop ≠ Claude Code**, e os dois executáveis se chamam `claude.exe`. O Desktop (Electron, em `AnthropicClaude/`) não embute o CLI — spawna um externo via `node-pty`. A busca do binário descarta caminhos sob `AnthropicClaude/` e exige que `--version` contenha "Claude Code".
- Rodar o app de dev **compartilha banco, gravações e keychain** com o instalado (mesmo `identifier`). Fechar o da bandeja antes, e fazer backup do `callrec.db`.

**Pendências abertas do usuário**: limpar os logs antigos e **rotacionar a URL privada do ICS** no Google Calendar — o link vazado continua nos logs já gravados e no histórico do Dropbox.

## Estado anterior (2026-07-27, v0.2.44 publicada)

**Funcional em produção (Windows e macOS testados pelo usuário):** gravação mic + sistema em faixas separadas, **codificadas ao vivo** em Opus/Ogg; player (MP3, por causa do WebKit no macOS); exportar áudio; **anotações manuais ao vivo** na Home; transcrição em duas faixas intercaladas em formato chat, com **dicionário** e **filtro de alucinação**; resumo com **prompt base editável + biblioteca de prompts** e override por reunião; **busca** em transcrição e resumo; agenda ICS com RRULE; auto-start por reunião, **lembrete horário** e **auto-stop configurável** (padrão 2h); upload ao **Attio ou Affinity** (pessoas + empresas); autoinicialização; tray; tema; auto-update; log persistente.

**Repo** `GeanChu/hicorder` (público). Releases via CI (draft → publicar com `gh release edit --draft=false --latest`). Só as 3 releases mais recentes ficam no GitHub (limpeza feita em 2026-07-08); as tags antigas continuam.

### O que quebrou e como foi resolvido (não regredir)

- **Áudio da call vazio no Windows** — `initialize_client` do loopback recebia o período mínimo do device em `buffer_duration_hns`; em drivers C-Media isso dá stream vivo porém mudo (0 eventos). Fix: **0** (default do device). Bug latente desde a v0.2.6, dependente do estado do driver.
- **Player mudo no macOS** — WebKit não decodifica Ogg/Opus. O player agora recebe `playback.mp3` gerado sob demanda.
- **CI macOS quebrando toda release** — `apple-metal` (dep transitiva do ScreenCaptureKit) compila Swift contra o SDK do macOS 26. Fix: runner `macos-15` + passo que seleciona o Xcode 26. A captura fica atrás da feature `macos-system-audio`, ligada só no macOS.
- **Auto-update morto em Windows/Linux** — o download do ffmpeg de host único falhava e a release saía só com macOS, então o `latest.json` não tinha essas plataformas. Fix: mirror do GitHub (BtbN, release n7.1) com retry e fallback. **Sempre conferir os 3 jobs antes de publicar** (`gh run view --json jobs`); `gh run watch --exit-status` retorna 0 mesmo com job da matriz falhando.
- **Chave de um provedor indo para outro** — havia fallback para a chave única antiga quando o escopo estava vazio. Removido: escopo vazio = sem chave.
- **Teste de chave falhando em modelo de raciocínio** — `max_tokens: 1` não deixa o modelo produzir nada. Agora 16.
- **Toast preso na tela** — janela sem decoração/taskbar não podia ser fechada se o webview travasse. Agora: `destroy()`, auto-dispensa em 60s e destruição de segurança no backend em 90s.

## Pendências (em ordem sugerida)

1. **Revisar o dicionário padrão** — o usuário pediu para trocar termos dos 40 atuais (lista enviada no chat); aguardando a lista nova.
2. **Affinity em uso real** — implementado e compilando, mas ainda **não testado contra a API de verdade**. Validar: `GET /auth/whoami`, busca de pessoa por email, `POST /notes` com múltiplos vínculos.
3. **SignPath Foundation** — usuário precisa submeter manualmente (form embed cross-origin + reCAPTCHA). Depois integrar no `release.yml` ([SIGNING.md](SIGNING.md)).
4. **Linux system audio** (monitor source via cpal) — hoje Linux grava só o mic.
5. **Notarização macOS** — usuário **decidiu não pagar** o Apple Developer (US$ 99/ano) por ora. Consequência aceita: a permissão de Gravação de Tela solta a cada atualização (assinatura ad-hoc muda). Recurso: `tccutil reset ScreenCapture com.hicapital.hicorder` + reabrir.
6. Reportar falso positivo do instalador ao Kaspersky/Microsoft — melhor depois de assinar (o hash muda).

## Gotchas de ambiente (máquina do usuário, Windows)

- **Kaspersky** dá falso positivo em `cargo.exe`/`rustc.exe` e pode segurar o acesso ao áudio (o prompt atrasa o início da captura). Exclusões configuradas; vale adicionar o Hicorder e o `ffmpeg.exe` como confiáveis.
- **Dropbox** trava `target/`. `.cargo/config.toml` local (não commitado) move o target para `%LOCALAPPDATA%\callrec-target`.
- **Rede do usuário tem IPv6 sem rota** — `net.rs` força IPv4 (ADR-008). Não remover sem testar.
- **Attio**: filtro `participants` do GET /v2/meetings trava o servidor (ADR-007). Não voltar a usá-lo.

## Testes locais úteis

```bash
# Pipeline real de gravação, sem instalador (Windows):
cargo test --lib rec_smoke -- --ignored --nocapture     # mic + sistema + níveis + tamanho das faixas
cargo test --lib system_probe -- --ignored --nocapture  # loopback WASAPI: eventos/bytes por etapa
cargo test --lib mic_probe -- --ignored --nocapture     # cpal isolado
```

## Regras do projeto

- Versões fixadas; lockfiles versionados e intocáveis.
- Antes de instalar pacote novo: checar data de publicação (>7 dias) e alertas (socket.dev/osv.dev).
- Commits frequentes e documentados. Nunca commitar `.env`, chaves, ou código quebrado.
- Chaves de API só no keychain/arquivo protegido; nunca em logs (nem no `callrec.log`); nunca devolvidas para a UI.
- Atualizar este arquivo ao fim de cada sessão.
