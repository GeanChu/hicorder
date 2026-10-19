# Architecture

## Overview

Single desktop application (Tauri 2). Web UI (React) communicates with Rust backend via Tauri commands (IPC) and events. The backend captures audio, encodes, persists to SQLite, and calls AI/CRM APIs.

```
┌─────────────────────────────────────────────────┐
│  UI (Vite + React + TS) — src/App.tsx           │
│  Home (calendar + record + live notes) ·        │
│  Recordings · Summary prompts · Settings        │
└───────────────┬─────────────────────────────────┘
                │ Tauri commands (IPC) + events
┌───────────────▼─────────────────────────────────┐
│  Rust Backend (src-tauri/src)                   │
│  audio/         mic (cpal) + system (per OS)    │
│  encode/        ffmpeg (resource): mix/export   │
│  transcription/ Whisper + hallucination filter  │
│  summary/       chat completions + Claude Code  │
│  attio/         meetings + people + notes       │
│  affinity/      persons + orgs + notes          │
│  meetings/      ICS fetch/parse + RRULE         │
│  scheduler/     auto-start/stop + reminders     │
│  tray.rs        system tray + recording state   │
│  storage/       SQLite                          │
│  settings/      keys (keychain / file)          │
│  net.rs         HTTP client (IPv4-only, OS TLS) │
│  logs/          persistent log + humanize       │
│  migrate.rs     Call Recorder→Hicorder migration│
│  commands/      IPC to UI                       │
└─────────────────────────────────────────────────┘
```

## Modules

### audio/

Captures **mic** and **system audio** in separate tracks, encoded live.

- `mic.rs` — captures via `cpal` in dedicated thread (`!Send` stream)
- `system.rs` — loopback per OS:
  - **Windows**: WASAPI loopback (crate `wasapi`). `buffer_duration_hns` **must be 0**; passing the device's minimum period breaks the stream on some drivers (0 events, empty track).
  - **macOS**: ScreenCaptureKit (crate `screencapturekit`), behind cargo feature `macos-system-audio`. Requires Screen Recording permission.
  - **Linux**: not implemented (records mic only).
- `opus.rs` — `OpusSink`: writes PCM f32 to stdin of a long-running ffmpeg process that records Opus/Ogg **during** the meeting. Benefits: crashes don't lose recording (Ogg tolerates truncation) and stopping is nearly instant.
- `recorder.rs` — start/stop session, elapsed time, level, hourly reminders.

Empty tracks (< 2 KB) are discarded on stop: the recording becomes mic-only instead of breaking the player on mix.

### encode/

