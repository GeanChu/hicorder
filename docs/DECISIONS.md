# Architecture Decisions (ADR summary)

## ADR-001 — Tauri 2 (not Electron)

**Decision**: Tauri 2 (Rust + web UI).  
**Why**: small native executable, better low-level access to system audio, and it's the same base as meetily — allows reusing the cross-platform audio capture (MIT), which is the hardest part. Electron would be all-JS and easier to evolve for JS devs, but generates heavy binary (~150 MB) and more limited system audio capture, especially on macOS.  
**Cost**: requires Rust in project.

## ADR-002 — macOS via ScreenCaptureKit

**Decision**: capture system audio on macOS with ScreenCaptureKit (13+). macOS <13 records mic only.  
**Why**: macOS doesn't have native loopback on older versions. ScreenCaptureKit is the official API, doesn't require installing virtual driver (BlackHole), avoiding friction for non-technical team. Virtual driver was discarded for requiring manual installation/configuration.  
**Cost**: asks for screen recording permission; macOS <13 is without system audio.

## ADR-003 — Pluggable transcription, Groq by default

**Decision**: trait `Transcriber` + `OpenAiCompatible` provider; endpoint/model/key configurable in UI; default = **Groq Whisper**.  
**Why**: discovered (probing endpoints) that **MiniMax has NO STT** — only chat (MiniMax-M3) and TTS. So default points to Groq Whisper (OpenAI-compat, free tier, excellent). Same provider serves OpenAI/any compatible endpoint. Pluggable abstraction saved us: switching provider is just config, no refactoring. Language per call (default pt). MiniMax-M3 reserved for future summary feature. See [PROVIDERS.md](PROVIDERS.md).

## ADR-004 — Code signing via SignPath Foundation (not paid certificate)

**Decision**: v1 distributes unsigned installers; Windows signing will come **free** via SignPath Foundation (OSS program), integrated into CI. See [SIGNING.md](SIGNING.md).  
**Why**: paid OV/EV certificate was discarded by user. SignPath Foundation signs qualifying open source projects (OSI license + public repo + CI build), which this is.  
**Cost**: until approval, SmartScreen/antivirus warnings on first run (mitigated with bundle metadata and false positive reporting). macOS notarization later (requires paid Apple Developer).

## ADR-005 — Opus format in `.webm` ~32 kbps

