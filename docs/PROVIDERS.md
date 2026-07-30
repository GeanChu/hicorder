# Provedores de IA e CRM

O Hicorder fala dois protocolos de IA, ambos estilo OpenAI. Qualquer endpoint compatível funciona (opção "Personalizado" nas Configurações).

## Transcrição (speech-to-text)
Protocolo: `POST <endpoint>` multipart (`file`, `model`, `language`, `response_format=verbose_json`, `prompt`), `Authorization: Bearer <chave>`. O app transcreve as faixas mic/sistema separadamente e intercala com rótulos "Você"/"Participantes".

| Provedor | Endpoint | Modelos no select | Chave |
|---|---|---|---|
| **Groq** (default, free tier) | `https://api.groq.com/openai/v1/audio/transcriptions` | `whisper-large-v3` (default), `whisper-large-v3-turbo` | console.groq.com/keys |
| OpenAI | `https://api.openai.com/v1/audio/transcriptions` | `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe` | platform.openai.com/api-keys |
| Fireworks AI | `https://api.fireworks.ai/inference/v1/audio/transcriptions` | `whisper-v3`, `whisper-v3-turbo` | fireworks.ai → API Keys |

O áudio enviado é Opus em `.ogg`.

**Por que large-v3 e não o turbo**: o turbo alucina mais em trechos de silêncio — e numa reunião cada faixa fica muda enquanto o outro lado fala.

**`distil-whisper-large-v3-en` foi removido**: a Groq descomissionou o modelo (erro `model_decommissioned`).

### Dicionário (campo `prompt`)
Termos que ajudam o modelo com nomes próprios, siglas e jargão. Configurações → Sistema → Dicionário.

- Limite da API: **224 tokens**. Acima disso o provedor **descarta o excedente em silêncio** (mantendo o final), por isso a UI estima os tokens e avisa.
- Limite do app: 65 palavras. O dicionário de fábrica traz 40 termos de VC/investimentos/fintech (~133 tokens), deixando folga para os termos do usuário.
- O maior ganho vem de nomes de fundos, empresas e pessoas recorrentes — palavra comum do português o modelo já acerta.

## Resumo (LLM)
Protocolo: `POST <endpoint>` JSON chat completions (`model`, `messages`), `Authorization: Bearer <chave>`. O prompt base é editável (Configurações → Sistema) e há biblioteca de prompts nomeados com override por resumo.

| Provedor | Endpoint | Modelos no select | Chave |
|---|---|---|---|
| OpenAI | `https://api.openai.com/v1/chat/completions` | `gpt-4o-mini`, `gpt-4o`, `gpt-4.1`, ... | platform.openai.com/api-keys |
| Claude (Anthropic) | `https://api.anthropic.com/v1/chat/completions` (camada compat OpenAI) | `claude-3-5-sonnet-latest`, ... | console.anthropic.com/settings/keys |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions` | `gemini-2.0-flash`, ... | aistudio.google.com/apikey |
| MiniMax (Subscription sk-cp) | `https://api.minimax.io/v1/chat/completions` | `MiniMax-M3`, `MiniMax-Text-01` | Subscription Key da conta |
| MiniMax (API) | idem | idem | platform.minimax.io → API Keys |
| **NVIDIA NIM** | `https://integrate.api.nvidia.com/v1/chat/completions` | `minimaxai/minimax-m3`, `deepseek-ai/deepseek-v4-pro`, `meta/llama-3.3-70b-instruct`, `deepseek-ai/deepseek-r1` | build.nvidia.com → API key (`nvapi-`) |

Notas da NVIDIA:
- A `nvapi-` é **chave de conta**, não de modelo: vale para todo o catálogo. O botão "Get API Key" na página de um modelo entrega/rotaciona a mesma chave — gerar por lá invalida a anterior.
- O NIM tem `max_tokens` padrão baixo, então o app envia um teto explícito **só nesse endpoint** (8192 no minimax-m3, 16384 nos demais).
- Modelos de raciocínio devolvem `<think>`; o app remove antes de salvar.

## CRM
Escolhido em Configurações → Conexões → CRM. Cada CRM guarda a própria chave; trocar não apaga a do outro.

| CRM | API | Auth | Modelo de nota |
|---|---|---|---|
| **Attio** | v2 (`api.attio.com`) | Bearer | Uma nota **por pessoa e por empresa**, ligada à meeting (a API aceita um único pai por nota) |
| **Affinity** | v1 (`api.affinity.co`) | Basic (usuário vazio, chave como senha) | **Uma nota só**, vinculada a várias pessoas e empresas |

No Affinity não há endpoint de meetings, então a etapa "escolha a reunião" usa a agenda local (ICS) apenas para sugerir os participantes.

## Chaves: uma por provedor
As chaves são guardadas por escopo `(tipo, host)` — a mesma chave da Groq serve todos os modelos Whisper, a da MiniMax todos os dela. Transcrição e resumo nunca compartilham chave, mesmo no mesmo host (ex.: OpenAI Whisper vs GPT). Ao trocar de provedor na tela, o campo limpa e o placeholder indica se já existe chave salva para aquele escopo.

## Teste de chave
Botão "Testar" ao lado de cada chave nas Configurações:
- Transcrição: `GET <base>/models` (valida sem enviar áudio).
- Resumo: chat completions mínimo com `max_tokens: 16` — 1 token quebrava modelos de raciocínio, que gastam tokens pensando antes de responder.
- Attio: `GET /v2/meetings?limit=1`. Affinity: `GET /auth/whoami`.

Erros aparecem em linguagem simples; o erro cru fica no log persistente (Configurações → Sistema → Ver logs).

## Nota histórica: MiniMax não tem STT
Verificado em 2026-06-30 sondando endpoints: a MiniMax oferece chat (`/v1/chat/completions`, MiniMax-M3) e TTS (`/v1/t2a_v2`), mas **nenhum** endpoint de transcrição (14 variações testadas, todas 404). Por isso o default de transcrição é Groq e a MiniMax aparece só como provedor de resumo.

## Segurança
- Chaves no keychain do SO (arquivo 0600 no macOS); nunca em texto puro, nunca em logs.
- A UI nunca recebe o valor de uma chave — só a informação de que existe.
- A transcrição/resumo envia áudio/texto apenas ao provedor configurado.
