# Arquitetura

## Visão geral
Aplicativo desktop único (Tauri 2). UI web (React) conversa com o backend Rust via comandos Tauri (IPC) e eventos. O backend captura áudio, faz encode, persiste em SQLite e chama as APIs de IA/CRM.

```
┌─────────────────────────────────────────────────┐
│  UI (Vite + React + TS) — src/App.tsx           │
│  Home (agenda + gravar + anotações ao vivo) ·   │
│  Gravações · Prompts de resumo · Configurações  │
└───────────────┬─────────────────────────────────┘
                │ comandos Tauri (IPC) + eventos
┌───────────────▼─────────────────────────────────┐
│  Backend Rust (src-tauri/src)                   │
│  audio/         mic (cpal) + sistema (por SO)   │
│  encode/        ffmpeg (resource): mix/export   │
│  transcription/ Whisper + filtro de alucinação  │
│  summary/       chat completions (OpenAI-like)  │
│  attio/         meetings + people + notes       │
│  affinity/      persons + orgs + notes          │
│  meetings/      ICS fetch/parse + RRULE         │
│  scheduler/     auto-start/stop + lembretes     │
│  tray.rs        bandeja + estado de gravação    │
│  storage/       SQLite                          │
│  settings/      chaves (keychain / arquivo)     │
│  net.rs         HTTP client (IPv4-only, TLS SO) │
│  logs/          log persistente + humanize      │
│  migrate.rs     migração Call Recorder→Hicorder │
│  commands/      IPC para a UI                   │
└─────────────────────────────────────────────────┘
```

## Módulos

### audio/
Captura **mic** e **áudio do sistema** em faixas separadas, já codificadas ao vivo.
- `mic.rs` — captura via `cpal` numa thread dedicada (stream `!Send`).
- `system.rs` — loopback por SO:
  - **Windows**: WASAPI loopback (crate `wasapi`). `buffer_duration_hns` **deve ser 0**; passar o período mínimo do device quebra o stream em alguns drivers (0 eventos, faixa vazia).
  - **macOS**: ScreenCaptureKit (crate `screencapturekit`), atrás da feature de cargo `macos-system-audio`. Exige permissão de Gravação de Tela.
  - **Linux**: não implementado (grava só o mic).
- `opus.rs` — `OpusSink`: escreve PCM f32 no stdin de um ffmpeg de longa duração que grava Opus/Ogg **durante** a reunião. Ganhos: travamento não perde a gravação (Ogg tolera truncamento) e parar é quase instantâneo.
- `recorder.rs` — sessão start/stop, tempo decorrido, nível, lembretes de hora cheia.

Faixa vazia (< 2 KB) é descartada ao parar: a gravação vira só-mic em vez de quebrar o player no mix.

### encode/
`ffmpeg` embutido como **resource** do bundle (baixado no CI de release; em dev usa o PATH ou `CALLREC_FFMPEG`). Usado para: mixar mic+sistema sob demanda (`recording.ogg`), gerar `playback.mp3` para o player (o WebKit do macOS não decodifica Ogg/Opus) e exportar (MP3/WAV/OGG).

