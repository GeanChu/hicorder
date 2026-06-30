# Call Recorder

Gravador de reuniões para times. Grava o áudio do **microfone** e do **sistema** (a voz dos outros participantes saindo da caixa de som), transcreve com IA e deixa você copiar, gerenciar e apagar as transcrições. Foco em UX simples para usuários não técnicos. Roda em **Windows, macOS e Linux** como aplicativo desktop.

> Status: **PR0 — planejamento e estrutura**. O código do app ainda não começou. Veja [docs/ROADMAP.md](docs/ROADMAP.md) e [docs/CONTINUATION.md](docs/CONTINUATION.md).

## O que faz (v1)
- Gravar reuniões (mic + áudio do sistema) com um botão.
- Áudio leve (Opus ~32 kbps), arquivos pequenos.
- Transcrever via IA (MiniMax por padrão), com **idioma selecionável** por transcrição (padrão **português do Brasil**).
- Copiar a transcrição.
- Apagar gravações e transcrições.

## Backlog (fase 2)
- Sincronizar com a agenda, listar as reuniões e, com um **checkbox por reunião**, habilitar gravação automática.
- Opção nas Configurações para deixar **"gravar todas"** como padrão.

## Stack
- **Tauri 2** (shell desktop, backend Rust)
- **Vite + React + TypeScript** (UI)
- **ffmpeg** empacotado como sidecar (encode/mix; usuário final não instala nada)
- **SQLite** (metadados de gravações e transcrições)
- Captura de áudio cross-platform portada do projeto **meetily** (MIT) — veja [NOTICE](NOTICE)

## Como rodar (dev) — disponível a partir do PR1
Pré-requisitos: Node 20+, Rust (stable), e dependências de sistema do Tauri. Detalhes em [docs/CONTINUATION.md](docs/CONTINUATION.md).
```bash
npm install
npm run tauri dev
```

## Documentação
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — desenho do sistema, módulos, captura de áudio por SO.
- [docs/ROADMAP.md](docs/ROADMAP.md) — plano PR a PR, com critérios de aceite.
- [docs/DECISIONS.md](docs/DECISIONS.md) — decisões de arquitetura e o porquê.
- [docs/MINIMAX.md](docs/MINIMAX.md) — integração de transcrição e pendências.
- [docs/CONTINUATION.md](docs/CONTINUATION.md) — onde paramos e como continuar (handoff).

## Privacidade
Áudio e transcrições ficam **locais** no computador do usuário. A transcrição envia o áudio para a API configurada (MiniMax). A chave da API é guardada no **keychain do sistema operacional**, não em texto puro.

## Licença
MIT. Veja [LICENSE](LICENSE) e [NOTICE](NOTICE).
