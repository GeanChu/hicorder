# Hicorder

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Hicorder" width="96" height="96" />
</p>

<p align="center">
  <strong>Gravador de reuniões open source com transcrição por IA e integração com CRM</strong>
</p>

<p align="center">
  <a href="https://github.com/GeanChu/hicorder/releases/latest">
    <img alt="GitHub release" src="https://img.shields.io/github/v/release/GeanChu/hicorder">
  </a>
  <a href="https://github.com/GeanChu/hicorder/blob/main/LICENSE">
    <img alt="License" src="https://img.shields.io/github/license/GeanChu/hicorder">
  </a>
  <a href="https://github.com/GeanChu/hicorder/actions">
    <img alt="CI" src="https://img.shields.io/github/actions/workflow/status/GeanChu/hicorder/ci.yml">
  </a>
</p>

<p align="center">
  <a href="README.md">English</a> · <a href="#documentação">Documentação</a> · <a href="CONTRIBUTING.md">Como Contribuir</a>
</p>

---

## O que é o Hicorder?

Hicorder é um gravador de reuniões desktop feito para times que precisam de anotações de reuniões flexíveis, privadas e poderosas. Ele captura o áudio do microfone e do sistema, transcreve com IA, gera resumos e envia notas direto para o seu CRM.

Nascido como alternativa ao Granola, o Hicorder te dá controle completo sobre seus provedores de IA, prompts e dados. Criado originalmente na [Hi Capital](https://hi.capital) (Honey Island VC, Curitiba, Brasil) por Gean Chu, agora é open source para a comunidade.

**O que torna o Hicorder diferente:**
- **Escolha sua IA**: Funciona com Groq, OpenAI, Claude, Gemini, MiniMax, NVIDIA NIM ou seus próprios endpoints
- **Prompts customizáveis**: Edite o prompt base e crie uma biblioteca de prompts nomeados para diferentes tipos de reunião
- **Integração nativa com CRM**: Envie transcrições e resumos para Attio ou Affinity com matching de participantes
- **Privacidade em primeiro lugar**: Todos os dados ficam locais; só os provedores de IA que você escolher veem seu áudio/transcrições
- **Foco em Venture Capital**: Dicionário de terminologia integrado para reuniões de VC/investimentos, facilmente extensível

## Funcionalidades Principais

### Gravação e Áudio
- **Gravação com um clique** com medidor de nível de áudio ao vivo e player integrado
- **Captura em duas faixas**: Grava microfone e áudio do sistema (vozes dos participantes) separadamente
- **Armazenamento leve**: Codificação Opus a ~32 kbps (~7-15 MB/hora), codificado durante a gravação para que travamentos não percam dados
- **Anotações ao vivo**: Painel lateral durante a gravação para anotações manuais que enriquecem o resumo depois
- **Exportar áudio**: Formatos MP3, WAV ou OGG
- **Upload de áudio**: Transcreva gravações existentes

### Transcrição por IA
- **Flexibilidade de provedor**: Escolha Groq Whisper (padrão, tem free tier), OpenAI, Fireworks AI ou endpoints customizados
- **Idioma por transcrição**: Padrão português (Brasil), configurável por gravação
- **Formato de chat**: Microfone aparece como "Você" (lado direito), áudio do sistema como "Participantes" (lado esquerdo)
- **Dicionário de terminologia**: Pré-carregado com 40 termos de VC/investimentos; adicione seus próprios nomes, siglas, jargões
- **Filtro de alucinação**: Remove automaticamente artefatos de preenchimento de silêncio do Whisper
- **Busca**: Busca de texto completo dentro das transcrições

### Resumo por IA
- **Suporte a múltiplos provedores**: OpenAI, Claude, Gemini, MiniMax, NVIDIA NIM ou seus próprios endpoints
- **Integração com Claude Code**: Use o Claude instalado localmente sem chave de API (usa sua assinatura Claude)
- **Prompt base editável**: Customize as instruções padrão de resumo
- **Biblioteca de prompts**: Crie prompts nomeados para diferentes tipos de reunião (updates para investidores, 1-on-1s, etc.)
- **Integração de notas manuais**: Suas anotações ao vivo são incluídas no prompt do resumo
- **Busca**: Busca de texto completo dentro dos resumos

### Calendário e Automação
- **Integração com calendário ICS**: Veja próximas reuniões com participantes, local e links de chamada
- **Destaque de reunião atual**: Indicador visual para reuniões acontecendo agora
- **Início com um clique**: Botão "Iniciar Gravação" por reunião, ou "Agendar Gravação" para auto-início
- **Lembrete de gravação esquecida**: Lembrete de hora em hora com botão Parar
- **Auto-stop**: Auto-stop configurável (padrão 2 horas) previne gravações descontroladas
- **Bandeja do sistema**: Ícone sempre visível na bandeja com indicador de gravação

### Integração com CRM
- **Suporte a Attio e Affinity**: Envie transcrições, resumos ou notas como notas do CRM
- **Matching de reuniões por horário**: Encontra a reunião do CRM pelo timestamp da gravação
- **Sugestões de participantes**: Sugere automaticamente participantes e suas empresas baseado nos dados da reunião
- **Criação de notas flexível**: Escolha o que fazer upload e para quais participantes/empresas

### Qualidade de Vida
- **Auto-atualização**: Verifica atualizações ao iniciar e a cada 24 horas
- **Auto-início com o SO**: Inicia minimizado na bandeja (habilitado por padrão)
- **Temas**: Claro, escuro ou automático (segue o sistema)
- **Logs persistentes**: Logs de troubleshooting acessíveis pelas Configurações
- **Teste suas chaves**: Botão de teste integrado para cada provedor de API
- **Mensagens de erro amigáveis**: Erros em linguagem simples com dicas de troubleshooting

## Suporte a Plataformas

| Plataforma | Microfone | Áudio do Sistema | Notas |
|----------|-----------|--------------|-------|
| **Windows** | ✅ | ✅ | WASAPI loopback |
| **macOS** | ✅ | ✅ | ScreenCaptureKit (macOS 13+, requer permissão de Gravação de Tela) |
| **Linux** | ✅ | ⏳ | Áudio do sistema em breve (monitor PulseAudio) |

## Instalação

Baixe a versão mais recente para seu sistema operacional na [página de Releases](https://github.com/GeanChu/hicorder/releases/latest).

### Windows

Baixe `Hicorder_x.y.z_x64-setup.exe` (instalador NSIS) ou `Hicorder_x.y.z_x64_en-US.msi`.

1. Execute o instalador e siga o assistente.
2. Se o **SmartScreen do Windows** avisar ("O Windows protegeu seu PC"): clique em **Mais informações** → **Executar assim mesmo**.
3. Se seu antivírus bloquear, veja [Avisos de Antivírus](#avisos-de-antivírus) abaixo.

### macOS (Intel e Apple Silicon)

Baixe `Hicorder_x.y.z_universal.dmg` (binário universal).

1. Abra o `.dmg` e arraste o **Hicorder** para a pasta Aplicativos.
2. Na primeira execução (app ainda não notarizado): clique com o **botão direito** no Hicorder → **Abrir** → **Abrir**. Ou vá em **Ajustes do Sistema → Privacidade e Segurança → Abrir mesmo assim**.
3. Se o macOS bloquear ("não é possível verificar" ou "está danificado"), rode no **Terminal** e abra de novo:
   ```bash
   xattr -dr com.apple.quarantine /Applications/Hicorder.app
   # Se ainda disser "danificado" (Apple Silicon):
   codesign --force --deep -s - /Applications/Hicorder.app
   ```
4. Autorize a permissão de **Microfone** quando pedida.
5. Para gravar o **áudio da call** (vozes dos participantes), o macOS exige permissão de **Gravação de Tela**: ao gravar pela primeira vez, vá em **Ajustes do Sistema → Privacidade e Segurança → Gravação de Tela**, ative o **Hicorder** e **reinicie o app**. Sem isso, grava só o microfone.

### Linux

Baixe `AppImage` (portátil), `.deb` (Debian/Ubuntu) ou `.rpm` (Fedora/RHEL).

```bash
# AppImage (portátil, não instala nada)
chmod +x Hicorder_*_amd64.AppImage && ./Hicorder_*_amd64.AppImage

# Debian / Ubuntu
sudo apt install ./Hicorder_*_amd64.deb

# Fedora / RHEL
sudo dnf install ./Hicorder-*.x86_64.rpm
```

> **Nota:** Linux atualmente grava apenas o microfone. Áudio do sistema (monitor PulseAudio) está no roadmap. Todas as outras funcionalidades (calendário, transcrição, resumo, CRM) funcionam normalmente.

### Avisos de Antivírus

Os instaladores ainda **não são assinados digitalmente** (assinatura de código via [SignPath Foundation](https://signpath.org/) está em andamento — veja [docs/SIGNING.md](docs/SIGNING.md)). O SmartScreen do Windows ou software antivírus podem mostrar um aviso na primeira execução. Aqui está o que fazemos para minimizar isso:

- Código 100% open source neste repositório; binários são construídos por CI público (GitHub Actions) a partir deste código.
- Metadados completos embutidos em executáveis e instaladores (editora, versão, descrição, site).
- Falsos positivos são reportados aos fornecedores (Kaspersky via opentip.kaspersky.com, Microsoft via portal de submissão).

Se seu antivírus bloquear: adicione o Hicorder como aplicativo confiável ou baixe novamente após a assinatura de código estar ativa. Para SmartScreen: "Mais informações" → "Executar assim mesmo".

A assinatura de código para executáveis Windows é fornecida pela **[SignPath Foundation](https://signpath.org/)** (programa gratuito de assinatura para projetos open source). *Assinatura de código gratuita fornecida por [SignPath.io](https://signpath.io), certificado por [SignPath Foundation](https://signpath.org/).*

## Início Rápido

1. **Instale o Hicorder** e inicie.
2. **Configure seus provedores de IA** nas Configurações:
   - **Transcrição**: Obtenha uma chave de API do [Groq](https://console.groq.com/keys) (tem free tier), [OpenAI](https://platform.openai.com/api-keys) ou [Fireworks AI](https://fireworks.ai).
   - **Resumo**: Obtenha uma chave de API do seu provedor de LLM preferido (OpenAI, Claude, Gemini, etc.) ou use [Claude Code](https://www.anthropic.com/claude/download) localmente (sem chave necessária).
   - Teste cada chave com o botão "Testar".
3. **(Opcional) Conecte seu calendário**: Cole sua URL ICS privada em Configurações → Conexões → Calendário. [Como obter sua URL ICS](https://support.google.com/calendar/answer/37648).
4. **(Opcional) Conecte seu CRM**: Adicione sua chave de API do Attio ou Affinity em Configurações → Conexões → CRM.
5. **Comece a gravar**: Clique no grande botão vermelho na aba Home, ou clique em "Iniciar Gravação" ao lado de uma reunião próxima.
6. **Transcreva**: Depois de parar, vá para a aba Gravações, selecione sua gravação e clique em "Transcrever".
7. **Resuma**: Depois que a transcrição completar, clique em "Gerar Resumo".
8. **(Opcional) Envie para o CRM**: Clique em "Subir para o CRM", selecione a reunião, escolha os participantes e confirme.

## Comparação com o Granola

O Hicorder foi criado como alternativa ao Granola. Aqui está como eles se comparam:

| Funcionalidade | Hicorder | Granola |
|---------|----------|---------|
| **Open Source** | ✅ Sim (MIT) | ❌ Não |
| **Preço** | Grátis (você fornece as chaves de API) | Freemium (tier grátis limitado, planos pagos) |
| **Escolha de Provedor de IA** | Qualquer endpoint compatível com OpenAI | Gerenciado pelo Granola |
| **Prompts Customizados** | ✅ Base editável + biblioteca de prompts | Customização limitada |
| **Integrações de CRM** | Attio, Affinity (extensível) | Múltiplos (proprietários) |
| **Privacidade de Dados** | 100% local, você escolhe os provedores | Cloud-first |
| **Foco em Venture Capital** | Sim (dicionário de VC integrado) | Negócios em geral |
| **Plataforma** | Desktop (Windows, macOS, Linux) | Desktop + mobile |
| **Áudio do Sistema** | Nativo (Windows/macOS) | Nativo |
| **Detecção de Reuniões** | Calendário ICS | Calendário + detecção de app |

**Quando escolher o Hicorder:**
- Você quer controle sobre seus provedores de IA e custos
- Você precisa de prompts customizáveis para tipos específicos de reunião
- Você prefere armazenamento de dados local-first
- Você está em Venture Capital ou quer integrações de CRM extensíveis

**Quando escolher o Granola:**
- Você prefere um serviço gerenciado sem setup
- Você quer apps mobile
- Você precisa de integrações que o Hicorder ainda não suporta

## Arquitetura

O Hicorder é construído com [Tauri 2](https://tauri.app) (backend Rust + frontend React):

- **Captura de áudio**: `cpal` para microfone, WASAPI loopback (Windows), ScreenCaptureKit (macOS)
- **Codificação de áudio**: `ffmpeg` embutido para codificação Opus/Ogg durante a gravação (~32 kbps, 16 kHz)
- **Armazenamento**: SQLite para gravações, transcrições, resumos, notas, prompts, reuniões
- **Segurança**: Chaves de API armazenadas no keychain do SO (Windows/Linux) ou arquivo protegido (macOS)
- **Rede**: Reqwest com TLS nativo, resolver DNS IPv4-only, timeouts explícitos

Veja [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para detalhes.

## Documentação

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) — Design do sistema e breakdown de módulos
- [ROADMAP.md](docs/ROADMAP.md) — O que está pronto e o que está vindo
- [DECISIONS.md](docs/DECISIONS.md) — Registros de Decisões de Arquitetura (ADRs)
- [PROVIDERS.md](docs/PROVIDERS.md) — Provedores de IA suportados e configuração
- [SIGNING.md](docs/SIGNING.md) — Setup de assinatura de código (SignPath Foundation)
- [CONTINUATION.md](docs/CONTINUATION.md) — Estado atual de desenvolvimento e notas de handoff

## Setup de Desenvolvimento

Veja [CONTRIBUTING.md](CONTRIBUTING.md) para instruções detalhadas de desenvolvimento.

**Início rápido:**

Pré-requisitos:
- Node.js 20+
- Rust stable (com MSVC no Windows: Visual Studio Build Tools com C++ e Windows SDK)
- Dependências de sistema do Tauri 2: [Guia de instalação](https://v2.tauri.app/start/prerequisites/)
- `ffmpeg` no PATH (só em dev; produção o embute)

```bash
git clone https://github.com/GeanChu/hicorder.git
cd hicorder
npm install
npm run tauri dev
```

Build sem empacotamento (para teste):
```bash
npm run tauri build -- --no-bundle
```

Rodar testes:
```bash
cd src-tauri
cargo test
```

## Privacidade

- **Local-first**: Áudio, transcrições e resumos são armazenados localmente no seu computador (SQLite + arquivos).
- **Provedores de IA**: Transcrição e resumo enviam áudio/texto **apenas** para o provedor de IA que você configurar. Recomendamos ler a política de privacidade de cada provedor.
- **Upload para CRM**: Acontece apenas quando você confirma explicitamente o upload.
- **Sem telemetria**: O Hicorder não coleta dados de uso, analytics ou relatórios de crash.
- **URL do calendário**: Sua URL ICS é uma credencial (qualquer um com ela pode ler seu calendário). Ela é armazenada localmente e nunca enviada para lugar nenhum exceto para buscar os dados do calendário.

## Segurança

Levamos segurança a sério. Se você descobrir uma vulnerabilidade, por favor reporte de forma responsável:

- **Preferido**: Envie um relatório de vulnerabilidade privado via [GitHub Security Advisories](https://github.com/GeanChu/hicorder/security/advisories/new)
- **Alternativa**: Email **financeiro@hi.capital**
- **Não** abra issues públicas para relatórios de segurança

Veja [SECURITY.md](SECURITY.md) para detalhes.

## Roadmap e Como Contribuir

Veja [ROADMAP.md](docs/ROADMAP.md) para funcionalidades planejadas e [CONTRIBUTING.md](CONTRIBUTING.md) para como contribuir.

**Áreas onde a comunidade pode ajudar:**
- **Áudio do sistema no Linux** (fonte de monitor PulseAudio)
- **Novas integrações de CRM** (veja [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) para a interface de integração)
- **Novos provedores de IA** (endpoints compatíveis com OpenAI são drop-in)
- **Traduções** (atualmente inglês e português; UI é localizável)
- **Notarização do macOS** (requer conta Apple Developer)
- **Melhorias de documentação**
- **Relatórios de bugs e pedidos de funcionalidades**

## Licença

Licença MIT. Veja [LICENSE](LICENSE) e [NOTICE](NOTICE).

Este projeto reutiliza e adapta código do [meetily](https://github.com/Zackriya-Solutions/meetily) (MIT), especificamente o pipeline de captura de áudio cross-platform.

## Agradecimentos

- Construído com [Tauri](https://tauri.app), [React](https://react.dev) e [Rust](https://www.rust-lang.org)
- Captura de áudio inspirada em [meetily](https://github.com/Zackriya-Solutions/meetily)
- Assinatura de código gratuita pela [SignPath Foundation](https://signpath.org/)

## Links

- [Website](https://hi.capital)
- [GitHub](https://github.com/GeanChu/hicorder)
- [Issues](https://github.com/GeanChu/hicorder/issues)
- [Releases](https://github.com/GeanChu/hicorder/releases)
- [Discussão](https://github.com/GeanChu/hicorder/discussions)

---

Feito com ❤️ por [Gean Chu](https://github.com/GeanChu) e [Hi Capital](https://hi.capital) • Open source para a comunidade
