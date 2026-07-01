# Roadmap — PR a PR

Cada PR é uma unidade lógica, testável e mergeável. Marque os checkboxes ao concluir. Branch por PR: `pr1-scaffold`, `pr2-audio-core`, etc.

---

## PR0 — Repo + planejamento + docs ✅ (esta sessão)
**Objetivo**: estrutura do repo e documentação de handoff.
- [x] `git init`, identidade local, `.gitignore`, LICENSE, NOTICE.
- [x] README, ARCHITECTURE, ROADMAP, DECISIONS, MINIMAX, CONTINUATION.
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

**Dependência**: confirmar endpoint/modelo de ASR da MiniMax (a chave é a Subscription Key `sk-cp`, enviada como Bearer) — ver [MINIMAX.md](MINIMAX.md). Default de fábrica aponta p/ OpenAI Whisper como caminho que funciona.

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
Repo público: github.com/GeanChu/call-recorder. Estratégia de assinatura grátis: **SignPath Foundation** (OSS) + reportar FP ao Kaspersky.

### PR7a — CI/Release (GitHub Actions) ✅ (código; falta rodar na nuvem)
- [x] `.github/workflows/ci.yml`: builda no push (ubuntu/macos/windows) — **1ª compilação no Mac/Linux**, acha bugs de plataforma.
- [x] `.github/workflows/release.yml`: `tauri-action` builda instaladores (.msi/NSIS, .dmg universal, .deb/.AppImage) em tag `v*` → Release em rascunho. Build na nuvem = sem Kaspersky.
- [ ] **Aceite**: dar push → CI verde nos 3 SOs; criar tag `v0.1.0` → instaladores no Release.

### PR7b — ffmpeg sidecar (app self-contained)
- [ ] Baixar ffmpeg por plataforma no build → `src-tauri/binaries/ffmpeg-<triple>`, `externalBin` no tauri.conf; encode chama o sidecar. Hoje o app exige ffmpeg no PATH.

### PR7c — Assinatura (SignPath Foundation, grátis p/ OSS)
- [ ] Aplicar em signpath.io/foundation (projeto OSS aprovado) → configurar step de assinatura no release.yml.
- [ ] Alternativa/complemento: reportar o instalador ao Kaspersky (opentip.kaspersky.com) + política de exclusão no Kaspersky do time.
- [ ] Instruções de liberação (Gatekeeper/SmartScreen) no README.

**Aceite final**: time instala e roda do zero, sem aviso de AV/segurança.

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
