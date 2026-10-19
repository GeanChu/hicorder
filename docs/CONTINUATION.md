# Continuation / Handoff

Document so the next session knows exactly where we left off and how to proceed.

## Current State (2026-08-11, v0.2.51)

**Workflow changed.** Windows 11 **Smart App Control** turned on on this machine (it's irreversible once disabled, so it stayed on) and blocks any freshly-compiled unsigned executable — `cargo build`, `cargo test` and `tauri dev` no longer run locally (`os error 4551`). The cycle is now: branch → push → PR → **CI compiles and tests** on 3 OSes → merge. Frontend `npm run build` still works locally (it's TypeScript, doesn't generate executable), so UI can still be inspected in browser.

**v0.2.52 — update delivery was broken for the whole team:**
- New version check was silent `catch { }`, with retry only in **24h**. Since the app has auto-launch and starts with Windows login, the network is almost never ready at that instant — so the check failed on boot of any machine, without trace in log, and the person went a whole day without receiving. Proven in log: app started 22:17:49, release was live since 22:15:50, ICS failing 22:18:24. Now retries at 30/60/90/120s and logs failures.
- ICS: timeout 30s → 120s (in reqwest it covers **body reading**, not just connection — full calendar burst in the middle and `error decoding response body` error looked like corruption), 3 retries, and now uses `net::describe()` to show root cause.
- Attio: creating meeting requires participant. UI guard accepted "person **or** company", so company-only passed and broke with 400 "No participants were provided".

**v0.2.51:**
- **`cargo test` on CI** — before it only compiled. On first run it found a published bug: the filter that separates Claude Desktop from Claude Code only worked on Windows (`path.components()` doesn't recognize `\` outside it, and on macOS Desktop lives in `Claude.app`). Was in v0.2.48–0.2.50.
- **Long audio is split into parts** — meeting above ~1h45 gave `413 Payload Too Large` (Opus 32 kbps ≈ 14.4 MB/h against 25 MB ceiling of providers). Splits preventively above 20 MB and reactively on any 413; timestamps shifted to meeting timeline; failing part becomes visible marker instead of bringing everything down.
- **Attio: deduplication became ours** (ADR-014) — `external_ref` was deprecated and POST now always creates a new meeting. The other two items in the announcement (call recordings) don't touch us.

**New in previous session (v0.2.48–0.2.50):**
- **Alert on single channel** (v0.2.49): native notification was removed (plugin included) and every alert became toast window, which is the one with button. Five types, new "Join call" button, and an × to dismiss — window has no decoration or taskbar, so without × a buttonless toast would get stuck on screen.
- **Calendar stops showing phantom meeting** (v0.2.50): meeting deleted in Google kept showing, and if marked for recording the scheduler started recording a nonexistent meeting. Two causes: sync was only additive (only removal was by time) and `STATUS:CANCELLED` was ignored. Now ICS is authoritative — with safeguards not to delete on fetch failure or empty feed — and canceled/declined-by-me events are discarded in parser.

**From previous session (v0.2.48):**
- **Summary via local Claude Code** (ADR-013): provider that executes machine's `claude --print` CLI instead of HTTP; only one without API key. "Test installation" button validates binary + version + real call. Tested by user in compiled app.
- **Security fix**: ICS URL (which is credential — whoever has the link reads the whole calendar) fell raw into `callrec.log`. `redact()` now masks URL path segments with 16+ characters, with test locking the 7 legitimate endpoints.
- **Dev environment reinstalled from scratch** on this machine: Node 24.18.1, Rust stable-MSVC, VS Build Tools 17.14 (MSVC 14.44 + Windows SDK 10.0.26100), ffmpeg 8.1.2, gh 2.97.

**Traps discovered this session (don't repeat):**
- `cargo build --release` **doesn't embed frontend** — who decides dev vs. production is Tauri CLI. Binary comes out pointing to `localhost:1420` and window shows `ERR_CONNECTION_REFUSED`. To generate testable executable: `npm run tauri build -- --no-bundle`.
- **Claude Desktop ≠ Claude Code**, and both executables are called `claude.exe`. Desktop (Electron, in `AnthropicClaude/`) doesn't embed CLI — spawns external via `node-pty`. Binary search discards paths under `AnthropicClaude/` and requires `--version` to contain "Claude Code".
- Running dev app **shares database, recordings and keychain** with installed one (same `identifier`). Close tray one first, and backup `callrec.db`.

**Open user pendencies**: clean old logs and **rotate private ICS URL** in Google Calendar — leaked link remains in already-saved logs and Dropbox history.

## Previous State (2026-07-27, v0.2.44 published)

**Functional in production (Windows and macOS tested by user):** recording mic + system in separate tracks, **live-encoded** to Opus/Ogg; player (MP3, because of WebKit on macOS); export audio; **live manual notes** on Home; transcription in two interleaved tracks in chat format, with **dictionary** and **hallucination filter**; summary with **editable base prompt + prompt library** and per-summary override; **search** in transcription and summary; ICS calendar with RRULE; auto-start per meeting, **hourly reminder** and **configurable auto-stop** (default 2h); upload to **Attio or Affinity** (people + companies); auto-launch; tray; theme; auto-update; persistent log.

**Repo** `GeanChu/hicorder` (public). Releases via CI (draft → publish with `gh release edit --draft=false --latest`). Only the 3 most recent releases stay on GitHub (cleanup done 2026-07-08); old tags remain.

### What broke and how it was resolved (don't regress)

- **Empty call audio on Windows** — loopback's `initialize_client` received device's minimum period in `buffer_duration_hns`; on C-Media drivers this gives live but mute stream (0 events). Fix: **0** (device default). Latent bug since v0.2.6, driver-state dependent.
- **Mute player on macOS** — WebKit doesn't decode Ogg/Opus. Player now receives on-demand generated `playback.mp3`.
- **macOS CI breaking every release** — `apple-metal` (ScreenCaptureKit's transitive dep) compiles Swift against macOS 26 SDK. Fix: runner `macos-15` + step that selects Xcode 26. Capture stays behind `macos-system-audio` feature, enabled only on macOS.
- **Dead auto-update on Windows/Linux** — ffmpeg download from single host failed and release came out with macOS only, so `latest.json` didn't have these platforms. Fix: GitHub mirror (BtbN, release n7.1) with retry and fallback. **Always check all 3 jobs before publishing** (`gh run view --json jobs`); `gh run watch --exit-status` returns 0 even with matrix job failing.
- **One provider's key going to another** — there was fallback to old single key when scope was empty. Removed: empty scope = no key.
- **Key test failing on reasoning model** — `max_tokens: 1` doesn't let model produce anything. Now 16.
- **Toast stuck on screen** — window without decoration/taskbar couldn't be closed if webview froze. Now: `destroy()`, auto-dismiss in 60s and backend safety destruction in 90s.

## Pendencies (in suggested order)

1. **Review default dictionary** — user asked to swap terms from current 40 (list sent in chat); awaiting new list.
2. **Affinity in real use** — implemented and compiling, but still **not tested against real API**. Validate: `GET /auth/whoami`, person search by email, `POST /notes` with multiple links.
3. **SignPath Foundation** — user needs to submit manually (embed form cross-origin + reCAPTCHA). Then integrate into `release.yml` ([SIGNING.md](SIGNING.md)).
4. **Linux system audio** (monitor source via cpal) — today Linux records mic only.
5. **macOS notarization** — user **decided not to pay** for Apple Developer (US$ 99/year) for now. Accepted consequence: Screen Recording permission drops each update (ad-hoc signature changes). Recourse: `tccutil reset ScreenCapture com.hicapital.hicorder` + reopen.
6. Report installer false positive to Kaspersky/Microsoft — better after signing (hash changes).

## Environment gotchas (user's machine, Windows)

- **Kaspersky** gives false positive on `cargo.exe`/`rustc.exe` and can hold audio access (prompt delays capture start). Exclusions configured; worth adding Hicorder and `ffmpeg.exe` as trusted.
- **Dropbox** locks `target/`. Local `.cargo/config.toml` (not committed) moves target to `%LOCALAPPDATA%\callrec-target`.
- **User network has IPv6 without route** — `net.rs` forces IPv4 (ADR-008). Don't remove without testing.
- **Attio**: `participants` filter on GET /v2/meetings crashes server (ADR-007). Don't use it again.

## Useful local tests

```bash
# Real recording pipeline, without installer (Windows):
cargo test --lib rec_smoke -- --ignored --nocapture     # mic + system + levels + track sizes
cargo test --lib system_probe -- --ignored --nocapture  # WASAPI loopback: events/bytes per step
cargo test --lib mic_probe -- --ignored --nocapture     # isolated cpal
```

## Project rules

- Pinned versions; lockfiles versioned and untouchable.
- Before installing new package: check publish date (>7 days) and alerts (socket.dev/osv.dev).
- Frequent documented commits. Never commit `.env`, keys, or broken code.
- API keys only in keychain/protected file; never in logs (not even in `callrec.log`); never returned to UI.
- Update this file at end of each session.
