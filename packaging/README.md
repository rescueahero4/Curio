# Packaging

Release CI builds these on every `v*.*.*` tag and attaches them to a **draft** GitHub
release ([`.github/workflows/release.yml`](../.github/workflows/release.yml)). The draft is
deliberate: ARCH-07's pipeline routes the footprint report through human review before a
release goes out (R-DEL-7), and a draft is what that arrow looks like in YAML.

| Directory | What it builds | Rule |
|---|---|---|
| `windows/` | NSIS installer. Per-user install under `%LOCALAPPDATA%\Programs\Curio`, no elevation. Registers the native-messaging host at install time; autostart via the Run key, toggled in-app. | R-DEL-9 |
| `macos/` | Universal `.app` bundle with `LSUIElement = true`, wrapped in a drag-to-install `.dmg`. "Start at Login" via `SMAppService.mainApp.register()` — the app is its own login item, no helper agent. | R-DEL-8 |
| `mcpb/` | **Still empty on purpose.** `manifest.json` for the MCPB bundle, packed by `mcpb pack` against the platform binary. Blocked on an open D0 row — the `mcpb pack` CLI shape and its binary-server manifest fields — and a stubbed bundle that produces nothing is worse than an absent one, because a green pipeline would imply it ran. | R-DEL-10 |

## Signing is conditional (D34)

Both jobs sign when credentials are present and produce an unsigned artifact when they are
not. **Release CI never fails for want of a certificate** (R-DEL-8a) — a pipeline that
cannot run until someone has paid a vendor has encoded a billing relationship as a build
dependency.

| Platform | Secrets | Without them |
|---|---|---|
| Windows | `WINDOWS_PFX_BASE64`, `WINDOWS_PFX_PASSWORD` | Ships unsigned. SmartScreen shows "Windows protected your PC". |
| macOS | `MACOS_SIGN_IDENTITY`, plus `AC_API_KEY_PATH` / `AC_API_KEY_ID` / `AC_API_ISSUER` to notarise | Ships **ad-hoc signed**. Gatekeeper blocks first launch; users take the Privacy & Security detour. |

The macOS ad-hoc signature is not cosmetic. Apple Silicon refuses to execute an arm64 binary
carrying no signature at all, and `lipo` strips whatever signature its inputs had — so
without that step the bundle dies with SIGKILL on every Mac shipped since 2020.

Windows has a genuinely free path: [SignPath Foundation](https://signpath.org/) issues
certificates to open-source projects. macOS does not — Apple gates both the Developer ID
certificate and the notarisation service behind the $99/year membership, with no open-source
exemption.

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

The installer inherits this for free by **invoking `curio-nmh --register`** rather than
writing registry keys itself. That binary is already on disk, already knows its own path,
and already owns the logic; restating its four registry keys in NSIS would give one fact a
second home (R-OV-2) and guarantee the two drift.

> **Open gap — macOS registration has no installer to run it.** Windows registers at install
> time. macOS drag-to-Applications has no install step, so nothing invokes `--register` and
> the extension falls back to the probe ladder (R-EXT-8) rather than working out of the box.
> Closing it means the app registering itself on first run. Tracked as P6 work; it is a code
> change in `curio-tray`, not a packaging one.

## Icons

The mark has **one source** — `assets/brand/curio-mark.svg` — and reaches three surfaces
from it (R-OV-2):

| Surface | Built from | When |
|---|---|---|
| `curio.exe` itself | `windows/curio.ico`, via `crates/curio-tray/build.rs` | every Windows build |
| The installer and uninstaller UI | the same `windows/curio.ico` | `makensis` |
| `Curio.app` | an `.iconset` assembled by `bundle.sh`, through `iconutil` | packaging |

Embedding it in the executable is the one that matters most. Windows reads the binary's icon
for Explorer, the taskbar, the Start Menu shortcut, and the Add/Remove Programs entry — a
handsome installer does nothing for any of them, because an installer is seen once and the
app is seen daily.

To regenerate after the mark changes:

```sh
node assets/brand/rasterize.mjs      # SVG -> PNGs at 16/32/48/128/256/512 (needs Playwright)
node packaging/windows/make-ico.mjs  # PNGs -> curio.ico (16/32/48/128/256)
```

Both outputs are committed, so a release never depends on regenerating an asset. `.ico` stops
at 256 because that is the largest size the format's one-byte dimension field can name;
macOS uses the 512 for Finder's large view.

> `rasterize.mjs` imports Playwright, which the repo installs under `target/pw`. ESM
> resolution walks up from the *script's* directory rather than the working directory, so
> `NODE_PATH` will not help — junction `assets/brand/node_modules` to
> `target/pw/node_modules` for the one run, or install Playwright at the repo root.

## Building locally

```sh
# Windows — needs NSIS on PATH.
# SRC_DIR and OUT_FILE must be ABSOLUTE and backslashed: NSIS resolves relative paths
# against the script's directory, so "stage" becomes packaging\windows\stage.
cargo build --release
mkdir stage && cp target/release/curio.exe target/release/curio-nmh.exe LICENSE stage/
makensis -DAPP_VERSION=0.1.0 \
  "-DSRC_DIR=$(pwd -W | tr / \\\\)\\stage" \
  "-DOUT_FILE=$(pwd -W | tr / \\\\)\\curio-setup.exe" \
  packaging/windows/curio.nsi

# macOS — both arch dirs may be the same one for a single-arch test build
cargo build --release
packaging/macos/bundle.sh 0.1.0 target/release target/release dist
```
