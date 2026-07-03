# Security Policy

## Supported versions

Only the latest release of Hicorder receives security fixes.

## Reporting a vulnerability

Please report vulnerabilities privately via GitHub Security Advisories
("Report a vulnerability" on the repository's Security tab) or by email to
**financeiro@hi.capital**. Do not open public issues for security reports.

We aim to acknowledge reports within 7 days.

## Scope notes

- API keys are stored in the operating system keychain, never in plain text,
  logs, or the SQLite database.
- Audio, transcripts and summaries are stored locally. Network calls happen
  only to the AI/CRM providers explicitly configured by the user.
- Installers are built by public CI (GitHub Actions) from this repository.
