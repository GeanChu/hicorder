# Hicorder

<p align="center">
  <img src="src-tauri/icons/128x128.png" alt="Hicorder" width="96" height="96" />
</p>

<p align="center">
  <strong>Open-source meeting recorder with AI transcription and CRM integration</strong>
</p>

<p align="center">
  <a href="https://github.com/GeanChu/hicorder/releases/latest">
    <img alt="GitHub release" src="https://img.shields.io/github/v/release/GeanChu/hicorder">
  </a>
  <a href="https://github.com/GeanChu/hicorder/blob/main/LICENSE">
    <img alt="License" src="https://img.shields.io/github/license/GeanChu/hicorder">
  </a>
  <a href="https://github.com/GeanChu/hicorder/actions">
    <img alt="CI" src="https://img.shields.io/github/actions/workflow/status/GeanChu/hicorder/ci.yml">
  </a>
</p>

<p align="center">
  <a href="#documentation">Documentation</a> · <a href="CONTRIBUTING.md">Contributing</a> · <a href="#quick-start">Quick Start</a>
</p>

---

## What is Hicorder?

Hicorder is a desktop meeting recorder built for teams that need flexible, private, and powerful meeting notes. It captures microphone and system audio, transcribes with AI, generates summaries, and pushes notes directly to your CRM.

