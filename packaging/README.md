# Packaging

**P6.** Empty on purpose — a stubbed installer that produces nothing is worse than an absent
one, because a green release pipeline would imply packaging ran.

| Directory | What lands here | Rule |
|---|---|---|
| `macos/` | `.app` bundle with `LSUIElement = true`, signed and notarised. "Start at Login" via `SMAppService.mainApp.register()` — the app is its own login item, no helper agent. | R-DEL-8 |
| `windows/` | MSI or MSIX installer. Writes the native-messaging registry key (`HKCU\SOFTWARE\Google\Chrome\NativeMessagingHosts\<name>`) at install time; autostart via the Run key, toggled in-app. | R-DEL-9 |
| `mcpb/` | `manifest.json` for the MCPB bundle; release CI runs `mcpb pack` against the platform binary to produce Claude Desktop's one-click artifact. | R-DEL-10 |

Three decisions here are still open D0 rows: **MSI vs MSIX** (MSIX sandboxing may complicate
the registry write and the Run key — MSI is the safe default), the **`mcpb pack` CLI shape**,
and its **binary-server manifest fields**. See [`../docs/architecture/D0-report.md`](../docs/architecture/D0-report.md).

## Uninstall is a feature (R-DEL-11)

Uninstall MUST remove the app, the NM manifests (registry key on Windows, per-user manifest
file on macOS), the autostart registration, and `runtime.json`.

It MUST leave the **data root** — the database, screenshots, sidecars, and prompts — exactly
where it is. Deleting someone's library is their explicit act, never a side effect of
removing an application.

## The NM manifest stays data-driven (R-EXT-20)

Firefox and Safari are out of scope for v1, and no browser-detection branches ship. The one
accommodation: generate the native-messaging manifest from data rather than hardcoding it,
because Firefox keys on `allowed_extensions` (add-on id) where Chrome uses `allowed_origins`.
