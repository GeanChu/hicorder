# Relatório de Preparação para Open Source - Hicorder

**Data:** 16 de outubro de 2026  
**Branch:** cursor/open-source-prep-8994  
**Autor da auditoria:** Cloud Agent (Cursor)  
**Solicitante:** Gean Chu (GeanChu), Hi Capital

---

## Sumário Executivo

O projeto Hicorder foi auditado e preparado para publicação como projeto open source. A auditoria cobriu segurança, privacidade, licenciamento, documentação e arquivos de comunidade. O projeto está pronto para ser tornado público após a revisão das decisões de política documentadas abaixo.

**Status:** ✅ Pronto para revisão e publicação (após decisões de política)

---

## O Que é o Hicorder Hoje

Hicorder é um gravador de reuniões desktop multiplataforma (Windows, macOS, Linux) construído com Tauri 2 (Rust + React). O projeto nasceu na Hi Capital (Honey Island VC, Curitiba) como alternativa ao Granola, oferecendo flexibilidade total para escolher modelos de IA, prompts customizáveis e integração nativa com CRMs usados em Venture Capital.

**Funcionalidades implementadas:**
- Gravação de áudio do microfone e do sistema (vozes dos participantes) em faixas separadas
- Codificação ao vivo em Opus/Ogg (~32 kbps) para armazenamento leve
- Anotações manuais durante a gravação
- Transcrição via IA com múltiplos provedores (Groq, OpenAI, Fireworks, endpoints customizados)
- Formato de chat com "Você" e "Participantes" identificados
- Dicionário de terminologia (40 termos de VC pré-carregados, editável)
- Filtro de alucinações do Whisper
- Resumo via LLM com múltiplos provedores (OpenAI, Claude, Gemini, MiniMax, NVIDIA NIM)
- Integração com Claude Code local (sem necessidade de chave de API)
- Prompt base editável e biblioteca de prompts nomeados
- Integração com CRMs: Attio e Affinity
- Calendário ICS com suporte a RRULE (eventos recorrentes)
- Auto-início de gravação por reunião agendada
- Lembretes e auto-stop configurável
- Tema claro/escuro
- Auto-atualização
- Logs persistentes com redação de segredos

---

## Auditoria de Segurança e Privacidade

### ✅ Segredos e Credenciais

**Status:** Nenhum segredo real encontrado no código ou histórico do Git.

**Análise:**
- ❌ Nenhuma chave de API real (sk-, nvapi-, gsk_, etc.) encontrada
- ❌ Nenhum token ou senha encontrado
- ✅ Exemplos de chaves nos comentários (gean@hi.capital, exemplo) são placeholders legítimos
- ✅ Sistema de redação de segredos já implementado em `logs/mod.rs`
- ✅ Chaves armazenadas no keychain do SO ou arquivo protegido 0600 no macOS

**Locais verificados:**
- Código-fonte (Rust e TypeScript)
- Arquivos de configuração (.github, tauri.conf.json)
- Histórico do Git (147 commits examinados)
- Variáveis de ambiente (.env não existe, criado .env.example)

### ⚠️ Dados Pessoais e de Clientes

**Status:** Dados de teste encontrados, nenhum dado real de cliente identificado.

**Dados encontrados (legítimos para o projeto):**
- `gean@hi.capital` - Email do autor (usado em testes unitários e documentação de contato)
- `financeiro@hi.capital` - Email de contato para segurança (legítimo)
- Referências a "hi.capital" como autor/copyright (legítimo)
- Exemplos em testes: `voce@hi.capital`, `outro@hi.capital` (dados de teste, não reais)

**Dados NÃO encontrados:**
- ❌ Nomes ou emails de clientes reais
- ❌ Gravações de reuniões reais
- ❌ Transcrições ou resumos de reuniões reais
- ❌ URLs de calendário ICS privadas
- ❌ Dados de CRM (reuniões, contatos, empresas)

**Recomendação:** ✅ Seguro para publicação. Os dados encontrados são apropriados para um projeto open source.

### ✅ Histórico do Git

**Status:** Histórico limpo, sem necessidade de reescrita.

