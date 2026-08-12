# Decisões de Arquitetura (ADR resumido)

## ADR-001 — Tauri 2 (não Electron)
**Decisão**: Tauri 2 (Rust + UI web).
**Por quê**: executável pequeno e nativo, melhor acesso de baixo nível ao áudio do sistema, e é a mesma base do meetily — permite reaproveitar a captura de áudio cross-platform (MIT), que é a parte mais difícil. Electron seria todo-JS e mais fácil de evoluir por devs JS, mas gera binário pesado (~150 MB) e captura de áudio do sistema mais limitada, principalmente no macOS.
**Custo**: exige Rust no projeto.

## ADR-002 — macOS via ScreenCaptureKit
**Decisão**: capturar áudio do sistema no macOS com ScreenCaptureKit (13+). macOS <13 grava só microfone.
**Por quê**: macOS não tem loopback nativo nas versões antigas. ScreenCaptureKit é a API oficial, não exige instalar driver virtual (BlackHole), evitando fricção para time não técnico. Driver virtual foi descartado por exigir instalação/configuração manual.
**Custo**: pede permissão de gravação de tela; macOS <13 fica sem áudio do sistema.

## ADR-003 — Transcrição plugável, Groq por padrão
**Decisão**: trait `Transcriber` + provedor `OpenAiCompatible`; endpoint/modelo/chave configuráveis na UI; default = **Groq Whisper**.
**Por quê**: descoberto (sondando endpoints) que **MiniMax NÃO tem STT** — só chat (MiniMax-M3) e TSS. Logo o default aponta pra Groq Whisper (OpenAI-compat, free tier, ótimo pt-BR). O mesmo provedor serve OpenAI/qualquer endpoint compatível. A abstração plugável foi o que salvou: trocar provedor é só config, sem refatorar. Idioma por chamada (padrão pt). MiniMax-M3 fica reservado p/ a feature futura de resumo. Ver [PROVIDERS.md](PROVIDERS.md).

## ADR-004 — Assinatura de código via SignPath Foundation (não certificado pago)
**Decisão**: v1 distribui instaladores não assinados; a assinatura Windows virá **gratuita** via SignPath Foundation (programa para OSS), integrada ao CI. Ver [SIGNING.md](SIGNING.md).
**Por quê**: certificado OV/EV pago foi descartado pelo usuário. SignPath Foundation assina projetos open source qualificados (licença OSI + repo público + build em CI), que é o caso.
**Custo**: até a aprovação, avisos de SmartScreen/antivírus na primeira execução (mitigados com metadados de bundle e reporte de falso positivo). Notarização do macOS fica para depois (exige Apple Developer pago).

