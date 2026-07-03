# Roadmap — PR a PR

Cada PR é uma unidade lógica, testável e mergeável. Marque os checkboxes ao concluir. Branch por PR: `pr1-scaffold`, `pr2-audio-core`, etc.

---

## PR0 — Repo + planejamento + docs ✅ (esta sessão)
**Objetivo**: estrutura do repo e documentação de handoff.
- [x] `git init`, identidade local, `.gitignore`, LICENSE, NOTICE.
- [x] README, ARCHITECTURE, ROADMAP, DECISIONS, PROVIDERS, CONTINUATION.
- [x] Commit inicial.

---

## PR1 — Scaffold Tauri 2 ✅
**Objetivo**: app abre uma janela vazia, builda nos 3 SOs.
- [x] Rust stable (MSVC) + VS Build Tools (C++ + Win11 SDK) instalados nesta máquina. ffmpeg adiado para PR4 (só sidecar em prod).
- [x] `create-tauri-app` (Tauri 2, Vite + React + TS) mergeado na raiz do repo.
- [x] Estrutura de pastas do backend (`audio/`, `encode/`, `transcription/`, `storage/`, `settings/`, `commands/`) com stubs.
- [x] Layout base das 4 telas (Gravar, Gravações, Transcrição, Configurações), vazias.
- [x] `cargo build` compila limpo (3m48s, sem warnings); frontend builda (tsc + vite). Falta só rodar `npm run tauri dev` para ver a janela.
- [ ] CI mínima (lint + build) opcional — não feito.

**Aceite**: `npm run tauri dev` abre o app. Compilação validada no Windows; validar build no Mac/Linux quando houver acesso.

---

## PR2 — Captura de áudio core
> Nota: meetily NÃO implementa loopback Win/Linux (só macOS); o resto bail!. Então
> construímos o loopback nós mesmos. Dividido em 2a/2b/2c.

### PR2a — Microfone (cross-platform) ✅
- [x] Captura do microfone via `cpal` (0.15.3) → WAV (`hound`).
- [x] Enumeração de dispositivos de entrada (`list_input_devices`).
- [x] Nível (pico) por polling (`recording_level`); medidor na UI.
- [x] `Recorder` (sessão start/stop) + comandos `start_recording`/`stop_recording`/`is_recording`.
- [x] Tela Gravar funcional (botão, timer, medidor) + lista básica em Gravações.
- [x] `cargo build` limpo; frontend builda. Grava em `app_data/recordings/<id>/mic.wav`.

**Aceite 2a**: `npm run tauri dev`, gravar, parar → existe `mic.wav` audível.

### PR2b — Áudio do sistema no Windows (loopback) ✅ (código; falta teste em chamada real)
- [x] WASAPI loopback via crate `wasapi` (0.23) → segunda faixa `system.wav` (`audio/system.rs`).
- [x] Mic + sistema em paralelo na mesma sessão; `system_path` no resultado; medidor usa o maior pico.
- [x] Falha do loopback degrada para só-mic (não perde a gravação). Compila limpo.
- [ ] **Aceite (você)**: tocar áudio / fazer uma chamada, gravar, conferir que `system.wav` tem a voz dos outros.

### PR2c — Áudio do sistema no Linux (monitor source)
- [ ] Capturar o `.monitor` do sink padrão via `cpal` (aparece como device de entrada).
- [ ] **Aceite**: idem em Linux.

---

## PR3 — Áudio macOS
**Objetivo**: paridade no Mac.
- [ ] Ponte ScreenCaptureKit (áudio do sistema, macOS 13+).
- [ ] Mic via CoreAudio/`cpal`.
- [ ] Detecção de versão: <13 → só microfone, avisar na UI.
- [ ] Fluxo de permissão de gravação de tela (primeira execução).

**Aceite**: no macOS 13+, WAV com as duas vozes; em <13, só mic sem crash.

---

## PR4 — Encode + storage + UI de gravações ✅ (código; falta teste runtime)
**Objetivo**: gravações leves e listadas.
- [x] Mix mic+sistema (`amix`) → encode **Opus `.ogg`** mono ~32 kbps, 16 kHz (`encode/mod.rs` chama ffmpeg). WAVs brutos apagados após encode.
- [x] SQLite (rusqlite bundled): tabela `recordings`; `storage/mod.rs` (open/insert/list).
- [x] Comandos `stop_recording` (encoda+persiste) e `list_recordings`; UI lista data/duração/tamanho e persiste entre execuções.
- [x] ffmpeg instalado para dev (winget Gyan.FFmpeg). Compila limpo.
- [ ] **ffmpeg sidecar empacotado** (download no build) → movido para o PR7 (packaging), pra prod sem instalar nada.
- [ ] Tocar/abrir o arquivo na UI → PR6.
- [ ] **Aceite (você)**: gravar→parar gera `.ogg` pequeno, aparece na lista, e continua lá após reabrir o app.

