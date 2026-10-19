# AI and CRM Providers

Hicorder speaks two AI protocols, both OpenAI-style. Any compatible endpoint works ("Custom" option in Settings).

## Transcription (speech-to-text)

Protocol: `POST <endpoint>` multipart (`file`, `model`, `language`, `response_format=verbose_json`, `prompt`), `Authorization: Bearer <key>`. The app transcribes mic/system tracks separately and interleaves with "You"/"Participants" labels.

| Provider | Endpoint | Models in dropdown | Key |
|---|---|---|---|
| **Groq** (default, free tier) | `https://api.groq.com/openai/v1/audio/transcriptions` | `whisper-large-v3` (default), `whisper-large-v3-turbo` | console.groq.com/keys |
| OpenAI | `https://api.openai.com/v1/audio/transcriptions` | `whisper-1`, `gpt-4o-transcribe`, `gpt-4o-mini-transcribe` | platform.openai.com/api-keys |
| Fireworks AI | `https://api.fireworks.ai/inference/v1/audio/transcriptions` | `whisper-v3`, `whisper-v3-turbo` | fireworks.ai → API Keys |

The audio sent is Opus in `.ogg` format.

**Why large-v3 and not turbo**: turbo hallucinates more in silence stretches — and in a meeting each track is mute while the other side speaks.

**`distil-whisper-large-v3-en` was removed**: Groq decommissioned the model (`model_decommissioned` error).

### Dictionary (`prompt` field)

Terms that help the model with proper names, acronyms, and jargon. Settings → System → Dictionary.

- API limit: **224 tokens**. Above that the provider **silently discards the excess** (keeping the end), so the UI estimates tokens and warns.
- App limit: 65 words. Factory dictionary has 40 VC/investment/fintech terms (~133 tokens), leaving room for user terms.
- Biggest gain comes from recurring fund names, companies, and people — common words the model already gets right.

## Summary (LLM)

Protocol: `POST <endpoint>` JSON chat completions (`model`, `messages`), `Authorization: Bearer <key>`. Base prompt is editable (Settings → System) and there's a library of named prompts with per-summary override.

| Provider | Endpoint | Models in dropdown | Key |
|---|---|---|---|
| OpenAI | `https://api.openai.com/v1/chat/completions` | `gpt-4o-mini`, `gpt-4o`, `gpt-4.1`, ... | platform.openai.com/api-keys |
| Claude (Anthropic) | `https://api.anthropic.com/v1/chat/completions` (OpenAI compat layer) | `claude-3-5-sonnet-latest`, ... | console.anthropic.com/settings/keys |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions` | `gemini-2.0-flash`, ... | aistudio.google.com/apikey |
| MiniMax (Subscription sk-cp) | `https://api.minimax.io/v1/chat/completions` | `MiniMax-M3`, `MiniMax-Text-01` | Account Subscription Key |
| MiniMax (API) | same | same | platform.minimax.io → API Keys |
| **NVIDIA NIM** | `https://integrate.api.nvidia.com/v1/chat/completions` | `minimaxai/minimax-m3`, `deepseek-ai/deepseek-v4-pro`, `meta/llama-3.3-70b-instruct`, `deepseek-ai/deepseek-r1` | build.nvidia.com → API key (`nvapi-`) |
| **Claude Code (local)** | `claude-code://local` (sentinel — process, not HTTP) | `claude-sonnet-5`, `claude-opus-5`, `claude-haiku-4-5` | **none** (uses machine's Claude subscription) |

NVIDIA notes:
- `nvapi-` is **account key**, not per-model: valid for entire catalog. The "Get API Key" button on a model's page delivers/rotates the same key — generating there invalidates the previous one.
- NIM has low default `max_tokens`, so the app sends an explicit ceiling **only on this endpoint** (8192 for minimax-m3, 16384 for others).
- Reasoning models return `<think>`; the app removes it before saving.

### Claude Code (local)

Only provider that **is not HTTP**: the app executes the `claude` CLI installed on the machine (`--print`, transcription via **stdin**) and authentication is the user's Claude subscription — no API key to register.

- **Not offline.** Claude Code talks to Anthropic's API like any other provider. The gain is not needing another key, not extra privacy.
- **Cost/quota**: each call carries ~14k context tokens from Claude Code itself (already with `--exclude-dynamic-system-prompt-sections`; without the flag it's ~27k). On Pro/Max subscription it comes from quota.
- **Requires per-machine installation.** Claude Code and **Claude Desktop are different programs with the same executable name** (`claude.exe`): Desktop doesn't embed the CLI — it spawns it via `node-pty`. Installing Desktop **does not** give you the CLI.
- **Binary search**: path informed in Settings → `PATH` → known locations (`~/.local/bin`, `~/.claude/local`, `%APPDATA%\npm`, `/usr/local/bin`, `/opt/homebrew/bin`). PATH alone isn't enough: opened from tray/autostart the app inherits a poorer PATH. Paths under `AnthropicClaude/` are discarded (that's Desktop).
- **Containment**: `--max-turns 1` and `--disallowed-tools` prevent the CLI from becoming an agent and reading machine files; runs in temp folder, not user directory.
- npm `.cmd`/`.bat` shims are executed via `cmd.exe /C` (Windows CreateProcess doesn't run `.cmd` directly).

## CRM

Chosen in Settings → Connections → CRM. Each CRM stores its own key; switching doesn't delete the other's.

| CRM | API | Auth | Note model |
|---|---|---|---|
| **Attio** | v2 (`api.attio.com`) | Bearer | One note **per person and per company**, linked to meeting (API accepts single parent per note) |
| **Affinity** | v1 (`api.affinity.co`) | Basic (empty username, key as password) | **One note**, linked to multiple persons and companies |

Affinity has no meetings endpoint, so the "choose meeting" step uses the local calendar (ICS) only to suggest participants.

## Keys: one per provider

Keys are stored by scope `(type, host)` — the same Groq key serves all Whisper models, MiniMax's serves all of theirs. Transcription and summary never share keys, even on the same host (e.g.: OpenAI Whisper vs GPT). When switching providers on screen, the field clears and placeholder indicates if a key already exists for that scope.

## Key testing

"Test" button next to each key in Settings:
- Transcription: `GET <base>/models` (validates without sending audio).
- Summary: minimal chat completions with `max_tokens: 16` — 1 token broke reasoning models, which spend tokens thinking before responding.
- Summary via **local Claude Code** ("Test installation"): finds binary → `claude --version` (response must contain "Claude Code", which discards the homonymous Desktop app) → minimal real call. Version alone wouldn't suffice: most likely case on a new user's machine is CLI installed and **not authenticated**.
- Attio: `GET /v2/meetings?limit=1`. Affinity: `GET /auth/whoami`.

Errors appear in plain language; raw error stays in persistent log (Settings → System → View logs).

## Historical note: MiniMax has no STT

Verified on 2026-06-30 by probing endpoints: MiniMax offers chat (`/v1/chat/completions`, MiniMax-M3) and TTS (`/v1/t2a_v2`), but **no** transcription endpoint (14 variations tested, all 404). That's why transcription default is Groq and MiniMax appears only as summary provider.

## Security

- Keys in OS keychain (file on macOS); never in plain text, never in logs.
- UI never receives a key's value — only the information that it exists.
- Transcription/summary sends audio/text only to the configured provider.
