---
title: Privacy
description: No telemetry, no analytics, no phone-home — and what that commitment costs to change.
---

**No telemetry.** Curio makes no network calls except the AI model calls you trigger with
your own API key. There is no analytics, no crash reporting, no update ping, and no
phone-home of any kind.

This is not a default that a future release quietly flips. Adding any phone-home requires a
major-version bump and an owner decision recorded in the decision register in
[ARCH-00](../../architecture/00-architecture-overview/) — the rule is
[R-DEL-16](../../architecture/07-delivery-open-source/), and it is enforced at code review
against that rule ID.

## Where your data sits

Everything is a file on your disk:

| What | Windows | macOS |
|---|---|---|
| Your library (`library.db`, `items/`, `prompts/`, `skills/`) | `%USERPROFILE%\Curio` | `~/Curio` |
| `runtime.json` — port and per-run token | `%LOCALAPPDATA%\Curio` | `~/Library/Application Support/Curio` |

`runtime.json` is deleted when Curio quits. Its absence is how everything else — the
extension, the MCP proxy — knows the app is not running.

## Your API key

The Anthropic API key lives in the OS keychain: DPAPI on Windows, Keychain on macOS. It is
never written to the database, the config file, the logs, or the repository.

If you never add a key, captures still land and stay browsable. They sit at "Queued — needs
an API key" and the queue drains by itself the moment you add one.

## Uninstall

Clean uninstall is a feature, not an afterthought
([R-DEL-11](../../architecture/07-delivery-open-source/)). Uninstalling removes the app,
the native-messaging manifests, the autostart registration and `runtime.json` — and leaves
your library exactly where it is. Deleting a library is your explicit act, never a side
effect of removing the software.

## Reporting a vulnerability

Curio is a loopback daemon holding a bearer token and an API key, so a disclosure mishap is
a user-machine compromise rather than a website defacement. Use the private route in
[SECURITY.md](https://github.com/rescueahero4/Curio/blob/master/SECURITY.md) — not a public
issue.