---

## PR5 — Transcrição + idioma ✅ (código; falta endpoint MiniMax p/ teste real)
**Objetivo**: transcrever e copiar.
- [x] Trait `Transcriber` + provedor `OpenAiCompatible` (multipart, Bearer) — `transcription/mod.rs`. Endpoint + modelo configuráveis na UI.
- [x] Seletor de idioma (padrão **pt**) na aba Transcrição e idioma padrão em Configurações.
- [x] Chave da API no **keychain** (crate `keyring`, per-OS); campo password em Configurações (nunca exibida).
- [x] Tabela `transcripts`; exibir texto; botão **Copiar** (clipboard).
- [x] Estados em progresso/erro; comando `transcribe` é async + `spawn_blocking` (não trava a UI). Compila limpo.
- [ ] **Aceite (você)**: configurar endpoint/modelo/chave da MiniMax → transcrever uma gravação → texto pt-BR → Copiar.

**Dependência**: confirmar endpoint/modelo de ASR da MiniMax (a chave é a Subscription Key `sk-cp`, enviada como Bearer) — ver [PROVIDERS.md](PROVIDERS.md). Default de fábrica aponta p/ OpenAI Whisper como caminho que funciona.

---

## PR6 — Transcrição rotulada por faixa (Você/Participantes) ✅ (código verificado por `cargo check`)
**Objetivo**: diarização de 2 lados grátis, usando as faixas mic/system separadas.
- [x] Parar de mixar: grava `mic.webm` (Você) + `system.webm` (Participantes) separados.
- [x] `recordings.system_path` (migração defensiva ALTER TABLE); RecordingRow ganha system_path.
- [x] Provedor retorna **segmentos com timestamp** (`verbose_json`); transcribe chama mic+system, intercala por tempo, rotula. Falha do system degrada pra só-mic.
- [x] `cargo check` limpo (build do .exe bloqueado localmente pelo Kaspersky; ver CONTINUATION).
- [ ] **Aceite (você)**: transcrever uma call → texto tipo `[mm:ss] Você: ...` / `[mm:ss] Participantes: ...`.

## PR6.5 — Apagar gravação ✅ (código verificado por `cargo check`)
- [x] `delete_recording(id)`: apaga a pasta da gravação (mic.webm + system.webm) + linha em recordings + transcript.
- [x] Botão **Apagar** por item na aba Gravações, com confirmação; lista atualiza.
- [ ] **Aceite (você)**: apagar remove da lista e do disco.
- (Tela de Configurações já existe desde o PR5.) Toggle "gravar todos" = fase 2.

---

## PR7 — Empacotamento e distribuição
Repo público: github.com/GeanChu/hicorder. Estratégia de assinatura grátis: **SignPath Foundation** (OSS) + reportar FP ao Kaspersky.

### PR7a — CI/Release (GitHub Actions) ✅ (código; falta rodar na nuvem)
- [x] `.github/workflows/ci.yml`: builda no push (ubuntu/macos/windows) — **1ª compilação no Mac/Linux**, acha bugs de plataforma.
- [x] `.github/workflows/release.yml`: `tauri-action` builda instaladores (.msi/NSIS, .dmg universal, .deb/.AppImage) em tag `v*` → Release em rascunho. Build na nuvem = sem Kaspersky.
- [ ] **Aceite**: dar push → CI verde nos 3 SOs; criar tag `v0.1.0` → instaladores no Release.

### PR7b — ffmpeg empacotado (app self-contained) ✅ (código; empacotamento valida no release)
- [x] ffmpeg como **resource** do Tauri (`tauri.conf` `resources: ["resources/*"]`); `encode::mix_to_opus` recebe o caminho; `resolve_ffmpeg` usa o resource (prod) com fallback PATH/`CALLREC_FFMPEG` (dev).
- [x] `release.yml` baixa o ffmpeg estático por plataforma (gyan/johnvansickle/evermeet) → `src-tauri/resources/` antes do bundle. Binário gitignorado; `.gitkeep` mantém a pasta.
- [ ] **Aceite**: cortar tag `v0.1.0` → instaladores com ffmpeg embutido → instalar num PC sem ffmpeg e gravar/transcrever funciona.

### PR7c — Assinatura (SignPath Foundation, grátis p/ OSS)
- [ ] Aplicar em signpath.io/foundation (projeto OSS aprovado) → configurar step de assinatura no release.yml.
- [ ] Alternativa/complemento: reportar o instalador ao Kaspersky (opentip.kaspersky.com) + política de exclusão no Kaspersky do time.
- [ ] Instruções de liberação (Gatekeeper/SmartScreen) no README.

