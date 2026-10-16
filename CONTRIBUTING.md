# Contributing to Hicorder

Thank you for your interest in contributing to Hicorder! This document provides guidelines and instructions for contributing.

## Table of Contents

- [Code of Conduct](#code-of-conduct)
- [How Can I Contribute?](#how-can-i-contribute)
- [Development Setup](#development-setup)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Submitting Changes](#submitting-changes)
- [Adding a New AI Provider](#adding-a-new-ai-provider)
- [Adding a New CRM Integration](#adding-a-new-crm-integration)
- [Testing](#testing)
- [Documentation](#documentation)
- [Getting Help](#getting-help)

## Code of Conduct

This project adheres to the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to financeiro@hi.capital.

## How Can I Contribute?

### Reporting Bugs

Before creating a bug report:
1. Check the [existing issues](https://github.com/GeanChu/hicorder/issues) to see if the problem has already been reported.
2. Check the [Troubleshooting section](README.md#troubleshooting) in the README.
3. Try to reproduce the issue with the latest version.

When filing a bug report, include:
- **Clear title and description**
- **Steps to reproduce** the issue
- **Expected vs. actual behavior**
- **Screenshots or error messages** (check for sensitive data first)
- **System information**: OS, Hicorder version (Help → About)
- **Logs**: Settings → System → View Logs (redact any sensitive information)

**Security issues**: Do NOT open public issues. See [SECURITY.md](SECURITY.md).

### Suggesting Features

Feature requests are welcome! Please:
- Check [existing issues](https://github.com/GeanChu/hicorder/issues) and [discussions](https://github.com/GeanChu/hicorder/discussions) first.
- Use the [Feature Request template](.github/ISSUE_TEMPLATE/feature_request.md).
- Clearly describe the problem you're trying to solve.
- Explain why existing features don't address your need.
- Provide use cases and examples.

### Contributing Code

We welcome pull requests for:
- Bug fixes
- New features (please discuss in an issue first for large changes)
- Documentation improvements
- Test coverage improvements
- Performance optimizations
- New AI provider integrations
- New CRM integrations
- Translations

## Development Setup

### Prerequisites

- **Node.js 20+**: [nodejs.org](https://nodejs.org/)
- **Rust stable**: [rustup.rs](https://rustup.rs/)
  - Windows: Install Visual Studio Build Tools with "Desktop development with C++" and Windows SDK
  - macOS: Install Xcode Command Line Tools (`xcode-select --install`)
  - Linux: See [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)
- **Tauri 2 system dependencies**: [Installation guide](https://v2.tauri.app/start/prerequisites/)
- **ffmpeg** in PATH (dev only; production bundles it)
  - Windows: `winget install Gyan.FFmpeg` or download from [gyan.dev](https://www.gyan.dev/ffmpeg/builds/)
  - macOS: `brew install ffmpeg`
  - Linux: `sudo apt install ffmpeg` or equivalent

### Clone and Setup

```bash
git clone https://github.com/GeanChu/hicorder.git
cd hicorder
npm install
```

### Running in Development

```bash
npm run tauri dev
```

This launches the app with hot-reload for both frontend (Vite) and backend (Rust).

### Building

Build without bundling (faster, for testing):
```bash
npm run tauri build -- --no-bundle
```

Full build with installers (slow):
```bash
npm run build  # Frontend
npm run tauri build  # Backend + bundling
```

Installers will be in `src-tauri/target/release/bundle/`.

### Testing

Run Rust tests:
```bash
cd src-tauri
cargo test
```

Run specific test:
```bash
cargo test test_name
```

Tests marked `#[ignore]` require hardware (microphone/loopback) or network. Run with:
```bash
cargo test -- --ignored --nocapture
```

## Project Structure

```
hicorder/
├── src/                    # React frontend (TypeScript)
│   ├── App.tsx            # Main UI component (tabs, settings)
│   ├── App.css            # Styles (design tokens, light/dark)
│   └── main.tsx           # Entry point
├── src-tauri/             # Rust backend (Tauri)
│   ├── src/
│   │   ├── audio/         # Audio capture (mic + system)
│   │   ├── encode/        # ffmpeg integration (mix, export)
│   │   ├── transcription/ # AI transcription (Whisper)
│   │   ├── summary/       # AI summarization (LLMs)
│   │   ├── attio/         # Attio CRM integration
│   │   ├── affinity/      # Affinity CRM integration
│   │   ├── meetings/      # ICS calendar parsing
│   │   ├── scheduler/     # Auto-start/stop, reminders
│   │   ├── storage/       # SQLite persistence
│   │   ├── settings/      # Settings + keychain
│   │   ├── logs/          # Logging (persistent, redacted)
│   │   ├── net.rs         # HTTP client
│   │   ├── tray.rs        # System tray
│   │   └── commands/      # Tauri IPC commands
│   ├── Cargo.toml         # Rust dependencies
│   └── tauri.conf.json    # Tauri configuration
├── docs/                  # Documentation
├── .github/               # GitHub templates and workflows
├── README.md              # English documentation
├── README.pt-BR.md        # Portuguese documentation
├── CONTRIBUTING.md        # This file
├── CODE_OF_CONDUCT.md     # Code of conduct
├── SECURITY.md            # Security policy
├── LICENSE                # MIT license
└── package.json           # Node dependencies
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed architecture documentation.

## Coding Standards

### Rust

- **Format**: Run `cargo fmt` before committing.
- **Lint**: Run `cargo clippy` and fix warnings.
- **Naming**: Follow [Rust naming conventions](https://rust-lang.github.io/api-guidelines/naming.html).
- **Error handling**: Use `anyhow::Result` for errors; provide context with `.context()`.
- **Secrets**: Never log secrets. Use `logs::redact()` for user-facing error messages.
- **Comments**: Explain "why", not "what". Document public APIs with `///` doc comments.
- **Tests**: Add tests for non-trivial logic. Use `#[ignore]` for hardware/network tests.

### TypeScript/React

- **Format**: Code follows project style (spaces, semicolons).
- **Types**: Use TypeScript types; avoid `any`.
- **Components**: Functional components with hooks.
- **State**: Use `useState` for local state; avoid over-optimization.
- **Comments**: Explain non-obvious UI decisions.

### Commits

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
type(scope): short summary in imperative mood

Longer explanation if needed (optional).
```

Types:
- `feat`: New feature
- `fix`: Bug fix
- `docs`: Documentation only
- `refactor`: Code restructuring without behavior change
- `test`: Adding or updating tests
- `chore`: Maintenance (dependencies, build, etc.)
- `perf`: Performance improvement

Examples:
```
feat(transcription): add support for Fireworks AI provider
fix(audio): prevent empty system audio track on C-Media drivers
docs(readme): add troubleshooting section for Linux
refactor(storage): extract migration logic to separate module
```

### Versioning

- **Lock files**: `package-lock.json` and `Cargo.lock` are committed. Never delete them.
- **Dependencies**: Pin versions. Check publish date (>7 days) and security advisories before adding new dependencies.
- **Bumping versions**: Maintainers handle version bumps in `package.json` and `Cargo.toml`.

## Submitting Changes

### Pull Request Process

1. **Fork** the repository and create a branch from `main`:
   ```bash
   git checkout -b feature/my-feature
   ```

2. **Make your changes**:
   - Write code following the [coding standards](#coding-standards).
   - Add tests if applicable.
   - Update documentation (README, inline comments, `/docs`).

3. **Test locally**:
   ```bash
   npm run build  # Frontend
   cd src-tauri && cargo test && cargo clippy && cargo fmt --check
   ```

4. **Commit** with clear messages (see [Commits](#commits)).

5. **Push** to your fork:
   ```bash
   git push origin feature/my-feature
   ```

6. **Open a Pull Request** against `main`:
   - Use the [PR template](.github/PULL_REQUEST_TEMPLATE.md).
   - Reference related issues (e.g., "Fixes #42").
   - Provide clear description of changes and motivation.
   - Add screenshots for UI changes.
   - Check "Allow edits from maintainers".

7. **CI checks**: The PR must pass automated checks (build + test on all platforms).

8. **Code review**: Address feedback from maintainers. Be patient and collaborative.

9. **Merge**: Maintainers will merge when ready. Squash commits if requested.

### PR Guidelines

- **One PR per feature/fix**: Keep changes focused.
- **No merge commits**: Rebase on `main` if needed (`git pull --rebase upstream main`).
- **Breaking changes**: Document in PR description and update relevant docs.
- **UI changes**: Include before/after screenshots.
- **Performance**: If claiming a performance improvement, provide benchmarks.

## Adding a New AI Provider

Hicorder supports any OpenAI-compatible endpoint. To add a new provider to the UI dropdown:

### For Transcription (Speech-to-Text)

1. **Add provider config** to `src/App.tsx`:
   ```typescript
   {
     id: "my-provider",
     label: "My Provider",
     endpoint: "https://api.myprovider.com/v1/audio/transcriptions",
     models: ["model-1", "model-2"],
     keyHelp: "Get your API key from https://myprovider.com/keys",
   }
   ```

2. **Test** with a real API key. Verify:
   - The provider accepts multipart uploads (`file`, `model`, `language`, `response_format=verbose_json`, `prompt`).
   - `Authorization: Bearer <key>` is used.
   - The response matches OpenAI's verbose JSON format (segments with timestamps).

3. **Document** in `docs/PROVIDERS.md`.

### For Summary (LLM)

1. **Add provider config** to `src/App.tsx`:
   ```typescript
   {
     id: "my-llm",
     label: "My LLM",
     endpoint: "https://api.myllm.com/v1/chat/completions",
     models: ["fast-model", "smart-model"],
     keyHelp: "Get your API key from https://myllm.com/api-keys",
   }
   ```

2. **Test** with a real API key. Verify:
   - The provider accepts JSON chat completions (`model`, `messages`).
   - `Authorization: Bearer <key>` is used.
   - The response matches OpenAI's format (`choices[0].message.content`).

3. **Special handling**: If the provider needs custom `max_tokens` or other parameters, update `src-tauri/src/summary/mod.rs`.

4. **Document** in `docs/PROVIDERS.md`.

## Adding a New CRM Integration

To add a new CRM (e.g., HubSpot, Pipedrive):

1. **Create a new module** in `src-tauri/src/your_crm/mod.rs`:
   ```rust
   use anyhow::{Context, Result};

   pub fn test_key(api_key: &str) -> Result<()> {
       // Test API key validity
   }

   pub fn find_meeting(api_key: &str, recording_start: i64) -> Result<Vec<Meeting>> {
       // Search for meetings by timestamp
   }

   pub fn upload_note(api_key: &str, meeting_id: &str, content: &str) -> Result<()> {
       // Upload note to CRM
   }
   ```

2. **Add settings storage**:
   - In `src-tauri/src/settings/mod.rs`: Add a constant for the key name (e.g., `HUBSPOT_KEY`).
   - In `src/App.tsx`: Add a setting field in the CRM section.

3. **Add UI**:
   - CRM selector dropdown in Settings.
   - API key field with test button.
   - Upload flow in the Recordings tab (reuse existing UI pattern).

4. **Handle API differences**:
   - **One note per person** (like Attio): Create multiple notes.
   - **One note, multiple links** (like Affinity): Create a single note with `person_ids`/`organization_ids`.

5. **Test**:
   - Mock API responses in tests.
   - Test against the real API with a trial account.

6. **Document**:
   - Add to [README.md](README.md) features list.
   - Add to [docs/PROVIDERS.md](docs/PROVIDERS.md) with authentication details.
   - Add to [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

See `src-tauri/src/attio/` and `src-tauri/src/affinity/` as reference implementations.

## Testing

### Unit Tests

```bash
cd src-tauri
cargo test
```

Existing tests cover:
- Secret redaction in logs
- ICS calendar parsing (RRULE, EXDATE, overrides, cancellations)
- Hallucination filtering
- API key scoping
- Meeting reconciliation

Add tests for:
- Non-trivial business logic
- Edge cases (empty strings, boundary values)
- Error handling
- Data parsing

### Integration Tests

Tests marked `#[ignore]` require hardware or network:

```bash
# Microphone capture (requires working microphone)
cargo test --lib mic_probe -- --ignored --nocapture

# System audio (requires playback during test)
cargo test --lib system_probe -- --ignored --nocapture

# Full recording pipeline (mic + system, 5 seconds)
cargo test --lib rec_smoke -- --ignored --nocapture
```

### Manual Testing Checklist

Before submitting a PR with UI changes:

- [ ] Test on your primary OS
- [ ] Light and dark themes
- [ ] Window resizing (responsive layout)
- [ ] Error states (invalid API key, network failure)
- [ ] Empty states (no recordings, no meetings)
- [ ] Persistence (reload app, data persists)

## Documentation

### Code Documentation

- **Public APIs**: Use Rust doc comments (`///`).
- **Complex logic**: Explain with inline comments.
- **ADRs**: For significant architectural decisions, add to [docs/DECISIONS.md](docs/DECISIONS.md).

### User Documentation

Update when adding features:
- [README.md](README.md) (English)
- [README.pt-BR.md](README.pt-BR.md) (Portuguese)
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) (if architecture changes)
- [docs/PROVIDERS.md](docs/PROVIDERS.md) (if adding providers)
- [docs/ROADMAP.md](docs/ROADMAP.md) (mark features as done)

## Getting Help

- **Questions**: [GitHub Discussions](https://github.com/GeanChu/hicorder/discussions)
- **Bugs**: [GitHub Issues](https://github.com/GeanChu/hicorder/issues)
- **Real-time chat**: Coming soon (Discord/Slack TBD)

## Recognition

Contributors are recognized in:
- GitHub contributors graph
- Release notes (if your PR is in the release)
- [NOTICE](NOTICE) file (for substantial contributions)

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](LICENSE), the same license covering the project.

---

Thank you for contributing to Hicorder! 🎉
