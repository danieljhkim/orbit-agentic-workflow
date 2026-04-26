# Security Policy

## Supported Versions

Orbit is pre-1.0 and ships from `main`. Security fixes land on `main` and the most recent tagged release; older tags do not receive backports.

| Version       | Supported          |
| ------------- | ------------------ |
| `main` (HEAD) | :white_check_mark: |
| Latest tag    | :white_check_mark: |
| Older tags    | :x:                |

## Reporting a Vulnerability

Please report security issues privately via GitHub: open the repository's **Security** tab and choose **Report a vulnerability** ([private vulnerability reporting](https://github.com/danieljhkim/orbit/security/advisories/new)).

Do **not** open a public issue, pull request, or discussion for suspected vulnerabilities.

Include enough detail to reproduce: affected version or commit, environment, steps, observed vs. expected behavior, and any proof-of-concept. A suggested fix is welcome but not required.

## What to Expect

This is a small project, so response is best-effort:

- **Acknowledgement:** within 7 days.
- **Triage and assessment:** within 30 days, including whether the report is accepted, declined, or needs more information.
- **Fix and disclosure:** coordinated with the reporter once a patch is available. Reporters are credited in the advisory unless they prefer to remain anonymous.

If a report is declined, you'll get a written explanation of why (out of scope, intended behavior, mitigated elsewhere, etc.).

## Scope

In scope:

- The `orbit` CLI, runtime, and crates published from this repository.
- Filesystem-scoping policy bypasses (`fsProfile`, `denyRead`, `denyModify`).
- Sandbox / process supervision escapes in `orbit-exec`.
- Audit log tampering or omission paths.
- Authentication, authorization, or origin-check bypasses on `orbit web serve` and `orbit mcp serve`.
- Credential handling and redaction for provider keys.

Out of scope:

- Vulnerabilities in upstream dependencies — please report those upstream. We'll bump the dependency once a fix is available.
- Issues that require an attacker who already has local code execution as the user running Orbit, or write access to the workspace, unless they cross a documented trust boundary.
- Social-engineering or phishing of project maintainers.
- Findings against forks or third-party redistributions of Orbit.