**Aceite final**: time instala e roda do zero, sem aviso de AV/segurança.

---

# Features pós-núcleo (pedidas após o núcleo)

## PR8 — Botão Play ✅ (código verificado por `cargo check`)
- [x] `stop_recording` também gera `recording.webm` mixado (só para reprodução), além das faixas separadas.
- [x] Asset protocol habilitado (`tauri.conf` scope `$APPDATA/recordings/**`, feature `protocol-asset`); player in-app `<audio>` via `convertFileSrc`.
- [x] Botão Play/Fechar por gravação na aba Gravações.
- [ ] **Aceite (você)**: Play toca os dois lados. (Webm/opus toca no WebView2/Win; validar no Mac depois.)

## PR9 — Resumo via MiniMax-M3 (opcional) ✅ (código verificado por `cargo check`)
- [x] `summary/mod.rs`: chat-completions OpenAI-compat (`generate_summary`, async+spawn_blocking) → default `https://api.minimax.io/v1/chat/completions`, `MiniMax-M3`, Bearer sk-cp. Prompt de resumo em pt-BR.
- [x] Configurações separadas e rotuladas: seção **Transcrição (Groq)** vs **Resumo (MiniMax-M3)** — endpoint/modelo/chave próprios; **2 chaves no keychain** (transcription_api_key + summary_api_key).
- [x] Tabela `summaries`; botão "Gerar resumo" na aba Transcrição (só habilita com chave configurada) + copiar; delete apaga o resumo junto.
- [ ] **Aceite (você)**: com a sk-cp configurada em Resumo → transcrever → Gerar resumo → texto do M3.

## PR10 — Banho de design ✅ (frontend compila; verificado no preview)
- [x] Design system em CSS variables (cores/espaço/tipografia/raios/sombra), light + dark mode.
- [x] Sidebar com ícones SVG inline + brand; nav com estado ativo; cards nas gravações; botão de gravar em pílula com indicador; medidor com gradiente; forms e seções refinados; estados vazios com ícone.
- [x] Sem framework (CSS puro). tsc verde; renderiza no preview vite (screenshot travou no ambiente, mas inspects confirmam estilos).

## PR11 — Calendário (era fase 2). Provider = **ICS URL** (sem OAuth).

### PR11a — ICS: listar reuniões + habilitar manual ✅ (código verificado por `cargo check`)
- [x] `meetings/mod.rs`: busca a URL ICS (reqwest), parseia VEVENTs (`ical`), datas com timezone (`chrono`/`chrono-tz`: Z/TZID/naive). RRULE não expandida (v1).
- [x] storage: tabela `meetings` (upsert preserva `record_enabled`); commands `refresh_meetings`/`list_meetings`/`set_meeting_record`.
- [x] Config: campo **URL do calendário (ICS)** + toggle **"Gravar todas"**. Aba **Agenda** lista as próximas reuniões com checkbox "Gravar" por reunião.
- [ ] **Aceite (você)**: colar a URL ICS → Atualizar → ver reuniões → marcar quais gravar.

### PR11b — Tray + start/stop manual ✅ (código verificado por `cargo check`)
- [x] Ícone na bandeja (`tray.rs`, feature `tray-icon`) com menu: iniciar/parar gravação, abrir, sair; tooltip indica "GRAVANDO".
- [x] Fechar a janela **minimiza pro tray** (app segue rodando p/ auto-gravar); left-click no tray reabre.
- [x] start/stop refatorados em `*_core` (chamáveis por command/tray/scheduler); evento `recording-changed` sincroniza UI (tray + telas).
- [x] plugin `tauri-plugin-notification` inicializado (usado no PR11c).
- [ ] **Aceite (você)**: ícone no tray; iniciar/parar por ele; fechar janela = fica no tray; tooltip muda.

### PR11c — Auto-gravar + alertas ✅ (código verificado por `cargo check`)
- [x] `scheduler.rs`: thread que a cada 30s auto-**inicia** gravação de reunião habilitada em andamento (uma vez por reunião, via `triggered` set).
- [x] Recorder guarda `meeting_end_ms`; `should_alert_end` (dispara 1x no fim) e `should_auto_stop` (fim+1h).
- [x] Notificações (`tauri-plugin-notification`): "reunião começou", "reunião terminou — recomendado parar", "gravação encerrada (fim+1h)". Parada continua **manual**.
- [ ] **Aceite (você)**: marcar uma reunião próxima em Agenda → app inicia sozinho na hora; notifica no fim; para sozinho em fim+1h. (Toast do Windows pode exigir app instalado; a lógica de start/stop roda de qualquer forma.)