Born as an alternative to Granola, Hicorder gives you complete control over your AI providers, prompts, and data. Originally created at [Hi Capital](https://hi.capital) (Honey Island VC, Curitiba, Brazil) by Gean Chu, it's now open source for the community.

**What makes Hicorder different:**
- **Choose your AI**: Works with Groq, OpenAI, Claude, Gemini, MiniMax, NVIDIA NIM, or your own endpoints
- **Customizable prompts**: Edit the base prompt and create a library of named prompts for different meeting types
- **Native CRM integration**: Push transcripts and summaries to Attio or Affinity with participant matching
- **Privacy-first**: All data stays local; only chosen AI providers see your audio/transcripts
- **Venture Capital focused**: Built-in terminology dictionary for VC/investment meetings, easily extensible

## Key Features

### Recording & Audio
- **One-click recording** with live audio level meter and built-in player
- **Dual-track capture**: Records microphone and system audio (participants' voices) separately
- **Lightweight storage**: Opus encoding at ~32 kbps (~7-15 MB/hour), encoded during recording so crashes don't lose data
- **Live note-taking**: Side panel during recording for manual annotations that enrich the summary later
- **Export audio**: MP3, WAV, or OGG formats
- **Upload audio**: Transcribe existing recordings

### AI Transcription
- **Provider flexibility**: Choose Groq Whisper (default, free tier), OpenAI, Fireworks AI, or custom endpoints
- **Language per transcription**: Defaults to Portuguese (Brazil), configurable per recording
- **Chat format**: Microphone appears as "You" (right side), system audio as "Participants" (left side)
- **Terminology dictionary**: Pre-loaded with 40 VC/investment terms; add your own names, acronyms, jargon
- **Hallucination filtering**: Removes Whisper's silence-filling artifacts automatically
- **Search**: Full-text search within transcripts

### AI Summary
- **Multi-provider support**: OpenAI, Claude, Gemini, MiniMax, NVIDIA NIM, or your own endpoints
- **Claude Code integration**: Use Claude installed locally without an API key (uses your Claude subscription)
- **Editable base prompt**: Customize the default summarization instructions
- **Prompt library**: Create named prompts for different meeting types (investor updates, 1-on-1s, etc.)
- **Manual notes integration**: Your live annotations are included in the summary prompt
- **Search**: Full-text search within summaries

### Calendar & Automation
- **ICS calendar integration**: See upcoming meetings with participants, location, and call links
- **Current meeting highlight**: Visual indicator for meetings happening now
- **One-click start**: "Start Recording" button per meeting, or "Schedule Recording" for auto-start
- **Forgotten recording reminder**: Hourly reminder with Stop button
- **Auto-stop**: Configurable auto-stop (default 2 hours) prevents runaway recordings
- **System tray**: Always-on tray icon with recording indicator

### CRM Integration
- **Attio & Affinity support**: Push transcripts, summaries, or notes as CRM notes
- **Time-based meeting matching**: Finds the CRM meeting by recording timestamp
- **Participant suggestions**: Auto-suggests participants and their companies based on meeting data
- **Flexible note creation**: Choose what to upload and to which participants/companies

### Quality of Life
- **Auto-update**: Checks for updates on launch and every 24 hours
- **Auto-start with OS**: Launches minimized to tray (enabled by default)
- **Themes**: Light, dark, or automatic (follows system)
- **Persistent logs**: Troubleshooting logs accessible from Settings
- **Test your keys**: Built-in test button for each API provider
- **Friendly error messages**: Plain-language errors with troubleshooting hints

## Platform Support

| Platform | Microphone | System Audio | Notes |
|----------|-----------|--------------|-------|
| **Windows** | ✅ | ✅ | WASAPI loopback |
| **macOS** | ✅ | ✅ | ScreenCaptureKit (macOS 13+, requires Screen Recording permission) |
| **Linux** | ✅ | ⏳ | System audio coming soon (PulseAudio monitor) |

## Installation

Download the latest release for your operating system from the [Releases page](https://github.com/GeanChu/hicorder/releases/latest).

### Windows

Download `Hicorder_x.y.z_x64-setup.exe` (NSIS installer) or `Hicorder_x.y.z_x64_en-US.msi`.

1. Run the installer and follow the wizard.
2. If **Windows SmartScreen** warns ("Windows protected your PC"): click **More info** → **Run anyway**.
3. If your antivirus blocks it, see [Antivirus Warnings](#antivirus-warnings) below.

### macOS (Intel & Apple Silicon)

Download `Hicorder_x.y.z_universal.dmg` (universal binary).

1. Open the `.dmg` and drag **Hicorder** to your Applications folder.
2. On first launch (app not yet notarized): **right-click** Hicorder → **Open** → **Open**. Or go to **System Settings → Privacy & Security → Open Anyway**.
3. If macOS blocks it ("cannot be verified" or "is damaged"), run in **Terminal** and open again:
   ```bash
   xattr -dr com.apple.quarantine /Applications/Hicorder.app
   # If still says "damaged" (Apple Silicon):
   codesign --force --deep -s - /Applications/Hicorder.app
   ```
4. Grant **Microphone** permission when prompted.
5. To record **call audio** (participants' voices), macOS requires **Screen Recording** permission: when you first record, go to **System Settings → Privacy & Security → Screen Recording**, enable **Hicorder**, and **restart the app**. Without this, it records microphone only.

### Linux

Download `AppImage` (portable), `.deb` (Debian/Ubuntu), or `.rpm` (Fedora/RHEL).

```bash
# AppImage (portable, no installation)
chmod +x Hicorder_*_amd64.AppImage && ./Hicorder_*_amd64.AppImage

# Debian / Ubuntu
sudo apt install ./Hicorder_*_amd64.deb

# Fedora / RHEL
sudo dnf install ./Hicorder-*.x86_64.rpm
```

> **Note:** Linux currently records microphone only. System audio (PulseAudio monitor) is on the roadmap. All other features (calendar, transcription, summary, CRM) work normally.

### Antivirus Warnings

The installers are **not yet digitally signed** (code signing via [SignPath Foundation](https://signpath.org/) is in progress — see [docs/SIGNING.md](docs/SIGNING.md)). Windows SmartScreen or antivirus software may show a warning on first run. Here's what we do to minimize this:

- 100% open source code in this repository; binaries are built by public CI (GitHub Actions) from this code.
- Full metadata embedded in executables and installers (publisher, version, description, website).
- False positives are reported to vendors (Kaspersky via opentip.kaspersky.com, Microsoft via submission portal).

If your antivirus blocks it: add Hicorder as a trusted application or download again after code signing is active. For SmartScreen: "More info" → "Run anyway".

Code signing for Windows executables is provided by **[SignPath Foundation](https://signpath.org/)** (free signing program for open source projects). *Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org/).*

## Quick Start

1. **Install Hicorder** and launch it.
2. **Configure your AI providers** in Settings:
   - **Transcription**: Get an API key from [Groq](https://console.groq.com/keys) (free tier available), [OpenAI](https://platform.openai.com/api-keys), or [Fireworks AI](https://fireworks.ai).
   - **Summary**: Get an API key from your preferred LLM provider (OpenAI, Claude, Gemini, etc.) or use [Claude Code](https://www.anthropic.com/claude/download) locally (no key needed).
   - Test each key with the "Test" button.
3. **(Optional) Connect your calendar**: Paste your private ICS URL in Settings → Connections → Calendar. [How to get your ICS URL](https://support.google.com/calendar/answer/37648).
4. **(Optional) Connect your CRM**: Add your Attio or Affinity API key in Settings → Connections → CRM.
5. **Start recording**: Click the big red button on the Home tab, or click "Start Recording" next to an upcoming meeting.
6. **Transcribe**: After stopping, go to the Recordings tab, select your recording, and click "Transcribe".
7. **Summarize**: After transcription completes, click "Generate Summary".
8. **(Optional) Push to CRM**: Click "Upload to CRM", select the meeting, choose participants, and confirm.

## Comparison with Granola

Hicorder was created as an alternative to Granola. Here's how they compare:

| Feature | Hicorder | Granola |
|---------|----------|---------|
| **Open Source** | ✅ Yes (MIT) | ❌ No |
| **Pricing** | Free (you provide API keys) | Freemium (capped free tier, paid plans) |
| **AI Provider Choice** | Any OpenAI-compatible endpoint | Granola-managed |
| **Custom Prompts** | ✅ Editable base + prompt library | Limited customization |
| **CRM Integrations** | Attio, Affinity (extendable) | Multiple (proprietary) |
| **Data Privacy** | 100% local, you choose providers | Cloud-first |
| **Venture Capital Focus** | Yes (built-in VC dictionary) | General business |
| **Platform** | Desktop (Windows, macOS, Linux) | Desktop + mobile |
| **System Audio** | Native (Windows/macOS) | Native |
| **Meeting Detection** | ICS calendar | Calendar + app detection |

**When to choose Hicorder:**
- You want control over your AI providers and costs
- You need customizable prompts for specific meeting types
- You prefer local-first data storage
- You're in Venture Capital or want extensible CRM integrations

**When to choose Granola:**
- You prefer a managed service with no setup
- You want mobile apps
- You need integrations Hicorder doesn't support yet

## Architecture

Hicorder is built with [Tauri 2](https://tauri.app) (Rust backend + React frontend):

- **Audio capture**: `cpal` for microphone, WASAPI loopback (Windows), ScreenCaptureKit (macOS)
- **Audio encoding**: Embedded `ffmpeg` for Opus/Ogg encoding during recording (~32 kbps, 16 kHz)
- **Storage**: SQLite for recordings, transcripts, summaries, notes, prompts, meetings
- **Security**: API keys stored in OS keychain (Windows/Linux) or protected file (macOS)
- **Networking**: Reqwest with native TLS, IPv4-only DNS resolver, explicit timeouts

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for details.

## Documentation

- [ARCHITECTURE.md](docs/ARCHITECTURE.md) — System design and module breakdown
- [ROADMAP.md](docs/ROADMAP.md) — What's done and what's coming
- [DECISIONS.md](docs/DECISIONS.md) — Architecture Decision Records (ADRs)
- [PROVIDERS.md](docs/PROVIDERS.md) — Supported AI providers and configuration
- [SIGNING.md](docs/SIGNING.md) — Code signing setup (SignPath Foundation)
- [CONTINUATION.md](docs/CONTINUATION.md) — Current development state and handoff notes

## Development Setup

See [CONTRIBUTING.md](CONTRIBUTING.md) for detailed development instructions.

**Quick start:**

Prerequisites:
- Node.js 20+
- Rust stable (with MSVC on Windows: Visual Studio Build Tools with C++ and Windows SDK)
- Tauri 2 system dependencies: [Installation guide](https://v2.tauri.app/start/prerequisites/)
- `ffmpeg` in PATH (dev only; production embeds it)

```bash
git clone https://github.com/GeanChu/hicorder.git
cd hicorder
npm install
npm run tauri dev
```

Build without bundling (for testing):
```bash
npm run tauri build -- --no-bundle
```

Run tests:
```bash
cd src-tauri
cargo test
```

## Privacy

- **Local-first**: Audio, transcripts, and summaries are stored locally on your computer (SQLite + files).
- **AI providers**: Transcription and summary send audio/text **only** to the AI provider you configure. We recommend reading each provider's privacy policy.
- **CRM upload**: Happens only when you explicitly confirm the upload.
- **No telemetry**: Hicorder does not collect usage data, analytics, or crash reports.
- **Calendar URL**: Your ICS URL is a credential (anyone with it can read your calendar). It's stored locally and never sent anywhere except to fetch the calendar data.

## Security

We take security seriously. If you discover a vulnerability, please report it responsibly:

- **Preferred**: Submit a private vulnerability report via [GitHub Security Advisories](https://github.com/GeanChu/hicorder/security/advisories/new)
- **Alternative**: Email **financeiro@hi.capital**
- **Do not** open public issues for security reports

See [SECURITY.md](SECURITY.md) for details.

## Roadmap & Contributing

See [ROADMAP.md](docs/ROADMAP.md) for planned features and [CONTRIBUTING.md](CONTRIBUTING.md) for how to contribute.

**Areas where the community can help:**
- **Linux system audio** (PulseAudio monitor source)
- **New CRM integrations** (see [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the integration interface)
- **New AI providers** (OpenAI-compatible endpoints are drop-in)
- **Translations** (currently English and Portuguese; UI is localizable)
- **macOS notarization** (requires Apple Developer account)
- **Documentation improvements**
- **Bug reports and feature requests**

## License

MIT License. See [LICENSE](LICENSE) and [NOTICE](NOTICE).

This project reuses and adapts code from [meetily](https://github.com/Zackriya-Solutions/meetily) (MIT), specifically the cross-platform audio capture pipeline.

## Acknowledgments

- Built with [Tauri](https://tauri.app), [React](https://react.dev), and [Rust](https://www.rust-lang.org)
- Audio capture inspired by [meetily](https://github.com/Zackriya-Solutions/meetily)
- Free code signing by [SignPath Foundation](https://signpath.org/)

## Links

- [Website](https://hi.capital)
- [GitHub](https://github.com/GeanChu/hicorder)
- [Issues](https://github.com/GeanChu/hicorder/issues)
- [Releases](https://github.com/GeanChu/hicorder/releases)
- [Discussion](https://github.com/GeanChu/hicorder/discussions)

---

Made with ❤️ by [Gean Chu](https://github.com/GeanChu) and [Hi Capital](https://hi.capital) • Open source for the community