**Análise:**
- 147 commits analisados
- Commits de desenvolvimento normais, sem evidência de commit de segredos
- Mensagens de commit apropriadas
- Nenhum arquivo .env commitado no histórico
- Arquivos sensíveis sempre estiveram no .gitignore

**Recomendação:** ✅ Não é necessário reescrever o histórico ou criar um repositório limpo.

### ✅ Arquivos de Configuração

**Status:** Sem credenciais ou dados sensíveis.

**Arquivos analisados:**
- `.gitignore` - Apropriado (ignora node_modules, target, .env, etc.)
- `tauri.conf.json` - Sem segredos
- `package.json` / `Cargo.toml` - Sem segredos
- `.github/workflows/` - Referência a secrets do GitHub (TAURI_SIGNING_PRIVATE_KEY, SIGNPATH_API_TOKEN) que são gerenciados corretamente pelo GitHub Secrets

**Novo arquivo criado:**
- `.env.example` - Template documentado com todas as variáveis necessárias

---

## Auditoria de Dependências e Licenciamento

### ✅ Licença do Projeto

**Status:** MIT (Apache 2.0 recomendada como alternativa no relatório de decisões).

**Análise atual:**
- LICENSE já existe e está correto (MIT, copyright hi.capital 2026)
- NOTICE existe e documenta a reutilização de código do meetily (MIT)
- Licença MIT é permissiva e compatível com o objetivo de adoção da comunidade

**Recomendações de licença (para decisão do Gean):**

1. **MIT (atual)** ✅ Recomendada para máxima adoção
   - Mais permissiva
   - Permite uso comercial sem restrições
   - Favorece adoção rápida pela comunidade
   - Permite que empresas criem versões proprietárias fechadas
   - **Melhor para:** Crescimento rápido, ecossistema amplo

2. **Apache 2.0** ⚠️ Alternativa equilibrada
   - Permissiva como MIT
   - Proteção contra patentes explícita
   - Exige preservação de notices e changelog
   - Permite uso comercial mas com mais clareza jurídica
   - **Melhor para:** Projetos com potencial de disputas de patentes

3. **AGPL-3.0** ❌ Não recomendada
   - Copyleft forte: quem usar em SaaS deve abrir o código
   - Protege contra uso comercial fechado
   - **Porém:** Reduz drasticamente a adoção da comunidade
   - Empresas evitam AGPL por receio de "contaminação"
   - **Evitar** se o objetivo é maximizar contribuições e uso

**Decisão sugerida:** Manter MIT. A missão do projeto é democratizar a gravação de reuniões com IA, e isso é melhor servido por uma licença permissiva que maximiza adoção. Se houver preocupação com SaaS competitivos fechados, essa pode ser endereçada através de marca registrada ("Hicorder") e diferenciação técnica, não através de licença restritiva.

### ✅ Compatibilidade de Dependências

**Status:** Todas as dependências são compatíveis com MIT.

