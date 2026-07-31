---
id: ARCH-D0
title: "D0 Verification Spike — results"
status: in-progress
version: 0.1.0
date: 2026-07-31
project: curio
depends_on: [ARCH-07]
governs: [verification]
---

# D0 Verification Spike — results

> **Why this document exists.** Budgets and version pins are **claims until measured**. The
> architecture rests on a dozen of them — that a tray crate behaves on both platforms, that
> an empty shell fits in 12 MB, that a browser still sends a header we plan to check. D0 is
> release-0 (R-DEL-20): **no Phase-1 work merges to main until every row below has a
> recorded result** — a pass, or a fallback consciously chosen.
>
> A row is not "done" because it looks obviously true. It is done when someone ran it and
> wrote down what happened, with the method. If a recorded result later stops holding, open
> a **D0 claim re-verification** issue rather than quietly updating the number.

## Status

| | Count |
|---|---|
| Verified | 5 |
| Partial (a real result, with a named gap) | 3 |
| Outstanding | 6 |
| Post-v1 sub-group (verified at the vector-activation release) | 2 |

Last updated 2026-08-01, at the end of the E1–E6 build. That pass closed rows 3 and 10
outright, re-measured row 2 against the finished server rather than an empty shell, and
turned row 4 from "not started" into a partial with the remaining gap named. Rows 5, 6, 8,
11, 12 and 13 are untouched; rows 8, 11, 12 and 13 all block phases (P5, P6) that this
pass did not reach.

Platform coverage so far: **Windows 11 only.** Every row below is a macOS gap until someone
runs it on macOS, and the rows most likely to differ — the tray, the single-instance guard,
file permissions, keychain — are exactly the ones with no shared implementation.

## Release-0 rows

