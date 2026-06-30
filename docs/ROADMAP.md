# Roadmap — PR a PR

Cada PR é uma unidade lógica, testável e mergeável. Marque os checkboxes ao concluir. Branch por PR: `pr1-scaffold`, `pr2-audio-core`, etc.

---

## PR0 — Repo + planejamento + docs ✅ (esta sessão)
**Objetivo**: estrutura do repo e documentação de handoff.
- [x] `git init`, identidade local, `.gitignore`, LICENSE, NOTICE.
- [x] README, ARCHITECTURE, ROADMAP, DECISIONS, MINIMAX, CONTINUATION.
- [x] Commit inicial.

---

## PR1 — Scaffold Tauri 2
**Objetivo**: app abre uma janela vazia, builda nos 3 SOs.
- [ ] Instalar pré-requisitos: Rust (stable), ffmpeg (dev), deps de sistema do Tauri.
- [ ] `npm create tauri-app@latest` → template Vite + React + TS dentro do repo.
- [ ] Estrutura de pastas do backend (`audio/`, `encode/`, `transcription/`, `storage/`, `settings/`, `commands/`) com stubs.
- [ ] `npm run tauri dev` abre janela; layout base das 4 telas (vazias).
- [ ] CI mínima (lint + build) opcional.

**Aceite**: `npm run tauri dev` abre o app em Windows/Mac/Linux.

---

## PR2 — Captura de áudio core (Windows + Linux)
**Objetivo**: gravar mic + áudio do sistema e salvar WAV bruto.
- [ ] Portar `audio/capture`, `audio/devices` do meetily (MIT) para Win/Linux.
- [ ] Enumeração de dispositivos exposta à UI.
- [ ] `level_monitor` → eventos Tauri para o medidor.
- [ ] `incremental_saver` grava WAV em disco.
- [ ] Comando `start_recording` / `stop_recording`.

**Aceite**: numa chamada real (Zoom/Meet) no Windows e no Linux, o WAV contém **as duas vozes** (mic + sistema).

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
