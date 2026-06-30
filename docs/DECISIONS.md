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
**Por quê**: descoberto (sondando endpoints) que **MiniMax NÃO tem STT** — só chat (MiniMax-M3) e TSS. Logo o default aponta pra Groq Whisper (OpenAI-compat, free tier, ótimo pt-BR). O mesmo provedor serve OpenAI/qualquer endpoint compatível. A abstração plugável foi o que salvou: trocar provedor é só config, sem refatorar. Idioma por chamada (padrão pt). MiniMax-M3 fica reservado p/ a feature futura de resumo. Ver [MINIMAX.md](MINIMAX.md).

## ADR-004 — Sem assinatura de código na v1
**Decisão**: distribuir instaladores não assinados na v1, com instruções de liberação.
**Por quê**: usuário ainda não tem certificados (Apple Developer / Windows). Para um time interno é aceitável (right-click→Open no Mac, "Executar assim mesmo" no SmartScreen).
**Custo**: avisos de segurança na primeira execução; notarização do Mac fica para depois. Recomendado adquirir certificados antes de distribuir mais amplamente.

## ADR-005 — Formato Opus em `.webm` ~32 kbps
**Decisão**: armazenar em Opus mono ~32 kbps, 16 kHz, container **`.webm`**.
**Por quê**: usuário pediu formato leve. Opus tem a melhor qualidade de fala por bit (~7–15 MB/hora). Container `.webm` (não `.ogg`) porque a MiniMax aceita mp3/mp4/m4a/wav/mpga/**webm**, mas **não** `.ogg`/opus puro. Opus-em-webm é leve E aceito (é o que o navegador/Open WebUI mandam pro endpoint). Meetily usa AAC/MP4 192 kbps (mais pesado).
**Custo**: nenhum relevante; webm/opus é amplamente suportado.

## ADR-006 — ffmpeg empacotado como sidecar
**Decisão**: empacotar o binário do ffmpeg por plataforma como sidecar do Tauri (baixado no build).
**Por quê**: usuário final não pode precisar instalar ffmpeg manualmente ("executável fácil"). Meetily já faz download no `build.rs` — mesmo padrão.
**Custo**: aumenta o tamanho do instalador; precisa de binários por arquitetura.
