# Roadmap — PR by PR

Each PR is a logical, testable, mergeable unit. Check boxes as completed. Branch per PR: `pr1-scaffold`, `pr2-audio-core`, etc.

---

## PR0 — Repo + planning + docs ✅ (this session)

**Goal**: repo structure and handoff documentation.
- [x] `git init`, local identity, `.gitignore`, LICENSE, NOTICE
- [x] README, ARCHITECTURE, ROADMAP, DECISIONS, PROVIDERS, CONTINUATION
- [x] Initial commit

---

## PR1 — Tauri 2 scaffold ✅

**Goal**: app opens empty window, builds on 3 OSes.
- [x] Rust stable (MSVC) + VS Build Tools (C++ + Win11 SDK) installed on this machine. ffmpeg postponed to PR4 (sidecar only in prod).
- [x] `create-tauri-app` (Tauri 2, Vite + React + TS) merged into repo root
- [x] Backend folder structure (`audio/`, `encode/`, `transcription/`, `storage/`, `settings/`, `commands/`) with stubs
- [x] Base layout of 4 screens (Record, Recordings, Transcription, Settings), empty
- [x] `cargo build` compiles clean (3m48s, no warnings); frontend builds (tsc + vite). Just need to run `npm run tauri dev` to see window.
- [ ] Minimal CI (lint + build) optional — not done

**Acceptance**: `npm run tauri dev` opens app. Compilation validated on Windows; validate Mac/Linux build when access available.

---

## PR2 — Core audio capture

> Note: meetily does NOT implement Win/Linux loopback (macOS only); the rest bail!. So
> we built loopback ourselves. Split into 2a/2b/2c.

### PR2a — Microphone (cross-platform) ✅

- [x] Mic capture via `cpal` (0.15.3) → WAV (`hound`)
- [x] Input device enumeration (`list_input_devices`)
- [x] Level (peak) by polling (`recording_level`); meter in UI
- [x] `Recorder` (start/stop session) + `start_recording`/`stop_recording`/`is_recording` commands
- [x] Functional Record screen (button, timer, meter) + basic list in Recordings
- [x] `cargo build` clean; frontend builds. Records to `app_data/recordings/<id>/mic.wav`

**Acceptance 2a**: `npm run tauri dev`, record, stop → `mic.wav` exists and is audible.

### PR2b — Windows system audio (loopback) ✅ (code; missing real call test)