| # | Item | Owner / OQ | Status | Result |
|---|---|---|---|---|
| 1 | **Tray crates** (`tray-icon` + tao): icon, menu, glyph swap, main-thread rules | [ARCH-01](01-backend-architecture.md) OQ-4 | ⚠️ Partial | `tray-icon` 0.24.2 + `tao` 0.36.0 build and run on Windows 11; the five-item menu (D14) renders, the icon is generated in code, and the event loop sits in `ControlFlow::Wait`. **macOS untested** — and macOS is where the main-thread rule actually bites. |
| 2 | **Empty-shell RSS ≤ 12 MB** | [ARCH-01](01-backend-architecture.md) OQ-4, R-BE-31 | ✅ Pass (Windows) | **2.0 MB private commit** at scaffold (tray + axum listener + SQLite open, empty library). **Re-measured 2026-08-01 with the full P1–P3 server** — every route, both push transports, the projects watcher, one item in the library: **2.2 MB**. The entire route surface, SSE, WebSocket and watcher cost ~0.2 MB against a 12 MB gate. **See the measurement note below — the method matters more than the number.** |
| 3 | **TipTap core sans React**: chip node views and the slash menu under Solid's lifecycle | [ARCH-03](03-frontend-architecture.md) OQ-2 | ✅ Pass — **the ProseMirror fallback is not needed** | Verified 2026-08-01 by building the editor and shipping it. `@tiptap/core` + `@tiptap/pm` + `@tiptap/starter-kit`, all pinned **3.29.2**; `@tiptap/react` appears in neither `package.json` nor the lockfile, and no React arrives transitively. Eleven checks pass, three of which mount `EditorSurface` through `solid-js/web`'s `render()` and then `dispose()` — so `onMount → new Editor(el)` and `onCleanup → editor.destroy()` are exercised through Solid's own lifecycle rather than called directly. Chip node views render plain DOM, `itemRef` falls back to its stored label, the `section` attribute survives a round trip, ghost decoration lands on the empty paragraph, and the slash trigger fires after a space but **not** inside `http://`. See the finding below for what the spike actually cost. |
| 4 | **Keychain crates** (DPAPI + Security.framework; scrypt fallback parameters matching the previous implementation) | [ARCH-06](06-security-architecture.md) OQ-3 | ⚠️ Partial — **DPAPI done, the scrypt half is still open** | `curio-server/src/secrets.rs` implements the keychain-first store: `ANTHROPIC_API_KEY` → DPAPI (`CryptProtectData`, via `windows-sys` directly — no wrapper crate) on Windows, the `security` CLI with the previous implementation's `curio-anthropic-api-key` service name on macOS. **The AES-256-GCM encrypted-file fallback is deliberately absent.** Its scrypt parameters are exactly what this row has not verified, and guessing them produces a file the old app wrote and this one silently cannot read. Until it is verified, a platform with no keychain backend **refuses to store a key** rather than writing one in plaintext — see the finding below. |
| 5 | **EcoQoS** via `SetThreadInformation` on the service thread (the Windows 11 ControlMask/StateMask gotcha) | [ARCH-01](01-backend-architecture.md) OQ-5 | ⬜ Not started | Blocks P1. Fallback: ship at default QoS. |
| 6 | **`Sec-Fetch-Site`** actually sent on loopback fetches from extension and SPA contexts | [ARCH-06](06-security-architecture.md) OQ-1 | ⬜ Not started | Blocks P1. If it is not sent, R-SEC-12 stays advisory and must not reject. |
| 7 | **rmcp pin** | [ARCH-05](05-mcp-architecture.md) OQ-1 | ⚠️ **Finding — needs an owner decision** | See below. |
| 8 | **NMH cold-start on Windows** (Defender's first-run scan) keeps the popup dot sub-second | [ARCH-04](04-extension-architecture.md) OQ-4 | ⬜ Not started | Blocks P5. The binary is built and is 180 KB, which is the right order of magnitude; the latency itself is unmeasured. |
| 9 | **Unpacked-extension `key`/id**: same id unpacked as packed | [ARCH-04](04-extension-architecture.md) OQ-2 | ⚠️ Partial | The id **derivation** is verified: `curio-server`'s `the_pinned_origin_is_the_one_chrome_derives_from_the_manifest_key` re-derives `oehjmjhhelijpkojhpichkfcgbdejhfa` from the manifest key and asserts it against the server's allowlist, so the two files cannot drift. Whether Chrome assigns that id to an **unpacked** load is untested. |
| 10 | **`opt-level "z"` vs `"s"`**: size and hot-path speed | [ARCH-07](07-delivery-open-source.md) OQ-1 | ✅ Pass (Windows) — **pinned to `"z"`** | Measured 2026-08-01, x86_64-pc-windows-msvc, `curio.exe` with no SPA assets embedded: **`"z"` 3,207,168 bytes vs `"s"` 3,462,656** — `"z"` is 7.4 % (250 KB) smaller. Hot-path speed did not turn out to be the tiebreak OQ-1 expected: the process is idle almost all of the time, and the one genuinely hot path — SQLite — is bundled C that `opt-level` does not touch. `Cargo.toml` now pins `"z"` with the measurement recorded beside it. ARCH-07 OQ-1 is closed. |
| 11 | **MSI vs MSIX**: the NM registry write and Run-key autostart under MSIX sandboxing | [ARCH-07](07-delivery-open-source.md) OQ-2 | ⬜ Not started | Blocks P6. MSI is the safe default. |
| 12 | **MCPB tooling**: `mcpb pack` CLI shape and binary-server manifest fields | [ARCH-07](07-delivery-open-source.md) OQ-3, [ARCH-05](05-mcp-architecture.md) OQ-3 | ⬜ Not started | Blocks P6. |
| 13 | **Chrome ~147 LNA gating** of loopback WebSockets | [ARCH-04](04-extension-architecture.md) OQ-1, [ARCH-06](06-security-architecture.md) OQ-5 | ⬜ Not started | Blocks P5. Secondary-sourced; verify against current Chrome. |

### New row, added at scaffold

| # | Item | Status | Result |
|---|---|---|---|
| 14 | **GUI-subsystem stdio purity.** R-DEL-9 requires `#![windows_subsystem = "windows"]` so the tray app has no console. R-MCP-5 and R-EXT-5 require clean stdout for `curio --mcp-stdio` and `curio-nmh`. A GUI-subsystem process has no console to fall back on, so if inherited pipes did not work, both would fail silently. | ✅ Pass (Windows) | The release build (`windows_subsystem = "windows"`) writes tracing output to an **inherited stderr pipe** normally. Confirmed by capturing a background launch's stderr. stdout purity for the stdio proxy itself is still untested — that is P4. |

## Findings

### Row 7 — rmcp has moved to a major version the contract does not cover

**R-MCP-14 says: "the rmcp version floor is 1.4.0; the workspace pins major version 2."**
As of 2026-07-31, crates.io publishes **rmcp 3.0.1**. There is no 2.x line to pin that is
also current.

This is an owner decision, not a Cargo.toml edit, so **`rmcp` is deliberately absent from
the workspace** rather than pinned to a major version nobody chose. `curio-mcp` compiles
without it — it currently holds the tool inventory and the refusal shapes, no transport.

Three things need deciding together before P4:

1. Which major version. 3.x is current; 2.x may be maintained but is not the head.
2. Whether `StreamableHttpService` still exposes the stateless / JSON-response configuration
   the architecture assumes (R-MCP-4). This is the substance of the original OQ-1 and does
   not go away by choosing a number.
3. Whether the CVE-2026-42559 fix and the residual Origin gap (rust-sdk #822) are still
   characterised as R-SEC-7 describes them. Curio's own Origin middleware sits in front
   either way, so this changes the belt, not the braces.

Whatever is chosen, **R-MCP-14 must be amended in the same PR** (R-DEL-18). The `deny.toml`
ban on `rmcp < 1.4.0` is already in place and holds regardless of the major version.

### Row 3 — TipTap under Solid cost three small workarounds, none architectural

D16 and R-FE-16 bet that TipTap's React bindings are a separate optional package and that
the core mounts anywhere. That held. The friction is worth recording because all of it is
the kind a reader would otherwise rediscover:

1. **`editor.isActive()` is not reactive.** It reads current ProseMirror state, so the
   toolbar subscribes to `editor.on("transaction")` and bumps a signal. Three lines — the
   React bindings do the same thing internally.
2. **The editor must not be reactive to its `doc` prop.** Pushing the autosaved document
   back into the view would move the caret mid-typing, so `EditorSurface` reads `props.doc`
   once in `onMount` and says so in a comment.
3. **Ghost text is a decoration, not a class on content.** Placeholder text must never be
   part of the document, or the serializer would emit Curio's words inside the user's brief.

One dependency was **rejected** on R-FE-3 grounds: `@tiptap/suggestion` pulls
`@floating-ui/dom`, which matches neither `@tiptap` nor `prosemirror` in the build's
`manualChunks` predicate and would therefore have leaked out of the editor chunk into every
route. A ~60-line ProseMirror plugin replaces it and keeps the boundary exact.

**Editor chunk: 371.91 kB raw / 118.88 kB gzip**, split and lazy, against a 140.94 kB app
chunk that contains no ProseMirror at all (asserted by grep, not by inspection). That is
ARCH-03 OQ-3 — "editor-chunk size after the Solid port" — answered at the same time.

### Row 4 — a refusal was chosen over a guessed encrypted-file format

R-SEC-10 specifies three secret backends in order: DPAPI, macOS Keychain, and an
AES-256-GCM encrypted file whose scrypt key is derived from `curio:<hostname>:<username>`.
The first two are implemented. The third is not, and that is deliberate rather than
unfinished.

The encrypted-file fallback exists so an in-place upgrade can read an existing
`.secrets.json`. Its value therefore depends entirely on the scrypt parameters matching the
previous implementation's byte for byte — which is precisely the thing this row has not
verified. Implementing it against guessed parameters produces the worst outcome available:
a file the old app wrote and the new one cannot read, discovered by a user whose API key
has silently vanished.

So on a platform with no keychain backend, `store_api_key` **fails loudly** with a message
pointing at `ANTHROPIC_API_KEY`, which works everywhere. This is stricter than the old app,
not looser — no key is ever written to disk in the clear. When the scrypt parameters are
verified against a real `.secrets.json`, the fallback lands and this row closes.

Windows and macOS are unaffected: both have a real backend today.

### Row 2 — the measurement method changes the answer by three orders of magnitude

The first measurement of the release binary reported **20 KiB**, which is not a triumph — it
is Windows having trimmed the working set of an idle process. Working Set is trimmable, so
it says how much the OS currently feels like keeping resident, not what the app costs.

`cargo xtask footprint` now reports **private commit** (`PrivateMemorySize64`): memory the
process owns, shares with nobody, and that cannot be trimmed away behind the measurement.
That is **2.0 MB**.

Two consequences:

- **The number recorded above is private commit, not private working set.** ARCH-01's budget
  table says "RSS idle (private)". These are close relatives but not the same counter, and
  the difference is larger than the headroom on some future regression.
- **Windows and macOS numbers are not comparable.** macOS's `footprint` reports something
  different again. Both are legitimate; a comparison between them is not.

### Row 1 — the single-instance guard is per-library, not per-machine

Not a listed row, but discovered while testing it and worth recording. The Windows named
mutex is derived from the lock file's path rather than being a fixed string, so two Curios
pointed at **different data roots** can both run. The invariant that matters is one writer
per library (R-DA-8); refusing a developer's scratch instance would be a false positive that
protects nothing.

## What the scaffold already demonstrates

Verified by running the release binary against a scratch data root, and by the test suite:

- **Boot order** (R-BE-5) end to end: migrate → bind `127.0.0.1:0` → publish `runtime.json`.
  Observed: schema stepped 0 → 4, ephemeral port assigned, file written at the app-data path.
- **`runtime.json` is never written before both** the migration and the bind succeed
  (R-BE-33), and it carries `{port, token, pid, version, state}` with a 43-character
  base64url token.
- **Stale reclaim** (R-BE-4): killing the process leaves the file behind, as a crash would;
  the next launch logs `reclaiming a stale runtime.json` and takes over.
- **`/health` returns exactly six fields** (R-SEC-11), asserted by test *and* observed live:
  `{status, version, port, items, queue, api_key_configured}`.
- **Route precedence** (R-BE-8): `/`, `/prompts/:id` → the SPA shell; `/api/items`, `/mcp`,
  `/assets/missing.js` → 404, never HTML.
- **A real shipped `library.db` opens and round-trips** on the existing chain (D20, P2's exit
  criterion) — 17 items, every status known to `curio-core`. Run it yourself with
  `CURIO_TEST_LIBRARY=<path> cargo test -p curio-db --test real_library`.

## How to record a result

Run the check, then edit the row: status, the number or observation, the method, and the
date. If a claim fails, take the documented fallback and say so — a budget may be
**consciously revised** (D17) but never silently missed. Add a row rather than overwrite one
when re-verifying after a dependency or OS update.
