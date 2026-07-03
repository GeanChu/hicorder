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