`ffmpeg` embedded as bundle **resource** (downloaded in release CI; in dev uses PATH or `CALLREC_FFMPEG`). Used to: mix mic+system on demand (`recording.ogg`), generate `playback.mp3` for the player (macOS WebKit doesn't decode Ogg/Opus), and export (MP3/WAV/OGG).

### transcription/

`OpenAiCompatible` provider: multipart `file`+`model`+`language`+`response_format=verbose_json`+`prompt`, `Authorization: Bearer`. Mic/system tracks are transcribed separately and interleaved with **You**/**Participants** labels. Default: Groq `whisper-large-v3`.

- **Dictionary** (`prompt`): terms that condition the model to recognize names, acronyms, and jargon. **224 token** API ceiling — above that the provider silently discards the excess, so the UI estimates and warns.
- **Hallucination filter**: in silence Whisper "fills" with filler text. `filter_hallucinations` combines (1) subtitle artifact substrings, (2) repeated prefix dominating the track — catches variants like "Caption X" / "Caption X And the" — and (3) `no_speech_prob` + `avg_logprob` when the provider sends them.

### summary/

OpenAI-style chat completions — with one exception: `claude_code.rs` (sentinel endpoint `claude-code://local`) doesn't make HTTP calls, executes the `claude --print` CLI installed on the machine, with transcription via stdin and no API key (ADR-013). The base prompt is editable in Settings and there's a library of named prompts ("Summary prompts" tab), with per-summary override. Manual notes enter the payload to enrich the summary. `max_tokens` is sent only to the NVIDIA endpoint (which has a low default) and `finish_reason: "length"` becomes a truncated summary warning.

### attio/ and affinity/

Two CRMs, chosen in Settings; each stores its own key.

- **attio/** — API v2, Bearer. Searches meetings by **time window** (the `participants` filter on the beta endpoint crashes the server, ADR-007). Creates **one note per person and per company**, linked to the meeting (Attio's model accepts a single parent per note).
- **affinity/** — API v1, Basic auth (empty username, key as password). Creates **ONE note** linked to multiple persons and companies (`person_ids`/`organization_ids`). Since the API has no meetings equivalent, the "choose meeting" step uses the local calendar (ICS) only to suggest participants.

### meetings/ + scheduler/

- `meetings/` — fetches the secret ICS URL and parses VEVENTs (`ical`, `chrono-tz`). Expands **RRULE** for 60 days (crate `rrule`), respects EXDATE and handles `RECURRENCE-ID` overrides (two-pass parse + dedup). `pick_call_link` prefers `X-GOOGLE-CONFERENCE` and known meeting domains.
  - **Discards** events with `STATUS:CANCELLED` (of the three RFC 5545 values — TENTATIVE, CONFIRMED, CANCELLED — only this one removes the meeting from the calendar) and those **I** declined (`ATTENDEE;PARTSTAT=DECLINED`). Another participant's decline doesn't count: the meeting is still happening. The "me" comes from `X-WR-CALNAME` in the feed itself, with the email from Settings as fallback.
  - **Sync is authoritative**: `prune_missing_meetings` removes future meetings that didn't come in the feed. Without this the calendar only grew — upsert added and the only removal was by time, so a meeting deleted in Google stayed in the database until past its time, and if marked for recording the scheduler started recording a nonexistent meeting. Two safeguards: reconciliation only runs **after a successful fetch**, and empty feed doesn't delete anything (ambiguous between empty calendar and truncated response).
- `scheduler/` — 30s loop: starts recording for enabled meeting, warns at expected end, **reminds hourly** that recording is in progress (toast with Stop button) and applies **configurable auto-stop** (default 2h).

### storage/

SQLite (`rusqlite` bundled, WAL + `busy_timeout`) at `app_data/callrec.db`:

- `recordings(id, title, path, system_path, created_at, duration_s, size_bytes)`
- `transcripts(recording_id, language, text, created_at)`
- `summaries(recording_id, text, created_at)`
- `notes(recording_id, text, updated_at)` — manual notes; separate table because they're saved **during** recording, before the `recordings` row exists
- `summary_prompts(id, name, text, created_at)`
- `settings(key, value)` — non-secret preferences (endpoints, models, ICS, CRM, dictionary, auto-stop)
- `meetings(uid, title, starts_at, ends_at, record_enabled, participants, location, link)`

### settings/

API keys **per scope** (`key_scope`): `"<type>:<host>"`, where type is `stt` or `summary`. The same key applies to all models of the provider (including on NVIDIA, whose `nvapi-` is account-wide). Switching providers doesn't delete the previous key; empty scope means no key (no fallback, which would send one provider's credential to another). CRM stores one key per provider (`attio_api_key` / `affinity_api_key`).

Where they are: **OS keychain** (Windows/Linux, crate `keyring`) or **0600 file** at `~/Library/Application Support/com.hicapital.hicorder/secrets.json` (macOS — unsigned app suffers infinite keychain prompts).

### net.rs

Shared `reqwest` client: OS native TLS (compatible with antivirus HTTPS inspection), no system proxy, explicit timeout, and **custom IPv4-only DNS resolver** — networks with IPv6 advertised but no route froze the connection until timeout (ADR-008).

### logs/

Persistent log at `app_data/callrec.log` (rotation ~1MB): raw API errors with timestamp/category — never secrets. `redact()` masks on three fronts: key prefixes (`sk-`, `gsk_`, `nvapi-`, `Bearer …`), query parameters (`key=`, `token=`, …), and **long URL path segments** (≥16 characters). The third exists because the ICS URL is itself a credential — whoever has the link reads the entire calendar without login — and fell raw into the log on every refresh failure. The 16 limit is above the longest legitimate segment of used endpoints (`transcriptions`, 15), so no endpoint is masked. `humanize()` converts raw error to layperson message; UI has "View logs"/"Clear logs". Every recording stop logs the size of each track (`stopped: Xs, mic N bytes, system M bytes`), which diagnoses empty capture without reproducing the problem.

### migrate.rs

One-time, non-destructive migration from old identifier (`com.hicapital.callrecorder`): copies the data folder and fixes absolute paths of recordings. Old folder remains as backup.

## Frontend

`src/App.tsx` (single screen with tabs) + `src/App.css` (design tokens, light/dark).

- **Home** — fixed recording bar at top, calendar, and **live notes** side panel (autosave). In narrow window notes come before calendar; recording and note-taking stay visible.
- **Recordings** — player, rename, export, chat-format transcription with search, editable notes, editable summary with search and prompt selection, and CRM upload.
- **Summary prompts** — CRUD of prompt library.
- **Settings** — two tabs: **Connections** (transcription, summary, calendar, CRM) and **System** (language, theme, recording/auto-stop, autostart, base prompt, dictionary, logs, about).

Toast windows (`?alert=1`) for meeting starting and recording reminder: no decoration, always on top, with auto-dismiss (and backend safety destruction) so they never get stuck on screen.

## Packaging

`tauri build` → `.msi`/NSIS (Windows), `.dmg` (macOS), `.AppImage`/`.deb`/`.rpm` (Linux), generated in CI (`release.yml`) with ffmpeg downloaded per platform. Auto-update via `tauri-plugin-updater` (`latest.json` manifest signed with minisign key). Code signing: SignPath Foundation pending ([SIGNING.md](SIGNING.md)).

**CI gotchas** (don't regress): macOS runner must be `macos-15` **with Xcode 26 selected** (`apple-metal`, ScreenCaptureKit's transitive dep, compiles against macOS 26 SDK); ffmpeg download uses GitHub mirror with retry (single host brought down Windows/Linux and release came out with macOS only); and it's mandatory to check **each job's** completion before publishing — `gh run watch --exit-status` returns 0 even with matrix job failing.