---

## PR12 — Upload ao Attio (CRM) ✅ (testado pelo usuário)
- [x] `attio/mod.rs`: cliente HTTP — `list_meetings` por **janela de tempo** (`ends_from`/`starts_before`/`timezone`; o filtro `participants` do endpoint beta trava o servidor — ADR-007), `find_or_create_meeting`, `find_person_by_email`, `create_note` (nota por participante com `meeting_id`).
- [x] Chave do Attio no keychain; comandos `attio_find_meetings` + `attio_upload` (async+spawn_blocking).
- [x] UI (aba Transcrição): "Subir transcrição/resumo" → busca reuniões pelo **horário da gravação** → escolher candidata ou criar nova → participantes sugeridos como checkboxes (usuário vem desmarcado) + emails manuais → "Confirmar e subir nota".
- [x] Filtro pelas reuniões do usuário (campo "Seu email no Attio" nas Configurações).
- [x] **Aceite**: busca e upload funcionando (validado pelo usuário).

## PR13 — Qualidade de vida + rebrand ✅ (esta sessão)
- [x] Selects de provedor e modelo por etapa (STT: Groq/OpenAI/Fireworks; Resumo: OpenAI/Claude/Gemini/MiniMax; + Personalizado), com ajuda de onde obter cada chave.
- [x] Botão **Testar** por chave (STT via GET /models; resumo via completions de 1 token; Attio via GET /v2/meetings).
- [x] Mensagens de erro amigáveis (`logs::humanize`) + **log persistente** (`callrec.log`, rotação 1MB) com Ver/Limpar nas Configurações.
- [x] `net.rs` com resolver DNS IPv4-only (ADR-008) — corrige timeout em rede com IPv6 sem rota.
- [x] **Rename para Hicorder** (v0.2.0): identifier `com.hicapital.hicorder`, migração não destrutiva de dados + keychain, logo novo (waveform) e icon set completo, metadados de bundle (AV), docs revisados.
- [x] Preparação SignPath Foundation: [SIGNING.md](SIGNING.md), SECURITY.md.

## PR14 — Home, exportar, autostart e releases ✅ (esta sessão)
- [x] **Home** = agenda como tela principal (botão gravar fixo no topo); Configurações vira engrenagem no rodapé da sidebar; aba Transcrição funde-se em **Gravações** (play/renomear/apagar + exportar).
- [x] Agenda: participantes/local/**link** (botão "Entrar na call"), destaque da reunião **atual**, botão **Iniciar Gravação** por reunião, checkbox "Agendar Gravação", refresh periódico do ICS no scheduler.
- [x] Gravações com **título** (nome da reunião ou "Gravação manual") + renomear; **exportar áudio** MP3/WAV/OGG (`tauri-plugin-dialog` + ffmpeg; ogg via libopus).
- [x] **Tema** claro/escuro/automático; transcrição em **chat**; resumo remove `<think>` de modelos reasoning.
- [x] **Autoinicialização** com o SO (`tauri-plugin-autostart`, ligada por padrão, `--minimized` p/ tray) + toggle.
- [x] Toast de reunião criado na **main thread** (0x80070057); notificações/permissão no macOS via `run_on_main_thread`.
- [x] **Releases**: instaladores v0.2.0→v0.2.3 nos 3 SOs (CI verifica libopus/libmp3lame); README com download por SO + atribuição SignPath.
- [x] Fixes de build instalado: ffmpeg `resources/` + `+x` no unix; SQLite WAL + busy_timeout; **log** cobrindo gravação/agenda/player/UI (`logged()` + `log_client`).

---

## Backlog / Fase 2 — Agenda + auto-gravar
**Objetivo**: gravar reuniões automaticamente.
- [ ] Conectores de agenda (Google/Outlook/ICS-CalDAV).
- [ ] Tabela `meetings`; listar próximas reuniões.
- [ ] **Checkbox por reunião** para habilitar gravação.
- [ ] Auto-iniciar gravação quando uma reunião habilitada começa.
- [ ] Configuração **"gravar todas" como padrão**.
- [ ] (Opcional) detecção de app de reunião ativo (Zoom/Meet/Teams) para gatilho mais preciso.

---

## Convenções
- Commits: `feat|fix|refactor|chore|docs|test: <resumo imperativo em inglês>` (corpo em PT se útil).
- Versão fixada em dependências; lockfiles versionados; nunca deletar lockfile.
- Atualizar [CONTINUATION.md](CONTINUATION.md) ao fim de cada sessão.
