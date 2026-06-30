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

### PR2b — Áudio do sistema no Windows (loopback)
- [ ] WASAPI loopback via crate `wasapi` (0.23) → segunda faixa `system.wav`.
- [ ] Rodar mic + sistema em paralelo na mesma sessão.
- [ ] **Aceite**: numa chamada real (Zoom/Meet), `system.wav` tem a voz dos outros.

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

## PR4 — Encode + storage + UI de gravações
**Objetivo**: gravações leves e listadas.
- [ ] ffmpeg sidecar empacotado (download no build).
- [ ] Mix mic+sistema (`amix`) → encode **Opus `.ogg`** mono ~32 kbps.
- [ ] SQLite: tabela `recordings`; salvar metadados.
- [ ] UI: botão Gravar/Parar com tempo + medidor; lista de gravações (data, duração, tamanho).
- [ ] Tocar/abrir o arquivo.

**Aceite**: gravar→parar gera `.ogg` pequeno, aparece na lista, toca correto.

---

## PR5 — Transcrição (MiniMax) + idioma
**Objetivo**: transcrever e copiar.
- [ ] Trait `Transcriber` + `MiniMaxTranscriber` (reqwest).
- [ ] Seletor de idioma na UI (padrão **pt-BR**).
- [ ] Chave da API no **keychain** (crate `keyring`); tela para inserir.
- [ ] Tabela `transcripts`; exibir texto; botão **Copiar**.
- [ ] Estados: em progresso, erro, concluído.

**Aceite**: enviar `.ogg`, receber texto pt-BR correto, copiar para o clipboard.
**Dependência**: endpoint/credencial MiniMax confirmados — ver [MINIMAX.md](MINIMAX.md).

---

## PR6 — Apagar + Configurações
**Objetivo**: gerenciamento completo do CRUD.
- [ ] Apagar gravação (arquivo + linha no DB) e transcrição, com confirmação.
- [ ] Tela de Configurações: idioma padrão, chave da API, toggle "gravar todos" (placeholder/desabilitado até fase 2).

**Aceite**: apagar remove arquivo e registros; configurações persistem.

---

## PR7 — Empacotamento e distribuição
**Objetivo**: instaladores fáceis (não assinados na v1).
- [ ] Build de instaladores: `.msi`/NSIS, `.dmg`, `.AppImage`/`.deb`.
- [ ] Instruções de liberação (Gatekeeper/SmartScreen) no README.
- [ ] (Opcional) `tauri-plugin-updater` + canal de releases.

**Aceite**: instalar e rodar do zero em cada SO seguindo só o README.

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
