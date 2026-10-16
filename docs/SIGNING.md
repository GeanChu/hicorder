# Code Signing — SignPath Foundation

Plan to sign Windows executables **for free** via [SignPath Foundation](https://signpath.org/), which offers signing certificates for qualifying open source projects.

## Why

Unsigned installers trigger SmartScreen and antivirus software (Kaspersky, etc.). Digital signature with timestamping solves most of this on Windows. macOS (notarization) requires paid Apple Developer account — out of scope for now.

## Prerequisites (status)

- [x] OSI license (MIT — [LICENSE](../LICENSE))
- [x] Public source code: https://github.com/GeanChu/hicorder
- [x] 100% CI public build (GitHub Actions — [release.yml](../.github/workflows/release.yml)); signed artifacts must come from CI, never from local builds
- [x] Product metadata in binary (publisher, version, description — `tauri.conf.json`)
- [x] README with project description and security policy ([SECURITY.md](../SECURITY.md))
- [x] Repository renamed to `hicorder` (consistency with product)

## How to Apply (done by repository owner)

1. Create account at https://app.signpath.io (GitHub login)
2. Submit OSS program form: https://signpath.org/apply
   - Project name: **Hicorder**
   - Repository: Public repo URL
   - License: MIT
   - Short description of what the app does (meeting recorder with AI transcription) and audience (teams)
   - Committers: GitHub account(s) with write access
3. Wait for evaluation (days to a few weeks). Foundation verifies project activity and maintainer identity.

## After Approval

SignPath creates an organization with a project and a *signing policy* (`release-signing`). Integration in `release.yml` (Windows job, after build):

```yaml
      - name: Sign Windows installers (SignPath)
        if: matrix.platform == 'windows-latest'
        run: |
          # Submit artifacts to SignPath
          # API call with SIGNPATH_API_TOKEN
          # Wait for signing to complete
          # Download signed artifacts back
```

Add secrets in repository settings:
- `SIGNPATH_API_TOKEN` — API key from SignPath dashboard (Settings → API tokens)
- `SIGNPATH_ORG_ID` — Organization ID from SignPath (visible in dashboard URL)

The job submits the installer to SignPath, waits for signing (usually < 5 minutes), and downloads the signed artifact back, replacing the unsigned one in the release.

## References

- SignPath Foundation: https://signpath.org/
- Application form: https://signpath.org/apply
- Documentation: https://about.signpath.io/documentation/
- GitHub Action example: https://github.com/signpath/github-actions

## Attribution

Free code signing provided by [SignPath.io](https://signpath.io), certificate by [SignPath Foundation](https://signpath.org/).

This attribution should be added to:
- README.md (already added in installers section)
- Release notes (automatically via CI)