- [x] WASAPI loopback via `wasapi` crate (0.23) → second track `system.wav` (`audio/system.rs`)
- [x] Mic + system in parallel in same session; `system_path` in result; meter uses max peak
- [x] Loopback failure degrades to mic-only (doesn't lose recording). Compiles clean.
- [ ] **Acceptance (you)**: play audio / make a call, record, check that `system.wav` has others' voices

### PR2c — Linux system audio (monitor source)

- [ ] Capture default sink's `.monitor` via `cpal` (appears as input device)
- [ ] **Acceptance**: same on Linux

---

## PR3 — macOS audio

**Goal**: Mac parity.
- [ ] ScreenCaptureKit bridge (system audio, macOS 13+)
- [ ] Mic via CoreAudio/`cpal`
- [ ] Version detection: <13 → mic only, warn in UI
- [ ] Screen recording permission flow (first run)

**Acceptance**: on macOS 13+, WAV with both voices; on <13, mic only without crash.

---

## PR4 — Encode + storage + recordings UI ✅ (code; missing runtime test)

**Goal**: lightweight listed recordings.
- [x] Mix mic+system (`amix`) → encode **Opus `.ogg`** mono ~32 kbps, 16 kHz (`encode/mod.rs` calls ffmpeg). Raw WAVs deleted after encode.
- [x] SQLite (rusqlite bundled): `recordings` table; `storage/mod.rs` (open/insert/list)
- [x] `stop_recording` (encode+persist) and `list_recordings` commands; UI lists date/duration/size and persists across runs
- [x] ffmpeg installed for dev (winget Gyan.FFmpeg). Compiles clean.
- [ ] **Packaged ffmpeg sidecar** (download on build) → moved to PR7 (packaging), for prod without installing anything
- [ ] Play/open file in UI → PR6
- [ ] **Acceptance (you)**: record→stop generates small `.ogg`, appears in list, and stays there after reopening app

---

## PR5 — Transcription + language ✅ (code; missing MiniMax endpoint for real test)

**Goal**: transcribe and copy.
- [x] `Transcriber` trait + `OpenAiCompatible` provider (multipart, Bearer) — `transcription/mod.rs`. Endpoint + model configurable in UI.
- [x] Language selector (default **pt**) in Transcription tab and default language in Settings
- [x] API key in **keychain** (crate `keyring`, per-OS); password field in Settings (never displayed)
- [x] `transcripts` table; display text; **Copy** button (clipboard)
- [x] In-progress/error states; `transcribe` command is async + `spawn_blocking` (doesn't freeze UI). Compiles clean.
- [ ] **Acceptance (you)**: configure MiniMax endpoint/model/key → transcribe a recording → pt-BR text → Copy

**Dependency**: confirm MiniMax ASR endpoint/model (key is Subscription Key `sk-cp`, sent as Bearer) — see [PROVIDERS.md](PROVIDERS.md). Factory default points to OpenAI Whisper as working path.

---

## PR6 — Track-labeled transcription (You/Participants) ✅ (code verified by `cargo check`)

**Goal**: free 2-sided diarization using separate mic/system tracks.
- [x] Stop mixing: records `mic.webm` (You) + `system.webm` (Participants) separately
- [x] `recordings.system_path` (defensive ALTER TABLE migration); RecordingRow gains system_path
- [x] Provider returns **timestamped segments** (`verbose_json`); transcribe calls mic+system, interleaves by time, labels. System failure degrades to mic-only.
- [x] `cargo check` clean (exe build blocked locally by Kaspersky; see CONTINUATION)
- [ ] **Acceptance (you)**: transcribe a call → text like `[mm:ss] You: ...` / `[mm:ss] Participants: ...`

## PR6.5 — Delete recording ✅ (code verified by `cargo check`)

- [x] `delete_recording(id)`: deletes recording folder (mic.webm + system.webm) + row in recordings + transcript
- [x] **Delete** button per item in Recordings tab, with confirmation; list updates
- [ ] **Acceptance (you)**: delete removes from list and disk
- (Settings screen already exists since PR5.) "Record all" toggle = phase 2

---

## PR7 — Packaging and distribution

Public repo: github.com/GeanChu/hicorder. Free signing strategy: **SignPath Foundation** (OSS) + report FP to Kaspersky.

### PR7a — CI/Release (GitHub Actions) ✅ (code; missing cloud run)

- [x] `.github/workflows/ci.yml`: builds on push (ubuntu/macos/windows) — **1st Mac/Linux compilation**, finds platform bugs
- [x] `.github/workflows/release.yml`: `tauri-action` builds installers (.msi/NSIS, .dmg universal, .deb/.AppImage) on tag `v*` → draft Release. Cloud build = no Kaspersky.
- [ ] **Acceptance**: push → green CI on 3 OSes; create tag `v0.1.0` → installers in Release

### PR7b — Packaged ffmpeg (self-contained app) ✅ (code; packaging validates in release)

- [x] ffmpeg as Tauri **resource** (`tauri.conf` `resources: ["resources/*"]`); `encode::mix_to_opus` receives path; `resolve_ffmpeg` uses resource (prod) with PATH/`CALLREC_FFMPEG` fallback (dev)
- [x] `release.yml` downloads static ffmpeg per platform (gyan/johnvansickle/evermeet) → `src-tauri/resources/` before bundle. Binary gitignored; `.gitkeep` keeps folder.
- [ ] **Acceptance**: cut tag `v0.1.0` → installers with embedded ffmpeg → install on PC without ffmpeg and record/transcribe works

### PR7c — Signing (SignPath Foundation, free for OSS)

- [ ] Apply at signpath.io/foundation (OSS project approved) → configure signing step in release.yml
- [ ] Alternative/complement: report installer to Kaspersky (opentip.kaspersky.com) + team Kaspersky exclusion policy
- [ ] Release instructions (Gatekeeper/SmartScreen) in README

**Final acceptance**: team installs and runs from scratch, no AV/security warning.

---

# Post-core features (requested after core)

## PR8 — Play button ✅ (code verified by `cargo check`)

- [x] `stop_recording` also generates mixed `recording.webm` (for playback only), plus separate tracks
- [x] Asset protocol enabled (`tauri.conf` scope `$APPDATA/recordings/**`, feature `protocol-asset`); in-app `<audio>` player via `convertFileSrc`
- [x] Play/Close button per recording in Recordings tab
- [ ] **Acceptance (you)**: Play plays both sides. (Webm/opus plays in WebView2/Win; validate on Mac later.)

## PR9 — Summary via MiniMax-M3 (optional) ✅ (code verified by `cargo check`)

- [x] `summary/mod.rs`: OpenAI-compat chat-completions (`generate_summary`, async+spawn_blocking) → default `https://api.minimax.io/v1/chat/completions`, `MiniMax-M3`, Bearer sk-cp. Summary prompt in pt-BR.
- [x] Separate labeled settings: **Transcription (Groq)** vs **Summary (MiniMax-M3)** section — own endpoint/model/key; **2 keys in keychain** (transcription_api_key + summary_api_key)
- [x] `summaries` table; "Generate summary" button in Transcription tab (only enabled with configured key) + copy; delete deletes summary too
- [ ] **Acceptance (you)**: with sk-cp configured in Summary → transcribe → Generate summary → M3 text

## PR10 — Design pass ✅ (frontend compiles; verified in preview)

- [x] Design system in CSS variables (color/space/typography/radius/shadow), light + dark mode
- [x] Sidebar with inline SVG icons + brand; nav with active state; cards in recordings; record button in pill with indicator; meter with gradient; refined forms and sections; empty states with icon
- [x] No framework (pure CSS). tsc green; renders in vite preview (screenshot hung in environment, but inspects confirm styles)

## PR11 — Calendar (was phase 2). Provider = **ICS URL** (no OAuth).

### PR11a — ICS: list meetings + manual enable ✅ (code verified by `cargo check`)

- [x] `meetings/mod.rs`: fetches ICS URL (reqwest), parses VEVENTs (`ical`), dates with timezone (`chrono`/`chrono-tz`: Z/TZID/naive). RRULE not expanded (v1).
- [x] storage: `meetings` table (upsert preserves `record_enabled`); commands `refresh_meetings`/`list_meetings`/`set_meeting_record`
- [x] Config: **Calendar URL (ICS)** field + **"Record all"** toggle. **Calendar** tab lists upcoming meetings with "Record" checkbox per meeting
- [ ] **Acceptance (you)**: paste ICS URL → Refresh → see meetings → mark which to record

### PR11b — Tray + manual start/stop ✅ (code verified by `cargo check`)

- [x] Tray icon (`tray.rs`, feature `tray-icon`) with menu: start/stop recording, open, quit; tooltip indicates "RECORDING"
- [x] Closing window **minimizes to tray** (app keeps running for auto-record); left-click on tray reopens
- [x] start/stop refactored to `*_core` (callable by command/tray/scheduler); `recording-changed` event syncs UI (tray + screens)
- [x] `tauri-plugin-notification` initialized (used in PR11c)
- [ ] **Acceptance (you)**: icon in tray; start/stop from it; close window = stays in tray; tooltip changes

### PR11c — Auto-record + alerts ✅ (code verified by `cargo check`)

- [x] `scheduler.rs`: thread that every 30s auto-**starts** recording for enabled in-progress meeting (once per meeting, via `triggered` set)
- [x] Recorder stores `meeting_end_ms`; `should_alert_end` (fires 1x at end) and `should_auto_stop` (end+1h)
- [x] Notifications (`tauri-plugin-notification`): "meeting started", "meeting ended — stop recommended", "recording stopped (end+1h)". Stopping remains **manual**.
- [ ] **Acceptance (you)**: mark upcoming meeting in Calendar → app starts on its own at time; notifies at end; stops on its own at end+1h. (Windows toast may require installed app; start/stop logic runs either way.)

---

## PR12 — Upload to Attio (CRM) ✅ (tested by user)

- [x] `attio/mod.rs`: HTTP client — `list_meetings` by **time window** (`ends_from`/`starts_before`/`timezone`; `participants` filter on beta endpoint crashes server — ADR-007), `find_or_create_meeting`, `find_person_by_email`, `create_note` (note per participant with `meeting_id`)
- [x] Attio key in keychain; `attio_find_meetings` + `attio_upload` commands (async+spawn_blocking)
- [x] UI (Transcription tab): "Upload transcript/summary" → searches meetings by **recording time** → choose candidate or create new → suggested participants as checkboxes (user comes unchecked) + manual emails → "Confirm and upload note"
- [x] Filter by user meetings (field "Your email on Attio" in Settings)
- [x] **Acceptance**: search and upload working (validated by user)

## PR13 — Quality of life + rebrand ✅ (this session)

- [x] Provider and model selects per stage (STT: Groq/OpenAI/Fireworks; Summary: OpenAI/Claude/Gemini/MiniMax; + Custom), with help on where to get each key
- [x] **Test** button per key (STT via GET /models; summary via 1-token completions; Attio via GET /v2/meetings)
- [x] Friendly error messages (`logs::humanize`) + **persistent log** (`callrec.log`, 1MB rotation) with View/Clear in Settings
- [x] `net.rs` with IPv4-only DNS resolver (ADR-008) — fixes timeout on network with IPv6 without route
- [x] **Rename to Hicorder** (v0.2.0): identifier `com.hicapital.hicorder`, non-destructive data + keychain migration, new logo (waveform) and complete icon set, bundle metadata (AV), docs revised
- [x] SignPath Foundation prep: [SIGNING.md](SIGNING.md), SECURITY.md

## PR14 — Home, export, autostart and releases ✅ (this session)

- [x] **Home** = calendar as main screen (fixed record button at top); Settings becomes gear in sidebar footer; Transcription tab merged into **Recordings** (play/rename/delete + export)
- [x] Calendar: participants/location/**link** ("Join call" button), **current** meeting highlight, **Start Recording** button per meeting, "Schedule Recording" checkbox, periodic ICS refresh in scheduler
- [x] Recordings with **title** (meeting name or "Manual recording") + rename; **export audio** MP3/WAV/OGG (`tauri-plugin-dialog` + ffmpeg; ogg via libopus)
- [x] **Theme** light/dark/automatic; transcription in **chat**; summary removes `<think>` from reasoning models
- [x] **Auto-launch** with OS (`tauri-plugin-autostart`, on by default, `--minimized` for tray) + toggle
- [x] Meeting toast created in **main thread** (0x80070057); notifications/permission on macOS via `run_on_main_thread`
- [x] **Releases**: installers v0.2.0→v0.2.3 on 3 OSes (CI checks libopus/libmp3lame); README with download per OS + SignPath attribution
- [x] Installed build fixes: ffmpeg `resources/` + `+x` on unix; SQLite WAL + busy_timeout; **log** covering recording/calendar/player/UI (`logged()` + `log_client`)

---

## Backlog / Phase 2 — Calendar + auto-record

**Goal**: record meetings automatically.
- [ ] Calendar connectors (Google/Outlook/ICS-CalDAV)
- [ ] `meetings` table; list upcoming meetings
- [ ] **Checkbox per meeting** to enable recording
- [ ] Auto-start recording when enabled meeting begins
- [ ] **"Record all" default** configuration
- [ ] (Optional) active meeting app detection (Zoom/Meet/Teams) for more precise trigger

---

## Conventions

- Commits: `feat|fix|refactor|chore|docs|test: <imperative summary in English>` (body in PT if useful)
- Pinned versions in dependencies; lockfiles versioned; never delete lockfile
- Update [CONTINUATION.md](CONTINUATION.md) at end of each session