**Decision**: store in Opus mono ~32 kbps, 16 kHz, **`.webm`** container.  
**Why**: user requested lightweight format. Opus has best speech quality per bit (~7–15 MB/hour). `.webm` container (not `.ogg`) because MiniMax accepts mp3/mp4/m4a/wav/mpga/**webm**, but **not** pure `.ogg`/opus. Opus-in-webm is lightweight AND accepted (it's what browser/Open WebUI send to endpoint). Meetily uses AAC/MP4 192 kbps (heavier).  
**Cost**: none relevant; webm/opus is widely supported.

## ADR-006 — ffmpeg packaged as resource

**Decision**: package ffmpeg binary per platform as Tauri bundle **resource** (downloaded in release CI). In dev uses ffmpeg from PATH or `CALLREC_FFMPEG`.  
**Why**: end user can't be required to install ffmpeg manually ("easy executable").  
**Cost**: increases installer size; needs binaries per platform.

## ADR-007 — Attio: meeting search by time window (not by participants)

**Decision**: search meetings in Attio by `ends_from`/`starts_before`/`timezone` and match emails **on client** over `participants[].email_address`.  
**Why**: the `participants` parameter of `GET /v2/meetings` (beta endpoint) exists in OpenAPI schema but **crashes the server** at runtime — same authenticated call responds 200 in ~0.5s without parameter and times out with it (validated with self-test inside app). Time filter works and matches product flow (match by time + user confirmation).  
**Cost**: fetches more meetings than necessary (limit 50 in window); fine filter is client-side.

## ADR-008 — HTTP with IPv4-only DNS resolver

**Decision**: shared HTTP client (`net.rs`) uses custom resolver that filters getaddrinfo results to IPv4, plus OS native TLS and system proxy disabled.  
**Why**: on networks with IPv6 advertised but no route (user's real case), reqwest tried AAAA address and hung until timeout; `local_address(0.0.0.0)` didn't solve it. Native TLS coexists with antivirus HTTPS inspection (Kaspersky); system proxy injected by antivirus broke connections.  
**Cost**: no IPv6-only network support (acceptable for today's target audience).

## ADR-009 — Rename to Hicorder with non-destructive migration

**Decision**: product/identifier renamed to Hicorder / `com.hicapital.hicorder` (v0.2.0). One-time migration: copies old data folder, fixes absolute paths in copied DB, and replicates old service keychain keys on first read. Nothing is deleted from old side.  
**Why**: new product name; copy (not move) allows trivial rollback.  
**Cost**: duplicated data on disk until user manually deletes old folder.

## ADR-010 — API keys per scope (provider), no fallback

**Decision**: each key is stored under `"<type>:<host>"` (type = `stt` | `summary`). Switching provider doesn't delete previous key; empty scope means **no key**, without falling back to any old key.  
**Why**: with single key per stage, switching provider overwrote the previous one. The fallback that existed in scope's first version was worse: sent one provider's credential to another (MiniMax key going to NVIDIA endpoint) and still made UI show "saved key" on never-configured provider.  
**Cost**: who had single key needs to enter it once on right provider. We don't auto-migrate because there's no record of which provider it belonged to.

## ADR-011 — Support for two CRMs with different note models

**Decision**: Attio and Affinity coexist; user chooses in Settings and each stores its own key. Upload UI is the same, but backend dispatches to active CRM module.  
**Why**: data models differ. In **Attio** a note has single parent, so one note per person and per company is created, linked to meeting. In **Affinity** a note accepts `person_ids` and `organization_ids`, so **one note** covers all links. Also Affinity has no meetings endpoint — the "choose meeting" step uses local calendar (ICS) only to suggest participants.  
**Cost**: result message and flow step 1 vary by CRM; two code paths to maintain.

## ADR-014 — Attio meeting deduplication became ours

**Context**: in 08/2026 Attio announced taking Meetings/Call Recordings out of alpha and **deprecating `external_ref`** — each `POST /v2/meetings` now creates a new meeting, even with same `external_ref`. Before, that field guaranteed reuse.

**Decision**: before creating, query `GET /v2/meetings` in meeting time window and reuse one with same title (`find_existing_meeting`). **And keep sending `external_ref`.**

**Why keep sending deprecated field**: production API version still requires it as mandatory. Removing it now would break calls; keeping it is harmless after migration, because obsolete field is ignored, not rejected. Same binary works on both sides of the change — and app is distributed, so can't sync deploy with Attio's date.

**Cost**: one extra request per upload that creates meeting. Search failure doesn't block: creates anyway, because uploading note matters more than duplicate risk.

**No impact**: the other two announcement items (call recordings now require transcript on 14/10/2026; transcript comes back with call recording) touch endpoints this app never called.

## ADR-013 — Summary via local Claude Code (process, not HTTP)

**Decision**: add "Claude Code (installed on this machine)" as summary provider. Endpoint is sentinel `claude-code://local`; backend detects and executes `claude --print` CLI with transcription via **stdin**, instead of making POST.  
**Why**: it's the only provider that **dispenses registering API key** — authenticates via Claude subscription user already has. For those already using Claude Code, it's the lowest-friction path.  
**Cost and limits** (measured on CLI 2.1.220):
- **Not offline.** Claude Code talks to Anthropic's API; no privacy gain over other providers.
- ~14k context tokens per call just from CLI scaffolding (27k without `--exclude-dynamic-system-prompt-sections`). Consumes subscription quota.
- Requires installation **and authentication** per machine — real barrier for product's non-technical audience. That's why it's advanced option, never default.
- Depends on third-party CLI flags, which have no API stability. Test validates version on each use.

**Why not use Claude Desktop instead**: they're different programs with the **same executable name**. Desktop (Electron, ~232 MB in `AnthropicClaude/`) doesn't embed CLI — it spawns external `claude` via `node-pty` and doesn't expose `--print`. Installing Desktop doesn't give you CLI. Reading Desktop's OAuth token to authenticate Hicorder was **discarded**: it's credential misuse from another app and would break on first refresh.

**Containment**: `--max-turns 1` + `--disallowed-tools` (Bash/Read/Write/Edit/Glob/Grep/WebFetch/WebSearch/Task/NotebookEdit) prevent CLI from becoming agent with disk access; execution in temp folder, not user directory; transcription via stdin because 1h meeting bursts ~32k command-line character limit on Windows.

## ADR-012 — Whisper dictionary limited to 65 words

**Decision**: Whisper's `prompt` field receives editable dictionary, with 40 factory terms and 65-word ceiling in UI.  
**Why**: `prompt` improves proper names, acronyms and jargon, but API cuts at **224 tokens** and silently discards excess. 40 terms occupy ~133 tokens, leaving ~91 for 25 user can add — the 65 limit is what actually fits. UI estimates tokens and warns before cutoff.  
**Cost**: token estimate is approximate (~4 chars/token); warning may fire slightly before or after real limit.