## ADR-005 — Formato Opus em `.webm` ~32 kbps
**Decisão**: armazenar em Opus mono ~32 kbps, 16 kHz, container **`.webm`**.
**Por quê**: usuário pediu formato leve. Opus tem a melhor qualidade de fala por bit (~7–15 MB/hora). Container `.webm` (não `.ogg`) porque a MiniMax aceita mp3/mp4/m4a/wav/mpga/**webm**, mas **não** `.ogg`/opus puro. Opus-em-webm é leve E aceito (é o que o navegador/Open WebUI mandam pro endpoint). Meetily usa AAC/MP4 192 kbps (mais pesado).
**Custo**: nenhum relevante; webm/opus é amplamente suportado.

## ADR-006 — ffmpeg empacotado como resource
**Decisão**: empacotar o binário do ffmpeg por plataforma como **resource** do bundle Tauri (baixado no CI de release). Em dev usa o ffmpeg do PATH ou `CALLREC_FFMPEG`.
**Por quê**: usuário final não pode precisar instalar ffmpeg manualmente ("executável fácil").
**Custo**: aumenta o tamanho do instalador; precisa de binários por plataforma.

## ADR-007 — Attio: busca de reunião por janela de tempo (não por participants)
**Decisão**: buscar meetings no Attio por `ends_from`/`starts_before`/`timezone` e casar emails **no cliente** sobre `participants[].email_address`.
**Por quê**: o parâmetro `participants` do `GET /v2/meetings` (endpoint beta) existe no schema OpenAPI mas **trava o servidor** em runtime — a mesma chamada autenticada responde 200 em ~0,5s sem o parâmetro e dá timeout com ele (validado com self-test dentro do app). O filtro por tempo funciona e casa com o fluxo do produto (match por horário + confirmação do usuário).
**Custo**: baixa mais reuniões que o necessário (limit 50 na janela); filtro fino é client-side.

## ADR-008 — HTTP com resolver DNS IPv4-only
**Decisão**: o client HTTP compartilhado (`net.rs`) usa um resolver custom que filtra os resultados do getaddrinfo para IPv4, além de TLS nativo do SO e proxy do sistema desabilitado.
**Por quê**: em redes com IPv6 anunciado mas sem rota (caso real do usuário), o reqwest tentava o endereço AAAA e pendurava até o timeout; `local_address(0.0.0.0)` não resolveu. TLS nativo convive com inspeção HTTPS de antivírus (Kaspersky); o proxy do sistema injetado pelo antivírus quebrava conexões.
**Custo**: sem suporte a redes IPv6-only (aceitável para o público-alvo hoje).

## ADR-009 — Rename para Hicorder com migração não destrutiva
**Decisão**: produto/identifier renomeados para Hicorder / `com.hicapital.hicorder` (v0.2.0). Migração única: copia a pasta de dados antiga, corrige paths absolutos no DB copiado e replica as chaves do keychain do serviço antigo na primeira leitura. Nada é apagado do lado antigo.
**Por quê**: novo nome de produto; cópia (não move) permite rollback trivial.
**Custo**: dados duplicados em disco até o usuário apagar a pasta antiga manualmente.

## ADR-010 — Chaves de API por escopo (provedor), sem fallback
**Decisão**: cada chave é guardada sob `"<tipo>:<host>"` (tipo = `stt` | `summary`). Trocar de provedor não apaga a chave anterior; escopo sem valor significa **sem chave**, sem cair em nenhuma chave antiga.
**Por quê**: com uma chave única por etapa, trocar de provedor sobrescrevia a anterior. O fallback que existia na primeira versão do escopo era pior: mandava a credencial de um provedor para outro (chave MiniMax indo ao endpoint da NVIDIA) e ainda fazia a UI mostrar "chave salva" em provedor nunca configurado.
**Custo**: quem tinha a chave única precisa informá-la uma vez no provedor certo. Não migramos automaticamente porque não há registro de a qual provedor ela pertencia.

## ADR-011 — Suporte a dois CRMs com modelos de nota diferentes
**Decisão**: Attio e Affinity coexistem; o usuário escolhe nas Configurações e cada um guarda a própria chave. A UI de envio é a mesma, mas o backend despacha para o módulo do CRM ativo.
**Por quê**: o modelo de dados difere. No **Attio** uma nota tem um único pai, então é criada uma nota por pessoa e por empresa, ligada à meeting. No **Affinity** uma nota aceita `person_ids` e `organization_ids`, então **uma nota só** cobre todos os vínculos. Além disso o Affinity não tem endpoint de meetings — a etapa "escolha a reunião" usa a agenda local (ICS) apenas para sugerir participantes.
**Custo**: a mensagem de resultado e o passo 1 do fluxo variam por CRM; dois caminhos de código para manter.

## ADR-014 — Deduplicação de meetings no Attio passa a ser nossa
**Contexto**: em 08/2026 o Attio comunicou que tira Meetings/Call Recordings do alpha e **deprecia o `external_ref`** — cada `POST /v2/meetings` passa a criar uma meeting nova, mesmo com o mesmo `external_ref`. Antes, esse campo é que garantia o reaproveitamento.

**Decisão**: antes de criar, consultar `GET /v2/meetings` na janela do horário da reunião e reusar a que tiver o mesmo título (`find_existing_meeting`). **E continuar enviando `external_ref`.**

**Por que continuar enviando um campo deprecado**: a versão da API em produção ainda o exige como obrigatório. Removê-lo agora quebraria as chamadas; mantê-lo é inofensivo depois da migração, porque campo obsoleto é ignorado e não rejeitado. O mesmo binário funciona nos dois lados da mudança — e o app é distribuído, então não dá para sincronizar o deploy com a data do Attio.

**Custo**: uma requisição a mais por upload que cria reunião. Falha na busca não bloqueia: cria assim mesmo, porque subir a nota importa mais que o risco de uma duplicata.

**Sem impacto**: os outros dois itens do comunicado (call recordings passam a exigir transcript em 14/10/2026; transcript volta junto do call recording) tocam endpoints que este app nunca chamou.

## ADR-013 — Resumo pelo Claude Code local (processo, não HTTP)
**Decisão**: adicionar "Claude Code (instalado nesta máquina)" como provedor de resumo. O endpoint é o sentinela `claude-code://local`; o backend detecta e executa o CLI `claude --print` com a transcrição pelo **stdin**, em vez de fazer POST.
**Por quê**: é o único provedor que **dispensa cadastrar chave de API** — autentica pela assinatura Claude que o usuário já tem. Para quem já usa Claude Code, é o caminho de menor fricção.
**Custo e limites** (medidos no CLI 2.1.220):
- **Não é offline.** O Claude Code fala com a API da Anthropic; não há ganho de privacidade sobre os outros provedores.
- ~14k tokens de contexto por chamada só de scaffolding do CLI (27k sem `--exclude-dynamic-system-prompt-sections`). Consome cota da assinatura.
- Exige instalação **e autenticação** por máquina — barreira real para o público não técnico do produto. Por isso é opção avançada, nunca default.
- Depende de flags de um CLI de terceiros, que não têm estabilidade de API. O teste valida a versão a cada uso.

**Por que não usar o Claude Desktop no lugar**: são programas diferentes com o **mesmo nome de executável**. O Desktop (Electron, ~232 MB em `AnthropicClaude/`) não embute o CLI — ele spawna um `claude` externo via `node-pty` e não expõe `--print`. Instalar o Desktop não dá o CLI. Ler o token OAuth do Desktop para autenticar o Hicorder foi **descartado**: é uso indevido de credencial de outro app e quebraria no primeiro refresh.

**Contenção**: `--max-turns 1` + `--disallowed-tools` (Bash/Read/Write/Edit/Glob/Grep/WebFetch/WebSearch/Task/NotebookEdit) impedem o CLI de virar agente com acesso ao disco; execução numa pasta temporária, não no diretório do usuário; transcrição por stdin porque reunião de 1h estoura o limite de ~32k caracteres da linha de comando no Windows.

## ADR-012 — Dicionário do Whisper limitado a 65 palavras
**Decisão**: o campo `prompt` do Whisper recebe um dicionário editável, com 40 termos de fábrica e teto de 65 palavras na UI.
**Por quê**: o `prompt` melhora nomes próprios, siglas e jargão, mas a API corta em **224 tokens** e descarta o excedente **em silêncio**. 40 termos ocupam ~133 tokens, deixando ~91 para os 25 que o usuário pode acrescentar — o limite de 65 é o que cabe de fato. A UI estima os tokens e avisa antes do corte.
**Custo**: estimativa de tokens é aproximada (~4 chars/token); o aviso pode disparar um pouco antes ou depois do limite real.