### transcription/
Provedor `OpenAiCompatible`: multipart `file`+`model`+`language`+`response_format=verbose_json`+`prompt`, `Authorization: Bearer`. As faixas mic/sistema são transcritas separadamente e intercaladas com rótulos **Você**/**Participantes**. Default: Groq `whisper-large-v3`.

- **Dicionário** (`prompt`): termos que condicionam o modelo a reconhecer nomes, siglas e jargão. Teto de **224 tokens** na API — acima disso o provedor descarta o excedente em silêncio, por isso a UI estima e avisa.
- **Filtro de alucinação**: em silêncio o Whisper "preenche" com muletas. `filter_hallucinations` combina (1) substrings de artefato de legenda, (2) prefixo repetido dominando a faixa — pega variantes como "Legenda X" / "Legenda X E a" — e (3) `no_speech_prob` + `avg_logprob` quando o provedor os envia.

### summary/
Chat completions estilo OpenAI. O prompt base é editável nas Configurações e há uma biblioteca de prompts nomeados (aba "Prompts de resumo"), com override por resumo. As anotações manuais entram no payload para enriquecer o resumo. `max_tokens` é enviado só no endpoint da NVIDIA (que tem default baixo) e `finish_reason: "length"` vira aviso de resumo truncado.

### attio/ e affinity/
Dois CRMs, escolhidos nas Configurações; cada um guarda a própria chave.

- **attio/** — API v2, Bearer. Busca reuniões por **janela de tempo** (o filtro `participants` do endpoint beta trava no servidor, ADR-007). Cria **uma nota por pessoa e por empresa**, ligada à meeting (o modelo do Attio aceita um único pai por nota).
- **affinity/** — API v1, Basic auth (usuário vazio, chave como senha). Cria **UMA nota** ligada a várias pessoas e empresas (`person_ids`/`organization_ids`). Como a API não tem equivalente de meetings, o passo "escolha a reunião" usa a agenda local (ICS) só para sugerir participantes.

### meetings/ + scheduler/
- `meetings/` — busca a URL ICS secreta e parseia VEVENTs (`ical`, `chrono-tz`). Expande **RRULE** em 60 dias (crate `rrule`), respeita EXDATE e trata overrides `RECURRENCE-ID` (parse em duas passadas + dedup). `pick_call_link` prefere `X-GOOGLE-CONFERENCE` e domínios de reunião conhecidos.
- `scheduler/` — loop de 30s: inicia gravação de reunião habilitada, avisa no fim previsto, **lembra de hora em hora** que há gravação em andamento (toast com botão Parar) e aplica o **auto-stop configurável** (padrão 2h).

### storage/
SQLite (`rusqlite` bundled, WAL + `busy_timeout`) em `app_data/callrec.db`:
- `recordings(id, title, path, system_path, created_at, duration_s, size_bytes)`
- `transcripts(recording_id, language, text, created_at)`
- `summaries(recording_id, text, created_at)`
- `notes(recording_id, text, updated_at)` — anotações manuais; tabela própria porque são salvas **durante** a gravação, antes de a linha de `recordings` existir
- `summary_prompts(id, name, text, created_at)`
- `settings(key, value)` — preferências não secretas (endpoints, modelos, ICS, CRM, dicionário, auto-stop)
- `meetings(uid, title, starts_at, ends_at, record_enabled, participants, location, link)`

### settings/
Chaves de API **por escopo** (`key_scope`): `"<tipo>:<host>"`, onde tipo é `stt` ou `summary`. A mesma chave vale para todos os modelos do provedor (inclusive na NVIDIA, cuja `nvapi-` é de conta). Trocar de provedor não apaga a chave do anterior; escopo vazio significa sem chave (sem fallback, que enviaria a credencial de um provedor a outro). O CRM guarda uma chave por provedor (`attio_api_key` / `affinity_api_key`).

Onde ficam: **keychain do SO** (Windows/Linux, crate `keyring`) ou **arquivo 0600** em `~/Library/Application Support/com.hicapital.hicorder/secrets.json` (macOS — app não assinado sofre prompts infinitos do chaveiro).

### net.rs
Client `reqwest` compartilhado: TLS nativo do SO (compatível com inspeção HTTPS de antivírus), sem proxy do sistema, timeout explícito e **resolver DNS custom IPv4-only** — redes com IPv6 anunciado mas sem rota travavam a conexão até o timeout (ADR-008).

### logs/
Log persistente em `app_data/callrec.log` (rotação ~1MB): erros crus de API com timestamp/categoria — nunca chaves. `humanize()` converte o erro cru em mensagem para leigos; a UI tem "Ver logs"/"Limpar logs". Toda parada de gravação registra o tamanho de cada faixa (`parada: Xs, mic N bytes, sistema M bytes`), o que diagnostica captura vazia sem reproduzir o problema.

### migrate.rs
Migração única e não destrutiva do identifier antigo (`com.hicapital.callrecorder`): copia a pasta de dados e corrige os paths absolutos das gravações. A pasta antiga permanece como backup.

## Frontend
`src/App.tsx` (tela única com abas) + `src/App.css` (design tokens, claro/escuro).

- **Home** — barra de gravação fixa no topo, agenda e painel lateral de **anotações ao vivo** (autosave). Em janela estreita as anotações vêm antes da agenda; gravar e anotar seguem visíveis.
- **Gravações** — player, renomear, exportar, transcrição em formato chat com busca, anotações editáveis, resumo editável com busca e escolha de prompt, e envio ao CRM.
- **Prompts de resumo** — CRUD da biblioteca de prompts.
- **Configurações** — duas abas: **Conexões** (transcrição, resumo, calendário, CRM) e **Sistema** (idioma, tema, gravação/auto-stop, autostart, prompt base, dicionário, logs, sobre).

Janelas-toast (`?alert=1`) para reunião começando e lembrete de gravação: sem decoração, sempre no topo, com auto-dispensa (e destruição de segurança no backend) para nunca ficarem presas na tela.

## Empacotamento
`tauri build` → `.msi`/NSIS (Windows), `.dmg` (macOS), `.AppImage`/`.deb`/`.rpm` (Linux), gerados no CI (`release.yml`) com ffmpeg baixado por plataforma. Auto-update via `tauri-plugin-updater` (manifesto `latest.json` assinado com chave minisign). Assinatura de código: SignPath Foundation pendente ([SIGNING.md](SIGNING.md)).

**Gotchas de CI** (não regredir): runner macOS precisa ser `macos-15` **com Xcode 26 selecionado** (o `apple-metal`, dep transitiva do ScreenCaptureKit, compila contra o SDK do macOS 26); o download do ffmpeg usa mirror do GitHub com retry (host único derrubava Windows/Linux e a release saía só com macOS); e é obrigatório conferir a conclusão **de cada job** antes de publicar — `gh run watch --exit-status` retorna 0 mesmo com job da matriz falhando.
