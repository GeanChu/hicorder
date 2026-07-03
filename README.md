# Hicorder

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Hicorder" width="96" height="96" />
</p>

Gravador de reuniões para times. Grava o áudio do **microfone** e do **sistema** (a voz dos outros participantes), transcreve com IA, gera resumo e envia notas para o CRM (Attio). Foco em UX simples para usuários não técnicos. Desktop **Windows, macOS e Linux** (Tauri 2).

> *Hicorder is a meeting recorder for teams: it captures mic + system audio, transcribes with AI, summarizes, and pushes notes to the Attio CRM. Open source (MIT), built with Tauri 2 (Rust + React).*

## O que faz

- **Gravar reuniões** (mic + áudio do sistema) com um botão, medidor de nível e player embutido.
- **Áudio leve**: Opus ~32 kbps em `.webm` (~7–15 MB/hora).
- **Transcrever via IA** com provedor selecionável (Groq Whisper por padrão; OpenAI, Fireworks ou endpoint próprio), idioma por transcrição (padrão pt-BR). As faixas viram um **chat**: "Você" à direita, "Participantes" à esquerda.
- **Resumo da reunião** (opcional) com provedor selecionável (OpenAI, Claude, Gemini, MiniMax ou endpoint próprio).
- **Exportar áudio** em MP3, WAV ou OGG.
- **Agenda (ICS)** na tela principal: próximas reuniões com participantes, local e **link da call**; destaque da reunião que está acontecendo; **Iniciar Gravação** por reunião ou **Agendar Gravação** (auto-start no horário, alerta no fim, auto-stop em fim+1h) ou "gravar todas".
- **Autoinicialização** com o sistema (ligada por padrão; abre minimizada na bandeja para gravar sozinha) e **ícone na bandeja** com start/stop e bolinha vermelha.
- **Attio (CRM)**: encontra a reunião pelo horário da gravação, sugere os participantes e sobe a transcrição/resumo como nota em cada pessoa, vinculada à meeting.
- **Tema** claro, escuro ou automático (segue o sistema).
- **Teste de chave** por provedor, mensagens de erro em linguagem simples e **log persistente** (Configurações → Ver logs) para troubleshooting.
- Renomear, apagar e copiar transcrição/resumo.

## Download e instalação

Baixe sempre a versão mais recente na página de **[Releases](https://github.com/GeanChu/hicorder/releases/latest)** e escolha o arquivo do seu sistema.

### Windows
[**Baixar para Windows**](https://github.com/GeanChu/hicorder/releases/latest) — arquivo `Hicorder_x.y.z_x64-setup.exe` (instalador) ou `Hicorder_x.y.z_x64_en-US.msi`.

1. Execute o instalador e siga o assistente.
2. Se o **SmartScreen** avisar ("Windows protegeu seu PC"): clique em **Mais informações** → **Executar assim mesmo**.
3. Se o antivírus bloquear, veja [Aviso do antivírus](#aviso-do-antivírus--smartscreen) abaixo.

### macOS (Intel e Apple Silicon)
[**Baixar para macOS**](https://github.com/GeanChu/hicorder/releases/latest) — arquivo `Hicorder_x.y.z_universal.dmg` (universal).

1. Abra o `.dmg` e arraste o **Hicorder** para a pasta Aplicativos.
2. Na primeira vez (app ainda não notarizado): clique com o **botão direito** no Hicorder → **Abrir** → **Abrir**. Ou vá em **Ajustes do Sistema → Privacidade e Segurança → Abrir mesmo assim**.
3. Autorize a permissão de **microfone** quando pedida.

### Linux
[**Baixar para Linux**](https://github.com/GeanChu/hicorder/releases/latest) — `AppImage` (portátil), `.deb` (Debian/Ubuntu) ou `.rpm` (Fedora/RHEL).

```bash
# AppImage (portátil, não instala nada)
chmod +x Hicorder_*_amd64.AppImage && ./Hicorder_*_amd64.AppImage

# Debian / Ubuntu
sudo apt install ./Hicorder_*_amd64.deb

# Fedora / RHEL
sudo dnf install ./Hicorder-*.x86_64.rpm
```

> Áudio do sistema (a voz dos outros participantes) é capturado por enquanto **só no Windows**. No macOS e Linux a versão atual grava apenas o **microfone** — as demais funções (agenda, transcrição, resumo, CRM) funcionam normalmente.

### Aviso do antivírus / SmartScreen

Os instaladores ainda **não são assinados digitalmente** (assinatura via [SignPath Foundation](https://signpath.org/) está em andamento — ver [docs/SIGNING.md](docs/SIGNING.md)). Por isso o Windows SmartScreen ou o antivírus podem exibir um aviso na primeira execução. O que fazemos para minimizar isso:

- Código 100% aberto neste repositório; os binários são gerados por CI público (GitHub Actions) a partir deste código.
- Metadados completos no executável e instalador (editora, versão, descrição, site).
- Falsos positivos são reportados aos fornecedores (Kaspersky via opentip.kaspersky.com, Microsoft via portal de submissão).

Se o seu antivírus bloquear: adicione o Hicorder como aplicativo confiável ou baixe novamente após a assinatura digital estar ativa. No SmartScreen: "Mais informações" → "Executar assim mesmo".

A assinatura de código dos executáveis Windows é feita pelo **[SignPath Foundation](https://signpath.org/)** (programa gratuito de assinatura para projetos open source). *Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org/).*

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
