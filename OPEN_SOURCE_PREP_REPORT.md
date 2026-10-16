# Open Source Preparation Report - Hicorder

**Date:** October 16, 2026  
**Branch:** cursor/open-source-prep-8994  
**Audit Author:** Cloud Agent (Cursor)  
**Requester:** Gean Chu (GeanChu), Hi Capital

---

## Executive Summary

The Hicorder project has been audited and prepared for publication as an open source project. The audit covered security, privacy, licensing, documentation, and community files. The project is ready to be made public after reviewing the policy decisions documented below.

**Status:** ✅ Ready for review and publication (after policy decisions)

---

## What Hicorder Is Today

Hicorder is a cross-platform desktop meeting recorder (Windows, macOS, Linux) built with Tauri 2 (Rust + React). The project was born at Hi Capital (Honey Island VC, Curitiba, Brazil) as an alternative to Granola, offering complete flexibility to choose AI models, customizable prompts, and native integration with CRMs used in Venture Capital.

**Implemented features:**
- Recording of microphone and system audio (participants' voices) in separate tracks
- Live encoding to Opus/Ogg (~32 kbps) for lightweight storage
- Manual notes during recording
- AI transcription with multiple providers (Groq, OpenAI, Fireworks, custom endpoints)
- Chat format with "You" and "Participants" identified
- Terminology dictionary (40 VC terms pre-loaded, editable)
- Whisper hallucination filtering
- LLM summary with multiple providers (OpenAI, Claude, Gemini, MiniMax, NVIDIA NIM)
- Local Claude Code integration (no API key needed)
- Editable base prompt and library of named prompts
- CRM integration: Attio and Affinity
- ICS calendar with RRULE support (recurring events)
- Auto-start recording for scheduled meetings
- Reminders and configurable auto-stop
- Light/dark themes
- Auto-update
- Persistent logs with secret redaction

---

## Security and Privacy Audit

### ✅ Secrets and Credentials

**Status:** No real secrets found in code or Git history.

**Analysis:**
- ❌ No real API keys found (sk-, nvapi-, gsk_, etc.)
- ❌ No tokens or passwords found
- ✅ Key examples in comments (gean@hi.capital, examples) are legitimate placeholders
- ✅ Secret redaction system already implemented in `logs/mod.rs`
- ✅ Keys stored in OS keychain or 0600 protected file on macOS

**Locations verified:**
- Source code (Rust and TypeScript)
- Configuration files (.github, tauri.conf.json)
- Git history (147 commits examined)
- Environment variables (.env does not exist, .env.example created)

### ⚠️ Personal and Customer Data

**Status:** Test data found, no real customer data identified.

**Data found (legitimate for the project):**
- `gean@hi.capital` - Author's email (used in unit tests and contact documentation)
- `financeiro@hi.capital` - Security contact email (legitimate)
- References to "hi.capital" as author/copyright (legitimate)
- Test examples: `voce@hi.capital`, `outro@hi.capital` (test data, not real)

**Data NOT found:**
- ❌ Real customer names or emails
- ❌ Real meeting recordings
- ❌ Real meeting transcripts or summaries
- ❌ Private ICS calendar URLs
- ❌ CRM data (meetings, contacts, companies)

**Recommendation:** ✅ Safe for publication. The data found is appropriate for an open source project.

### ✅ Git History

**Status:** Clean history, no rewrite needed.

**Analysis:**
- 147 commits analyzed
- Normal development commits, no evidence of committed secrets
- Appropriate commit messages
- No .env files committed in history
- Sensitive files have always been in .gitignore

**Recommendation:** ✅ No need to rewrite history or create a clean repository.

### ✅ Configuration Files

**Status:** No credentials or sensitive data.

**Files analyzed:**
- `.gitignore` - Appropriate (ignores node_modules, target, .env, etc.)
- `tauri.conf.json` - No secrets
- `package.json` / `Cargo.toml` - No secrets
- `.github/workflows/` - References GitHub secrets (TAURI_SIGNING_PRIVATE_KEY, SIGNPATH_API_TOKEN) which are properly managed by GitHub Secrets

**New file created:**
- `.env.example` - Documented template with all required variables

---

## Dependency and Licensing Audit

### ✅ Project License

**Status:** MIT (Apache 2.0 recommended as alternative in decision report).

**Current analysis:**
- LICENSE already exists and is correct (MIT, copyright hi.capital 2026)
- NOTICE exists and documents code reuse from meetily (MIT)
- MIT license is permissive and compatible with community adoption goal

**License recommendations (for Gean's decision):**

1. **MIT (current)** ✅ Recommended for maximum adoption
   - Most permissive
   - Allows commercial use without restrictions
   - Favors rapid community adoption
   - Allows companies to create proprietary closed versions
   - **Best for:** Rapid growth, broad ecosystem

2. **Apache 2.0** ⚠️ Balanced alternative
   - Permissive like MIT
   - Explicit patent protection
   - Requires preserving notices and changelog
   - Allows commercial use but with more legal clarity
   - **Best for:** Projects with potential patent disputes

3. **AGPL-3.0** ❌ Not recommended
   - Strong copyleft: SaaS users must open their code
   - Protects against closed commercial use
   - **However:** Drastically reduces community adoption
   - Companies avoid AGPL due to "contamination" fears
   - **Avoid** if goal is to maximize contributions and usage

**Suggested decision:** Keep MIT. The project's mission is to democratize AI meeting recording, and this is best served by a permissive license that maximizes adoption. Since Hi Capital has no commercial plans for Hicorder, there is no concern about competitive SaaS—in fact, commercial forks are welcome as they extend the project's reach.

### ✅ Dependency Compatibility

**Status:** All dependencies are compatible with MIT.

**Main frontend dependencies (npm):**
- @tauri-apps/* - MIT/Apache-2.0
- react, react-dom - MIT
- vite - MIT
- typescript - Apache-2.0

**Main backend dependencies (Rust):**
- tauri - MIT/Apache-2.0
- serde, serde_json - MIT/Apache-2.0
- anyhow - MIT/Apache-2.0
- cpal - Apache-2.0
- rusqlite - MIT
- reqwest - MIT/Apache-2.0
- chrono - MIT/Apache-2.0

**Code reuse:**
- meetily (MIT) - Audio capture pipeline adapted, already documented in NOTICE

**Conclusion:** ✅ All licenses are permissive and compatible with MIT or Apache-2.0.

---

## Documentation Created/Updated

### ✅ Main Files

1. **README.md** (updated, English)
   - Complete project description
   - Honest comparison with Granola
   - Platform installation guide
   - Quick start
   - High-level architecture
   - Links to detailed documentation

2. **CONTRIBUTING.md** (new)
   - Complete development setup
   - Project structure
   - Code standards (Rust and TypeScript)
   - PR submission process
   - Guides for adding new AI providers
   - Guides for adding new CRM integrations
   - Testing guidelines

3. **.env.example** (new)
   - Template with all environment variables
   - Inline documentation of where to get API keys
   - Security warnings (ICS URL is credential)

4. **CODE_OF_CONDUCT.md** (new)
   - Short, objective code of conduct in English
   - Based on Contributor Covenant 2.1
   - Contact: gean@hi.capital

5. **GOVERNANCE.md** (new)
   - Project governance model
   - Gean Chu as lead maintainer
   - Decision process
   - How to become a maintainer
   - Conflict resolution

6. **SECURITY.md** (already existed, reviewed)
   - Vulnerability reporting policy
   - Security scope
   - Contact: financeiro@hi.capital

### ✅ GitHub Templates

1. **.github/ISSUE_TEMPLATE/bug_report.md**
   - Structured template for bug reports
   - Fields for system info, logs, screenshots

2. **.github/ISSUE_TEMPLATE/feature_request.md**
   - Template for feature requests
   - Fields for problem, proposed solution, use cases

3. **.github/ISSUE_TEMPLATE/crm_integration.md**
   - Specific template for CRM integration requests
   - API capabilities checklist
   - Fields for documentation and test availability

4. **.github/PULL_REQUEST_TEMPLATE.md**
   - Complete checklist for PRs
   - Fields for change type, testing, screenshots
   - Quality checks

5. **.github/CODEOWNERS**
   - Gean Chu as default owner
   - Specific ownership for docs, CI, security files

6. **.github/dependabot.yml**
   - Configured for npm, cargo, and github-actions
   - Weekly updates
   - Automatic labels

### ✅ Existing Documentation (reviewed)

Files in `docs/` already existed and are well documented:
- ARCHITECTURE.md - Detailed system design
- ROADMAP.md - PR-by-PR development history
- DECISIONS.md - ADRs with technical justifications
- PROVIDERS.md - Supported AI providers
- SIGNING.md - Code signing process
- CONTINUATION.md - Current state for session handoff

### ⚠️ Repository Metadata (to configure on GitHub)

**Suggested short description (1 line):**
```
Open-source meeting recorder with AI transcription and CRM integration for Venture Capital
```

**Suggested topics:**
```
meeting-recorder, transcription, ai, crm, venture-capital, tauri, rust, react, 
typescript, desktop-app, speech-to-text, llm, attio, affinity, whisper, groq
```

**Social Preview:**
- Use project icon (src-tauri/icons/icon.png) with text "Hicorder - Open Source Meeting Recorder"
- Professional background (blue/purple from app theme)

**About section on GitHub:**
- Website: https://hi.capital
- Topics: (list above)
- Check: ✅ Releases
- Check: ✅ Packages (if publishing in future)

---

## CI/CD

### ✅ Existing GitHub Actions

The project already has complete CI:

1. **.github/workflows/ci.yml**
   - Build and test on 3 platforms (ubuntu-22.04, macos-15, windows-latest)
   - Rust tests included
   - Xcode 26 pinned on macOS for dependencies

2. **.github/workflows/release.yml**
   - Automatic installer builds on v* tags
   - ffmpeg download with multiple mirrors and retry
   - Update signing (TAURI_SIGNING_PRIVATE_KEY)
   - SignPath Foundation integration (prepared, awaiting approval)
   - Creates draft release on GitHub

**Status:** ✅ Functional CI, no changes needed for open source.

**Required secrets (already configured):**
- `TAURI_SIGNING_PRIVATE_KEY` - For update signing
- `SIGNPATH_API_TOKEN` - For code signing (when approved)

---

## Policy Decisions for Gean

The project is technically ready, but some business decisions need to be made before public launch:

### 1. License ✅

**Current situation:** MIT  
**Recommendation:** Keep MIT  
**Alternatives:** Apache 2.0 (if patent concerns), AGPL-3.0 (not recommended)  
**Decision needed:** Confirm MIT or change to Apache 2.0  
**Action:** If changing, update LICENSE and file headers

### 2. Brand and "Hicorder" Name ⚠️

**Decision needed:** 
- Will "Hicorder" be a registered trademark?
- Can third parties create forks and continue using the name?
- Should there be a trademark policy (e.g., "Hicorder® is a trademark of Hi Capital, forks must use different name")?

**Recommendation:** Allow name usage freely. Since Hi Capital has no commercial plans for Hicorder, there's no need to restrict third-party commercial use of the name.

**Action:** No trademark restrictions needed.

### 3. CLA vs DCO ⚠️

**Decision needed:** Require Contributor License Agreement (CLA) or Developer Certificate of Origin (DCO)?

**Options:**
- **None (current):** Easier for contributors, MIT is already permissive
- **DCO:** Contributors attest they have right to contribute (git commit -s)
- **CLA:** Contributors transfer/license rights to Hi Capital (more formal, more friction)

**Recommendation:** No CLA/DCO initially. MIT already protects sufficiently. If it grows significantly, consider DCO.

**Action:** If opting for DCO, configure bot on GitHub and update CONTRIBUTING.md.

### 4. Proprietary CRM Contributions ⚠️

**Decision needed:** Accept integrations with CRMs that don't have public API or free trial?

**Scenario:** Someone wants to contribute integration with Enterprise CRM that only works with paid account.

**Options:**
- Accept any integration (more features)
- Accept only CRMs with publicly testable API (better quality)
- Accept with "not tested by core team" disclaimer

**Recommendation:** Accept any integration, mark untested integrations in README.

**Action:** Document policy in CONTRIBUTING.md.

### 5. Telemetry/Analytics ✅

**Current situation:** No telemetry  
**Decision needed:** Add opt-in telemetry in the future?

**Recommendation:** Keep without telemetry. Privacy-first is a project differentiator.

**Action:** If changing in future, full disclosure and explicit opt-in, never opt-out.

### 6. Hosted or Paid Version 💰

**Decision:** Honey Island (Hi Capital) does **NOT** plan to monetize Hicorder.

**Policy:**
- No SaaS or hosted version will be offered by Hi Capital
- No "open core" model (all features remain open source)
- No paid/enterprise version or commercial support
- The project is maintained by the community without commercial intent
- MIT license allows third parties to offer commercial services, but Hi Capital will not

**Action:** Document this policy in README and GOVERNANCE for transparency.

### 7. Data and Privacy Policy 📋

**Current situation:** "Privacy" section in README documents that data is local  
**Decision needed:** Is this sufficient or need formal Privacy Policy?

**Recommendation:** Current is sufficient. Hicorder is a desktop application with local data storage only—no SaaS, no telemetry, no data collection by Hi Capital.

**Action:** None needed.

### 8. Security and Disclosure Policy 📋

**Current situation:** SECURITY.md with reporting process (GitHub Security Advisories or financeiro@hi.capital)  
**Decision needed:** Define SLA for response?

**Recommendation:** 
- Acknowledge receipt in 7 days (already documented)
- Fix in 30 days for critical vulnerabilities
- Fix in 90 days for non-critical
- Public disclosure after fix + 7 days grace period

**Action:** Document SLA in SECURITY.md if agreeing.

### 9. PR Review and SLA ⏱️

**Decision needed:** How long to review PRs? Who can merge?

**Current situation:** Gean is the only maintainer  
**Recommendation:**
- First response on PRs: 14 days (confirm you saw it)
- Full review: 14 days
- No guaranteed SLA (community project), but reasonable effort

**Action:** Document expectations in CONTRIBUTING.md.

### 10. CRM Credentials and Recording Consent ⚠️

**Decision needed:** Warn users about:
- Recording may require participant consent (laws vary by jurisdiction)
- CRM keys give broad access to CRM data

**Recommendation:** 
- Add disclaimer in README about recording laws
- Add warning on first recording (modal in app)
- Document minimum permission scopes for each CRM

**Action:** 
- Add "Legal Considerations" section to README
- Issue to implement warning modal in app

---

## Launch Checklist

Use this list when preparing for public launch:

### Before Making Repo Public

- [x] Complete security audit
- [x] README in English
- [x] CONTRIBUTING.md
- [x] CODE_OF_CONDUCT.md
- [x] GOVERNANCE.md
- [x] Issue and PR templates
- [x] CODEOWNERS
- [x] Dependabot configured
- [x] .env.example created
- [ ] Review and decide policy questions (sections above)
- [ ] Update repository metadata (description, topics, social preview)
- [ ] Confirm SECURITY.md has correct contact
- [ ] Create protected `main` branch (require PR reviews, require status checks)
- [ ] Test build in GitHub Actions (test tag)

### On Launch Day

- [ ] Make repository public on GitHub (Settings → Danger Zone → Change visibility)
- [ ] Create v0.3.0 release (or next version) with release notes
- [ ] Publish installers in release
- [ ] Post on LinkedIn/Twitter announcing (Hi Capital and Gean personal)
- [ ] Create GitHub Discussion "Welcome"
- [ ] Configure GitHub Projects for public roadmap (optional)
- [ ] Add "open source" badge on Hi Capital website (optional)

### First Week

- [ ] Respond to issues and PRs promptly (first impression matters)
- [ ] Create labels on GitHub (bug, enhancement, good first issue, help wanted)
- [ ] Mark simple issues as "good first issue" to attract new contributors
- [ ] Configure GitHub Sponsors if wanting donations (optional)
- [ ] Submit to aggregators (awesome lists, alternativeto.net, etc)

### First Month

- [ ] Review and merge first community PRs
- [ ] Create CHANGELOG.md with version history
- [ ] Consider creating Discord/Slack for community (if demand exists)
- [ ] Request code signing from SignPath Foundation (if not yet approved)
- [ ] Evaluate feedback and adjust roadmap

---

## Identified Risks and Mitigations

### Risk: Low Initial Adoption

**Probability:** Medium  
**Impact:** Medium  
**Mitigation:**
- Well-executed launch marketing
- Excellent documentation (✅ done)
- Quick response to first issues/PRs
- Identify "early adopters" (VC community, dissatisfied Granola users)

### Risk: Competitive Commercial Forks

**Probability:** Low (short term), Medium (long term)  
**Impact:** Low (Hi Capital has no commercial plans)  
**Mitigation:**
- MIT allows this (it's the nature of the license)
- Since Hi Capital does not plan to monetize Hicorder, commercial forks are welcome
- Community benefits from any improvements contributed back
- The project remains purely community-driven

### Risk: Security Vulnerabilities

**Probability:** Medium (all software has bugs)  
**Impact:** High (handles private recordings)  
**Mitigation:**
- SECURITY.md with clear process (✅ done)
- Dependabot configured (✅ done)
- Respond to vulnerabilities in ≤30 days
- Bug bounty in future (if project grows)

### Risk: Low-Quality Contributions

**Probability:** Medium  
**Impact:** Low  
**Mitigation:**
- Automatic CI catches build errors (✅ exists)
- CONTRIBUTING.md with clear standards (✅ done)
- Mandatory code review (configure branch protection)
- Maintainer has right to reject PRs

### Risk: Maintainer Overload (Gean)

**Probability:** High (if project grows)  
**Impact:** High (burnout, project stagnates)  
**Mitigation:**
- Invite co-maintainers when project grows
- Document everything to ease onboarding (✅ well documented)
- Automate what's possible (CI, dependabot - ✅ done)
- Set realistic SLA expectations (not full-time work)

---

## Code Quality Analysis

### ✅ Strengths

1. **Well-thought-out architecture**
   - Clear separation of responsibilities (audio/, transcription/, summary/, CRM)
   - Pluggable provider abstraction
   - Local storage with SQLite

2. **Implemented security**
   - Keys in OS keychain
   - Secret redaction in logs
   - No telemetry

3. **Excellent technical documentation**
   - ADRs document decisions with context
   - ARCHITECTURE.md details entire system
   - Inline comments explain "whys"

4. **Existing tests**
   - Unit tests for critical logic
   - Integration tests for audio (#[ignore])
   - CI runs tests on 3 platforms

5. **Robust CI/CD**
   - Automatic build on 3 OSes
   - ffmpeg download with retry and fallback
   - Functional auto-update

### ⚠️ Areas for Improvement (for future roadmap)

1. **Test coverage**
   - Missing tests for CRM modules
   - Missing tests for ICS parser
   - Consider coverage tracking

2. **Internationalization**
   - UI hardcoded in Portuguese
   - Easy to add i18n (React + JSON)
   - Community can contribute translations

3. **Error telemetry (opt-in)**
   - Consider Sentry or similar (opt-in)
   - Would help discover production bugs
   - But: conflicts with privacy positioning

4. **End-user docs**
   - README is technical (for developers)
   - Consider wiki or docs/ for user guides
   - Video tutorials

---

## Final Recommendation

✅ **The Hicorder project is ready to be published as open source.**

**Next steps:**

1. **Gean reviews this report** and confirms remaining policy questions (CLA, etc.)
2. **Merge this PR** (cursor/open-source-prep-8994) into main branch
3. **Configure repository metadata** on GitHub (description, topics, social preview)
4. **Make repository public**
5. **Announce** the launch

The code is clean, documented, and secure. The open source community will respond well to an honest, well-documented project with clear leadership.

---

**Files created/modified in this PR:**
- README.md (updated)
- CONTRIBUTING.md (new)
- CODE_OF_CONDUCT.md (new)
- GOVERNANCE.md (new)
- .env.example (new)
- .github/ISSUE_TEMPLATE/bug_report.md (new)
- .github/ISSUE_TEMPLATE/feature_request.md (new)
- .github/ISSUE_TEMPLATE/crm_integration.md (new)
- .github/PULL_REQUEST_TEMPLATE.md (new)
- .github/CODEOWNERS (new)
- .github/dependabot.yml (new)
- OPEN_SOURCE_PREP_REPORT.md (this file)

**Total commits:** 14 (up to this point)
