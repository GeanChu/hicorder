# Hicorder

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Hicorder" width="96" height="96" />
</p>

Gravador de reuniões para times. Grava o áudio do **microfone** e do **sistema** (a voz dos outros participantes), transcreve com IA, gera resumo e envia notas para o CRM (Attio). Foco em UX simples para usuários não técnicos. Desktop **Windows, macOS e Linux** (Tauri 2).

> *Hicorder is a meeting recorder for teams: it captures mic + system audio, transcribes with AI, summarizes, and pushes notes to the Attio CRM. Open source (MIT), built with Tauri 2 (Rust + React).*

## O que faz

- **Gravar reuniões** (mic + áudio do sistema) com um botão, medidor de nível e player embutido.
- **Áudio leve**: Opus ~32 kbps em `.webm` (~7–15 MB/hora).
- **Transcrever via IA** com provedor selecionável (Groq Whisper por padrão; OpenAI, Fireworks ou endpoint próprio), idioma por transcrição (padrão pt-BR). As duas faixas são intercaladas com rótulos "Você" / "Participantes".
- **Resumo da reunião** (opcional) com provedor selecionável (OpenAI, Claude, Gemini, MiniMax ou endpoint próprio).
- **Agenda (ICS)**: lista as próximas reuniões, checkbox por reunião ou "gravar todas"; gravação **inicia sozinha** no horário, alerta no fim previsto e para sozinha 1h depois do fim.
- **Ícone na bandeja** com start/stop e bolinha vermelha enquanto grava; fechar a janela minimiza para a bandeja.
- **Attio (CRM)**: encontra a reunião pelo horário da gravação, sugere os participantes e sobe a transcrição/resumo como nota em cada pessoa, vinculada à meeting.
- **Teste de chave** por provedor nas Configurações, mensagens de erro em linguagem simples e **log persistente** para troubleshooting.
- Apagar gravações, copiar transcrição/resumo.

## Instalação

Baixe o instalador do seu sistema em [Releases](https://github.com/GeanChu/hicorder/releases): `.msi`/`.exe` (Windows), `.dmg` (macOS), `.AppImage`/`.deb` (Linux).

### Aviso do antivírus / SmartScreen

Os instaladores ainda **não são assinados digitalmente** (assinatura via [SignPath Foundation](https://signpath.org/) está em andamento — ver [docs/SIGNING.md](docs/SIGNING.md)). Por isso o Windows SmartScreen ou o antivírus podem exibir um aviso na primeira execução. O que fazemos para minimizar isso:

- Código 100% aberto neste repositório; os binários são gerados por CI público (GitHub Actions) a partir deste código.
- Metadados completos no executável e instalador (editora, versão, descrição, site).
- Falsos positivos são reportados aos fornecedores (Kaspersky via opentip.kaspersky.com, Microsoft via portal de submissão).

Se o seu antivírus bloquear: adicione o Hicorder como aplicativo confiável ou baixe novamente após a assinatura digital estar ativa. No SmartScreen: "Mais informações" → "Executar assim mesmo".

## Chaves de API

Cada etapa de IA usa um provedor à sua escolha (Configurações → selects de provedor, com instrução de onde obter cada chave):

| Etapa | Provedores | Custo típico |
|---|---|---|
| Transcrição | Groq (padrão), OpenAI, Fireworks, personalizado | Groq tem free tier |
| Resumo | OpenAI, Claude, Gemini, MiniMax (API ou Subscription), personalizado | conforme o plano |
| CRM | Attio (Settings → Developers → API tokens) | — |

As chaves ficam no **keychain do sistema operacional**, nunca em texto puro.

## Como rodar (dev)

Pré-requisitos: Node 20+, Rust stable, dependências do Tauri 2 e `ffmpeg` no PATH (só em dev; no app final o ffmpeg vai embutido).

```bash
npm install
npm run tauri dev
```

Verificação sem gerar binário (útil com antivírus agressivo): `cargo check` em `src-tauri/`.

## Stack

- **Tauri 2** (backend Rust) + **Vite + React + TypeScript** (UI)
- **cpal** (mic) e **WASAPI loopback** (áudio do sistema no Windows); macOS/Linux no roadmap
- **ffmpeg** embutido como resource (mix/encode Opus)
- **SQLite** (gravações, transcrições, resumos, agenda) · **keyring** (chaves no keychain)
- Captura de áudio referenciada do projeto **meetily** (MIT) — veja [NOTICE](NOTICE)

## Documentação

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — módulos e desenho do sistema.
- [docs/ROADMAP.md](docs/ROADMAP.md) — o que está pronto e o que falta.
- [docs/DECISIONS.md](docs/DECISIONS.md) — decisões de arquitetura (ADRs).
- [docs/PROVIDERS.md](docs/PROVIDERS.md) — provedores de IA suportados e como configurar.
- [docs/SIGNING.md](docs/SIGNING.md) — assinatura de código (SignPath Foundation).
- [docs/CONTINUATION.md](docs/CONTINUATION.md) — estado atual e handoff de desenvolvimento.

## Privacidade

Áudio, transcrições e resumos ficam **locais** no computador do usuário (SQLite + arquivos). A transcrição/resumo envia o áudio/texto **apenas** para o provedor de IA que você configurar. O upload ao Attio só acontece quando você confirma. Nenhuma telemetria.

## Segurança

Vulnerabilidades: veja [SECURITY.md](SECURITY.md).

## Licença

MIT. Veja [LICENSE](LICENSE) e [NOTICE](NOTICE).
