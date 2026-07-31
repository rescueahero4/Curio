# Security Policy

## Reporting a vulnerability

**Please do not open a public issue for a security vulnerability.**

Report it privately through GitHub's [private vulnerability
reporting](https://docs.github.com/en/code-security/security-advisories/guiding-contributors-through-security-vulnerabilities/privately-reporting-a-security-vulnerability)
on this repository, or by email to **rescueahero@gmail.com**.

Private disclosure is mandatory here rather than merely polite. Curio is a loopback daemon
that holds a bearer token and an Anthropic API key on a user's own machine; a disclosure
mishap is a user-machine compromise, not a website defacement.

Please include: what you found, how to reproduce it, which version or commit, and your
platform. You will get an acknowledgement within 72 hours and an assessment within 7 days.
If the report is valid you will be credited in the release notes unless you'd rather not be.

## What Curio does with your data

**No telemetry.** Curio makes no network calls except the AI model calls you trigger with
your own API key, and it serves only on `127.0.0.1`. There is no analytics, crash reporting,
update check, or phone-home. Adding any would require a major-version bump and an owner
decision recorded in the [decision register](docs/architecture/00-architecture-overview.md).

Your Anthropic API key is stored in the OS keychain (DPAPI on Windows, Keychain on macOS),
falling back to an AES-256-GCM encrypted file with owner-only permissions. It never enters
the database, sidecars, logs, `config.json`, or `runtime.json`.

## Threat model

The full model is [ARCH-06 Security Architecture](docs/architecture/06-security-architecture.md).
In short, the actors that matter are **remote web content and rebound DNS** — because a web
page in your browser also runs on your machine, "local" is not the same as "trusted".

Explicitly **out of scope**: other processes running as the same OS user. They can read
`runtime.json` by design; that is the trust boundary, not a bug.

## Scope

In scope: anything that lets remote web content, a rebound DNS name, or an unpinned browser
extension reach the API, the MCP tools, the serve jails (`/files`, `/p/`), the runtime token,
the quit token, or the API key. Out of scope: findings that require an attacker who is
already running code as your OS user, physical access, or a compromised browser.

## For contributors

Any change touching `curio-server` middleware, the `/p` or `/files` serving paths,
`runtime.json`, `curio-nmh`, or an MCP tool must complete the security review checklist in
[ARCH-06 §Review checklist](docs/architecture/06-security-architecture.md) in the pull
request description (R-SEC-16). The pull request template carries it.