**Principais dependências frontend (npm):**
- @tauri-apps/* - MIT/Apache-2.0
- react, react-dom - MIT
- vite - MIT
- typescript - Apache-2.0

**Principais dependências backend (Rust):**
- tauri - MIT/Apache-2.0
- serde, serde_json - MIT/Apache-2.0
- anyhow - MIT/Apache-2.0
- cpal - Apache-2.0
- rusqlite - MIT
- reqwest - MIT/Apache-2.0
- chrono - MIT/Apache-2.0

**Reutilização de código:**
- meetily (MIT) - Audio capture pipeline adaptado, já documentado em NOTICE

**Conclusão:** ✅ Todas as licenças são permissivas e compatíveis com MIT ou Apache-2.0.

---

## Documentação Criada/Atualizada

### ✅ Arquivos Principais

1. **README.md** (novo, inglês)
   - Descrição completa do projeto
   - Comparação honesta com Granola
   - Guia de instalação por plataforma
   - Quick start
   - Arquitetura em alto nível
   - Links para documentação detalhada

2. **README.pt-BR.md** (novo)
   - Tradução completa do README em português do Brasil

3. **CONTRIBUTING.md** (novo)
   - Setup de desenvolvimento completo
   - Estrutura do projeto
   - Padrões de código (Rust e TypeScript)
   - Processo de submissão de PRs
   - Guias para adicionar novos provedores de IA
   - Guias para adicionar novas integrações de CRM
   - Guidelines de testes

4. **.env.example** (novo)
   - Template com todas as variáveis de ambiente
   - Documentação inline de onde obter chaves de API
   - Avisos de segurança (URL ICS é credencial)

5. **CODE_OF_CONDUCT.md** (novo)
   - Código de conduta curto e objetivo em português
   - Baseado no Contributor Covenant 2.1
   - Contato: gean@hi.capital

6. **GOVERNANCE.md** (novo)
   - Modelo de governança do projeto
   - Gean Chu como maintainer principal
   - Processo de decisão
   - Como virar maintainer
   - Resolução de conflitos

7. **SECURITY.md** (já existia, revisado)
   - Política de reporte de vulnerabilidades
   - Escopo de segurança
   - Contato: financeiro@hi.capital

### ✅ Templates do GitHub

1. **.github/ISSUE_TEMPLATE/bug_report.md**
   - Template estruturado para bug reports
   - Campos para informações de sistema, logs, screenshots

2. **.github/ISSUE_TEMPLATE/feature_request.md**
   - Template para pedidos de funcionalidades
   - Campos para problema, solução proposta, casos de uso

3. **.github/ISSUE_TEMPLATE/crm_integration.md**
   - Template específico para pedidos de integração de CRM
   - Checklist de capacidades da API
   - Campos para documentação e disponibilidade de teste

4. **.github/PULL_REQUEST_TEMPLATE.md**
   - Checklist completo para PRs
   - Campos para tipo de mudança, testes, screenshots
   - Verificações de qualidade

5. **.github/CODEOWNERS**
   - Gean Chu como owner padrão
   - Ownership específico para docs, CI, arquivos de segurança

6. **.github/dependabot.yml**
   - Configurado para npm, cargo e github-actions
   - Updates semanais
   - Labels automáticas

### ✅ Documentação Existente (revisada)

Arquivos em `docs/` já existiam e estão bem documentados:
- ARCHITECTURE.md - Desenho do sistema detalhado
- ROADMAP.md - Histórico de desenvolvimento PR a PR
- DECISIONS.md - ADRs com justificativas técnicas
- PROVIDERS.md - Provedores de IA suportados
- SIGNING.md - Processo de assinatura de código
- CONTINUATION.md - Estado atual para handoff entre sessões

### ⚠️ Metadata do Repositório (para configurar no GitHub)

**Descrição curta sugerida (1 linha):**
```
Open-source meeting recorder with AI transcription and CRM integration for Venture Capital
```

**Topics sugeridas:**
```
meeting-recorder, transcription, ai, crm, venture-capital, tauri, rust, react, 
typescript, desktop-app, speech-to-text, llm, attio, affinity, whisper, groq
```

**Social Preview:**
- Usar o ícone do projeto (src-tauri/icons/icon.png) com o texto "Hicorder - Open Source Meeting Recorder"
- Fundo profissional (azul/roxo do tema do app)

**About section no GitHub:**
- Website: https://hi.capital
- Topics: (lista acima)
- Check: ✅ Releases
- Check: ✅ Packages (se publicar no futuro)

---

## CI/CD

### ✅ GitHub Actions Existente

O projeto já tem CI completo:

1. **.github/workflows/ci.yml**
   - Build e teste em 3 plataformas (ubuntu-22.04, macos-15, windows-latest)
   - Testes Rust incluídos
   - Xcode 26 fixado no macOS para dependências

2. **.github/workflows/release.yml**
   - Build automático de instaladores em tags v*
   - Download de ffmpeg com múltiplos mirrors e retry
   - Assinatura de updates (TAURI_SIGNING_PRIVATE_KEY)
   - Integração com SignPath Foundation (preparada, aguardando aprovação)
   - Cria draft release no GitHub

**Status:** ✅ CI funcional, nenhuma mudança necessária para open source.

**Secrets necessários (já configurados):**
- `TAURI_SIGNING_PRIVATE_KEY` - Para assinatura de updates
- `SIGNPATH_API_TOKEN` - Para assinatura de código (quando aprovado)

---

## Decisões de Política para o Gean

O projeto está tecnicamente pronto, mas algumas decisões de negócio precisam ser tomadas antes do lançamento público:

### 1. Licença ✅

**Situação atual:** MIT  
**Recomendação:** Manter MIT  
**Alternativas:** Apache 2.0 (se houver preocupação com patentes), AGPL-3.0 (não recomendado)  
**Decisão necessária:** Confirmar MIT ou mudar para Apache 2.0  
**Ação:** Se mudar, atualizar LICENSE e headers nos arquivos

### 2. Marca e Nome "Hicorder" ⚠️

**Decisão necessária:** 
- O nome "Hicorder" será marca registrada?
- Terceiros podem criar forks e continuar usando o nome?
- Deve haver uma política de trademark (ex: "Hicorder® é marca registrada da Hi Capital, forks devem usar nome diferente")?

**Recomendação:** Permitir uso do nome para forks não-comerciais, exigir nome diferente para produtos comerciais ou SaaS. Documentar no README se houver restrições.

**Ação:** Se houver política de marca, adicionar seção "Trademark" no README.

### 3. CLA vs DCO ⚠️

**Decisão necessária:** Exigir Contributor License Agreement (CLA) ou Developer Certificate of Origin (DCO)?

**Opções:**
- **Nenhum (atual):** Mais fácil para contribuidores, MIT já é permissivo
- **DCO:** Contribuidores atestam que têm direito de contribuir (git commit -s)
- **CLA:** Contribuidores transferem/licenciam direitos para Hi Capital (mais formal, mais fricção)

**Recomendação:** Nenhum CLA/DCO inicialmente. MIT já protege suficientemente. Se crescer muito, considerar DCO.

**Ação:** Se optar por DCO, configurar bot no GitHub e atualizar CONTRIBUTING.md.

### 4. Contribuições de CRMs Proprietários ⚠️

**Decisão necessária:** Aceitar integrações com CRMs que não têm API pública ou trial gratuito?

**Cenário:** Alguém quer contribuir integração com CRM Enterprise que só funciona com conta paga.

**Opções:**
- Aceitar qualquer integração (mais features)
- Aceitar apenas CRMs com API pública testável (mais qualidade)
- Aceitar com disclaimer de "não testado pela equipe principal"

**Recomendação:** Aceitar qualquer integração, marcar integrações não testadas no README.

**Ação:** Documentar política em CONTRIBUTING.md.

### 5. Telemetria/Analytics ✅

**Situação atual:** Nenhuma telemetria  
**Decisão necessária:** Adicionar telemetria opt-in no futuro?

**Recomendação:** Manter sem telemetria. A privacidade-first é um diferencial do projeto.

**Ação:** Se mudar no futuro, disclosure completo e opt-in explícito, nunca opt-out.

### 6. Versão Hospedada ou Paga 💰

**Decisão necessária:** Hi Capital planeja oferecer versão hospedada (SaaS) ou suporte pago?

**Implicações:**
- **Se sim:** MIT permite que outros também façam isso (concorrência)
- **Se sim:** Considerar modelo "open core" (features enterprise fechadas)
- **Se sim:** Documentar no README as diferenças entre versão community e paga

**Recomendação:** Se houver planos de monetização, documentar de forma transparente. Comunidade valoriza honestidade.

**Ação:** Se houver versão paga futura, adicionar seção "Pricing & Hosting" no README.

### 7. Política de Dados e Privacidade 📋

**Situação atual:** Seção "Privacy" no README documenta que dados são locais  
**Decisão necessária:** É suficiente ou precisa de Privacy Policy formal?

**Recomendação:** Atual é suficiente (não é SaaS, não coleta dados). Se adicionar telemetria ou versão hospedada, criar privacy policy formal.

**Ação:** Nenhuma agora. Revisitar se escopo mudar.

### 8. Política de Segurança e Divulgação 📋

**Situação atual:** SECURITY.md com processo de reporte (GitHub Security Advisories ou financeiro@hi.capital)  
**Decisão necessária:** Definir SLA de resposta?

**Recomendação:** 
- Confirmar recebimento em 7 dias (já documentado)
- Fix em 30 dias para vulnerabilidades críticas
- Fix em 90 dias para não-críticas
- Disclosure pública após fix + 7 dias de grace period

**Ação:** Documentar SLA no SECURITY.md se concordar.

### 9. Revisão de PRs e SLA ⏱️

**Decisão necessária:** Quanto tempo para revisar PRs? Quem pode fazer merge?

**Situação atual:** Gean é o único maintainer  
**Recomendação:**
- Primeira resposta em PRs: 7 dias (confirmar que viu)
- Review completo: 14 dias
- Sem SLA garantido (projeto comunitário), mas esforço razoável (reasonable effort)

**Ação:** Documentar expectativas em CONTRIBUTING.md.

### 10. Credenciais de CRM e Consentimento de Gravação ⚠️

**Decisão necessária:** Avisar usuários sobre:
- Gravação pode precisar de consentimento dos participantes (leis variam por jurisdição)
- Chaves de CRM dão acesso amplo aos dados do CRM

**Recomendação:** 
- Adicionar disclaimer no README sobre leis de gravação
- Adicionar aviso na primeira gravação (modal no app)
- Documentar escopos mínimos de permissões para cada CRM

**Ação:** 
- Adicionar seção "Legal Considerations" no README
- Issue para implementar modal de aviso no app

---

## Checklist de Lançamento

Use esta lista ao preparar o lançamento público:

### Antes de Tornar o Repo Público

- [x] Auditoria de segurança completa
- [x] README em inglês e português
- [x] CONTRIBUTING.md
- [x] CODE_OF_CONDUCT.md
- [x] GOVERNANCE.md
- [x] Templates de issue e PR
- [x] CODEOWNERS
- [x] Dependabot configurado
- [x] .env.example criado
- [ ] Revisar e decidir questões de política (seções acima)
- [ ] Atualizar metadata do repositório (descrição, topics, social preview)
- [ ] Confirmar que SECURITY.md tem o contato correto
- [ ] Criar branch `main` protegida (require PR reviews, require status checks)
- [ ] Testar build em GitHub Actions (tag de teste)

### No Dia do Lançamento

- [ ] Tornar repositório público no GitHub (Settings → Danger Zone → Change visibility)
- [ ] Criar release v0.3.0 (ou próxima versão) com notas de lançamento
- [ ] Publicar instaladores na release
- [ ] Post no LinkedIn/Twitter anunciando (Hi Capital e Gean pessoal)
- [ ] Criar GitHub Discussion "Welcome / Boas-vindas"
- [ ] Configurar GitHub Projects para roadmap público (opcional)
- [ ] Adicionar badge de "open source" no site da Hi Capital (opcional)

### Primeira Semana

- [ ] Responder issues e PRs prontamente (primeira impressão importa)
- [ ] Criar labels no GitHub (bug, enhancement, good first issue, help wanted)
- [ ] Marcar issues simples como "good first issue" para atrair novos contribuidores
- [ ] Configurar GitHub Sponsors se quiser receber doações (opcional)
- [ ] Submeter para agregadores (awesome lists, alternativeto.net, etc)

### Primeira Mês

- [ ] Revisar e merge primeiros PRs da comunidade
- [ ] Criar CHANGELOG.md com histórico de versões
- [ ] Considerar criar Discord/Slack para comunidade (se demanda existir)
- [ ] Solicitar code signing no SignPath Foundation (se ainda não aprovado)
- [ ] Avaliar feedback e ajustar roadmap

---

## Riscos Identificados e Mitigações

### Risco: Baixa Adoção Inicial

**Probabilidade:** Média  
**Impacto:** Médio  
**Mitigação:**
- Marketing bem feito no lançamento
- Documentação excelente (✅ feito)
- Responder rapidamente primeiras issues/PRs
- Identificar "early adopters" (comunidade de VC, usuários de Granola insatisfeitos)

### Risco: Forks Comerciais Competitivos

**Probabilidade:** Baixa (curto prazo), Média (longo prazo)  
**Impacto:** Médio  
**Mitigação:**
- MIT permite isso (é a natureza da licença)
- Diferenciar através de qualidade, velocidade de inovação e comunidade
- Se preocupação for grande, considerar Apache 2.0 ou trademark do nome
- Versão enterprise da Hi Capital pode ter features exclusivas (open core)

### Risco: Vulnerabilidades de Segurança

**Probabilidade:** Média (todo software tem bugs)  
**Impacto:** Alto (manipula gravações privadas)  
**Mitigação:**
- SECURITY.md com processo claro (✅ feito)
- Dependabot configurado (✅ feito)
- Responder vulnerabilidades em ≤30 dias
- Bug bounty no futuro (se projeto crescer)

### Risco: Contribuições de Baixa Qualidade

**Probabilidade:** Média  
**Impacto:** Baixo  
**Mitigação:**
- CI automático pega erros de build (✅ existe)
- CONTRIBUTING.md com padrões claros (✅ feito)
- Code review obrigatório (configurar branch protection)
- Maintainer tem direito de rejeitar PRs

### Risco: Sobrecarga do Maintainer (Gean)

**Probabilidade:** Alta (se projeto crescer)  
**Impacto:** Alto (burnout, projeto estagna)  
**Mitigação:**
- Convidar co-maintainers quando projeto crescer
- Documentar tudo para facilitar onboarding (✅ bem documentado)
- Automatizar o que for possível (CI, dependabot - ✅ feito)
- Definir expectativas realistas de SLA (não é trabalho em tempo integral)

---

## Análise de Qualidade de Código

### ✅ Pontos Fortes

1. **Arquitetura bem pensada**
   - Separação clara de responsabilidades (audio/, transcription/, summary/, CRM)
   - Abstração plugável de provedores
   - Storage local com SQLite

2. **Segurança implementada**
   - Chaves no keychain do SO
   - Redação de segredos nos logs
   - Sem telemetria

3. **Documentação técnica excelente**
   - ADRs documentam decisões com contexto
   - ARCHITECTURE.md detalha todo o sistema
   - Comentários inline explicam "por quês"

4. **Testes existentes**
   - Testes unitários para lógica crítica
   - Testes de integração para audio (#[ignore])
   - CI roda testes em 3 plataformas

5. **CI/CD robusto**
   - Build automático em 3 SOs
   - Download de ffmpeg com retry e fallback
   - Auto-update funcional

### ⚠️ Áreas de Melhoria (para roadmap futuro)

1. **Cobertura de testes**
   - Faltam testes para módulos de CRM
   - Faltam testes para parser ICS
   - Considerar coverage tracking

2. **Internacionalização**
   - UI hardcoded em português
   - Fácil de adicionar i18n (React + JSON)
   - Comunidade pode contribuir traduções

3. **Telemetria de erros (opt-in)**
   - Considerar Sentry ou similar (opt-in)
   - Ajudaria a descobrir bugs em produção
   - Mas: conflita com posicionamento de privacidade

4. **Docs para usuários finais**
   - README é técnico (para desenvolvedores)
   - Considerar wiki ou docs/ para guias de usuário
   - Tutoriais em vídeo

---

## Recomendação Final

✅ **O projeto Hicorder está pronto para ser publicado como open source.**

**Próximos passos:**

1. **Gean revisa este relatório** e decide as questões de política (licença, marca, CLA, monetização, etc.)
2. **Merge deste PR** (cursor/open-source-prep-8994) na branch main
3. **Configurar metadata** do repositório no GitHub (descrição, topics, social preview)
4. **Tornar o repositório público**
5. **Anunciar** o lançamento

O código está limpo, documentado e seguro. A comunidade open source responderá bem a um projeto honesto, bem documentado e com liderança clara.

---

**Arquivos criados/modificados neste PR:**
- README.md (atualizado)
- README.pt-BR.md (novo)
- CONTRIBUTING.md (novo)
- CODE_OF_CONDUCT.md (novo)
- GOVERNANCE.md (novo)
- .env.example (novo)
- .github/ISSUE_TEMPLATE/bug_report.md (novo)
- .github/ISSUE_TEMPLATE/feature_request.md (novo)
- .github/ISSUE_TEMPLATE/crm_integration.md (novo)
- .github/PULL_REQUEST_TEMPLATE.md (novo)
- .github/CODEOWNERS (novo)
- .github/dependabot.yml (novo)
- OPEN_SOURCE_PREP_REPORT.md (este arquivo)

**Total de commits:** 8 (até este ponto)
