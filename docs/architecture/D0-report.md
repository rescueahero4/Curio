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
| Release-0 rows with a recorded result | **13 of 13** |
| — of which: verified pass | 12 |
| — of which: closed by owner decision | 1 (row 11) |
| Moved out of release-0, re-targeted to a later phase | 1 (row 7 → P4) |
| Post-v1 sub-group (verified at the vector-activation release) | 2 |

## **E0 is COMPLETE.**

R-DEL-20's bar is *"every checklist item has a recorded result — pass, or fallback
chosen"*, and as of 2026-08-01 every release-0 row clears it. The gate that E1–E6 crossed
is now satisfied retroactively, which is worth stating plainly: it does not un-cross it,
and the process finding below stays in this document permanently.

Four owner decisions on 2026-08-01 closed the last of it, each recorded in the
[ARCH-00](00-architecture-overview.md) register with its rule amended in the same change
(R-DEL-18):

| # | Row | Decision |
|---|---|---|
| **D28** | 7 — rmcp pin | Major version **3.x**; the row leaves release-0 for **P4**, where the `StreamableHttpService` question can actually be answered. R-MCP-14 amended. |
| **D29** | 11 — MSI vs MSIX | **MSIX dropped**; NSIS or MSI, or both. The row tested MSIX sandboxing, so removing the format removed the question. R-DEL-9 amended. |
| **D30** | 1 — tray on macOS | Windows completes first; **macOS is verified retroactively** in a dedicated pass. Scheduled, not skipped. |
| **D31** | 4 — scrypt parameters | The encrypted-file fallback is **retired from v1**, not deferred. R-SEC-10 amended. |

What was measured rather than decided: rows **5, 6, 8, 9, 12 and 13** were verified in one
pass after the E1–E6 merge, and row 2 was re-measured against the finished server rather
than an empty shell. Row 13's verification found a shipped bug — `/ws` had never been
reachable — which is the clearest available argument that this gate is not ceremony.

**The one thing still outstanding is not a release-0 row.** macOS coverage (D30) is a
scheduled obligation: every result below is Windows-only, and the places macOS differs —
the tray's main-thread rule, the single-instance guard, file permissions, the keychain —
are exactly the ones with no shared implementation. Verifying them retroactively is the
accepted risk, not an oversight.

## Release-0 rows

| # | Item | Owner / OQ | Status | Result |
|---|---|---|---|---|
| 1 | **Tray crates** (`tray-icon` + tao): icon, menu, glyph swap, main-thread rules | [ARCH-01](01-backend-architecture.md) OQ-4 | ✅ Pass (Windows) — **macOS is retroactive by decision (D30)** | `tray-icon` 0.24.2 + `tao` 0.36.0 build and run on Windows 11; the five-item menu (D14) renders, the icon is generated in code, and the event loop sits in `ControlFlow::Wait`. macOS is **scheduled, not skipped**: the owner chose on 2026-08-01 to complete Windows first and verify macOS in a dedicated retroactive pass (D30), so this row no longer gates release-0. The risk is bounded and named — the main-thread rule, the single-instance guard, file permissions and the keychain are the places macOS differs, and all four are already behind per-platform code. |
| 2 | **Empty-shell RSS ≤ 12 MB** | [ARCH-01](01-backend-architecture.md) OQ-4, R-BE-31 | ✅ Pass (Windows) | **2.0 MB private commit** at scaffold (tray + axum listener + SQLite open, empty library). **Re-measured 2026-08-01 with the full P1–P3 server** — every route, both push transports, the projects watcher, one item in the library: **2.2 MB**. The entire route surface, SSE, WebSocket and watcher cost ~0.2 MB against a 12 MB gate. **See the measurement note below — the method matters more than the number.** |
| 3 | **TipTap core sans React**: chip node views and the slash menu under Solid's lifecycle | [ARCH-03](03-frontend-architecture.md) OQ-2 | ✅ Pass — **the ProseMirror fallback is not needed** | Verified 2026-08-01 by building the editor and shipping it. `@tiptap/core` + `@tiptap/pm` + `@tiptap/starter-kit`, all pinned **3.29.2**; `@tiptap/react` appears in neither `package.json` nor the lockfile, and no React arrives transitively. Eleven checks pass, three of which mount `EditorSurface` through `solid-js/web`'s `render()` and then `dispose()` — so `onMount → new Editor(el)` and `onCleanup → editor.destroy()` are exercised through Solid's own lifecycle rather than called directly. Chip node views render plain DOM, `itemRef` falls back to its stored label, the `section` attribute survives a round trip, ghost decoration lands on the empty paragraph, and the slash trigger fires after a space but **not** inside `http://`. See the finding below for what the spike actually cost. |
| 4 | **Keychain crates** (DPAPI + Security.framework) | [ARCH-06](06-security-architecture.md) OQ-3 | ✅ Pass — **and the scrypt half is retired, not deferred (D31)** | `curio-server/src/secrets.rs` implements the keychain-first store: `ANTHROPIC_API_KEY` → DPAPI (`CryptProtectData`, via `windows-sys` directly) on Windows, the `security` CLI on macOS under the previous implementation's `curio-anthropic-api-key` service name — so a macOS upgrade finds its existing key. The AES-256-GCM file fallback is **retired from v1** rather than shipped against guessed parameters: all three release targets (R-DEL-5) have a real keychain, so it was dead code on everything we ship. A legacy `.secrets.json` is detected and reported so an upgrading user re-enters a re-obtainable credential once. See the finding below. |
| 5 | **EcoQoS** via `SetThreadInformation` on the service thread (the Windows 11 ControlMask/StateMask gotcha) | [ARCH-01](01-backend-architecture.md) OQ-5 | ✅ Pass (Windows 11) — **implemented** | Verified 2026-08-01. `curio-tray/src/qos.rs` sets `THREAD_POWER_THROTTLING_STATE` on the service thread and Windows accepts it; the test asserts the **return value**, because a wrong mask pairing is a silent no-op that looks identical to success from outside. The gotcha resolved: `ControlMask` and `StateMask` must **both** carry `EXECUTION_SPEED` — control alone means "I manage this", state alone is ignored, and both zero clears the override. The main thread deliberately stays at default QoS so a Pause click never waits behind a throttled scheduler. |
| 6 | **`Sec-Fetch-Site`** actually sent on loopback fetches from extension and SPA contexts | [ARCH-06](06-security-architecture.md) OQ-1 | ✅ Pass (Chromium 1234, Windows) — **R-SEC-12 may be enforced** | Measured 2026-08-01 by loading the real unpacked extension into Chromium and fetching a throwaway echo server on loopback, so the answer is the browser's behaviour and not a property of our middleware. Extension **service worker** → `none`; extension **page** → `none`; **same-origin** page fetch → `same-origin`; a page on a **different site** (`localhost` → `127.0.0.1`, which Chrome treats as cross-site) → **`cross-site`, with an `Origin` header**. So the header is sent (the check is not vacuous), no legitimate context ever reports `cross-site` (enforcement cannot 403 capture), and the attack case does (the check does real work). See the finding below — this row was verified *after* the code that depends on it shipped. |
| 7 | **rmcp pin** | [ARCH-05](05-mcp-architecture.md) OQ-1 | ⚠️ **Finding — needs an owner decision** | See below. |
| 8 | **NMH cold-start on Windows** (Defender's first-run scan) keeps the popup dot sub-second | [ARCH-04](04-extension-architecture.md) OQ-4 | ✅ Pass (Windows 11) | Measured 2026-08-01, spawn → reply, which is what the popup actually waits on. **Cold 72.3 ms** against a binary copied to a path Windows had never seen, so the first call paid whatever first-run scan cost exists; **warm median 5.2 ms** over five runs. Against a 1 s budget that is ~14× headroom cold and ~190× warm, so no `{port, token}` caching ladder is needed. The reply on a dead instance was `{"state":"stale"}`, which also exercises R-BE-34's PID-liveness path. |
| 9 | **Unpacked-extension `key`/id**: same id unpacked as packed | [ARCH-04](04-extension-architecture.md) OQ-2 | ✅ Pass (Chromium, Windows) | The id **derivation** was already verified by test: `curio-server`'s `the_pinned_origin_is_the_one_chrome_derives_from_the_manifest_key` re-derives `oehjmjhhelijpkojhpichkfcgbdejhfa` from the manifest key and asserts it against the server's allowlist, so the two files cannot drift. The remaining half — whether Chrome assigns that id to an **unpacked** load — was answered incidentally on 2026-08-01 while verifying row 6: `--load-extension=web/extension/dist` produced exactly `chrome-extension://oehjmjhhelijpkojhpichkfcgbdejhfa`. Unpacked and packed agree, so the pinned origin allowlist works for a development install. |
| 10 | **`opt-level "z"` vs `"s"`**: size and hot-path speed | [ARCH-07](07-delivery-open-source.md) OQ-1 | ✅ Pass (Windows) — **pinned to `"z"`** | Measured 2026-08-01, x86_64-pc-windows-msvc, `curio.exe` with no SPA assets embedded: **`"z"` 3,207,168 bytes vs `"s"` 3,462,656** — `"z"` is 7.4 % (250 KB) smaller. Hot-path speed did not turn out to be the tiebreak OQ-1 expected: the process is idle almost all of the time, and the one genuinely hot path — SQLite — is bundled C that `opt-level` does not touch. `Cargo.toml` now pins `"z"` with the measurement recorded beside it. ARCH-07 OQ-1 is closed. |
| 11 | **MSI vs MSIX**: the NM registry write and Run-key autostart under MSIX sandboxing | [ARCH-07](07-delivery-open-source.md) OQ-2 | ⬜ Not started | Blocks P6. MSI is the safe default. |
| 12 | **MCPB tooling**: `mcpb pack` CLI shape and binary-server manifest fields | [ARCH-07](07-delivery-open-source.md) OQ-3, [ARCH-05](05-mcp-architecture.md) OQ-3 | ✅ Pass | Verified 2026-08-01 against `@anthropic-ai/mcpb` **2.1.2**. The CLI shape R-DEL-10 assumes exists: `mcpb pack [directory] [output]`, alongside `init`, `validate`, `sign`, `verify` and `info`. A **binary-server** manifest validates — `manifest_version 0.2`, `server.type: "binary"`, `server.entry_point`, and `server.mcp_config.{command,args}` with `${__dirname}` interpolation, which is how the bundle finds the packaged `curio` binary to run `--mcp-stdio` against. Signing exists as a separate step, so R-DEL-10's release job gains one. |
| 13 | **Chrome ~147 LNA gating** of loopback WebSockets | [ARCH-04](04-extension-architecture.md) OQ-1, [ARCH-06](06-security-architecture.md) OQ-5 | ✅ Pass (Chromium 1234) — **and it found a real bug** | Verified 2026-08-01 by opening `ws://127.0.0.1:<port>/ws` from the **real extension's MV3 service worker** and completing the D23 first-message handshake: `hello {state, version}` in **5 ms**, not gated. The secondary-sourced concern does not reproduce. The first attempt failed, and isolating it — a plain Node client failed identically, with no browser involved — proved the cause was ours, not Chrome's: see the finding below. |

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

### Row 13 — verifying it found that `/ws` had never worked

The first attempt to open a loopback WebSocket from the extension's service worker failed.
The tempting conclusion was the one the row was written to expect: Chrome's LNA work is
gating loopback sockets. That conclusion would have been wrong, and acting on it would
have meant redesigning the extension's push channel around a browser restriction that does
not exist.

Isolating it took one step — a plain Node client, no browser and therefore no LNA, failed
identically. The cause was ours: `/ws` had been grouped with the read routes, which put the
bearer/cookie credential layer in front of it. A browser `WebSocket` can send neither
headers nor cookies on the upgrade, so **every client was rejected with a 401 before the
handler ran**. R-BE-32 says the credential is the first message precisely because of that
constraint; the route's own comment said the shared layer did not apply, while the code
applied it.

Nothing caught this. The `/ws` unit tests asserted constants — the auth deadline, the shape
of the handshake frame — and never drove a socket, so they passed against a route no client
could reach. `cargo gate` was green throughout.

Fixed by moving `/ws` into the identity-only group, with a regression test that asserts a
credential-less upgrade is **not** a 401. The handshake now completes in 5 ms and returns
`hello {state, version}`.

Two things worth keeping from this. First, the row is genuinely a **pass**: LNA does not
gate loopback WebSockets from an MV3 service worker. Second, and more useful — a D0 row
that looks like it is about someone else's software found a defect in ours, in a feature
that would otherwise have appeared broken at P5 with the extension as the obvious suspect.

### Process — R-DEL-20's merge gate was crossed, and this is the record of it

**E1–E6 were built and merged to main while six index rows had no recorded result at
all** (5, 6, 8, 11, 12, 13). R-DEL-20 does not permit that. Its wording is blanket and
unambiguous: *"No Phase-1 work merges to main until every checklist item below has a
recorded result (pass, or fallback chosen)."* Not "every item that phase depends on" —
every item. The PRD agrees from the product side ("E0 … blocks all").

Two things are worth separating, because they carry very different weight.

**The one that mattered.** Row 6 blocks P1, and [ARCH-06](06-security-architecture.md)
OQ-1 is explicit about *why*: verify "**before P1 enforces R-SEC-12 as
reject-on-cross-site**". P1 shipped enforcing exactly that, unverified. It has since
passed — but that is luck, not diligence. Had Chrome labelled extension→loopback fetches
`cross-site`, the `identity` middleware would have 403'd **every capture**, and because
the extension is P5 the symptom would have surfaced phases away from its cause, against a
middleware that looked obviously correct. That is precisely the failure mode release-0
exists to prevent, and it is the argument for the blanket rule over a
"gate only what this phase touches" reading.

**The ones that did not.** Rows 8, 11, 12 and 13 block P5 and P6, which this pass did not
build, so [ARCH-00](00-architecture-overview.md) R-OV-4's narrower test — verified "before
code depends on it" — was never breached for them. Row 5 (EcoQoS) blocks P1 and is
unimplemented, so nothing depends on it either; the documented fallback ("ship at default
QoS") is in effect **by omission rather than by decision**, which is D17's "consciously
revised, never silently missed" failing in miniature. It needs an owner's ratification,
not a maintainer's assumption, and is left open here rather than quietly closed.

The gap between the two readings — R-DEL-20's blanket merge gate and R-OV-4's per-item
dependency test — is real, and the documents do not reconcile it. R-OV-2 says one fact has
one home; here two rules govern the same question with different strictness. Resolving
that is an owner decision and belongs in the [ARCH-00](00-architecture-overview.md)
register, not in this report. Until it is resolved, **R-DEL-20 is the operative rule**: it
is the stricter of the two, it lives in the document whose `governs` domain is delivery,
and the row-6 near-miss is evidence that the stricter reading earns its cost.

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


## Row 7 closed at P4 — rmcp 3.x exposes the configuration ARCH-05 assumes

Recorded 2026-08-01, during E9.

D28 pinned rmcp to major version 3 and moved this row out of release-0 to P4, on the
grounds that the second half of its question — whether `StreamableHttpService` still exposes
the stateless / JSON-response configuration R-MCP-4 rests on — could not be answered without
writing the MCP surface. The surface is written, so here is the answer.

**It does.** `rmcp 3.1.0`'s `StreamableHttpServerConfig` carries both knobs:

| Field | Value used | What it buys |
|---|---|---|
| `legacy_session_mode` | `false` | Every request served statelessly. Sessions were removed from protocol version `2026-07-28` (SEP-2567) and this flag only affects older versions, so stateless is now the default posture rather than an opt-out. |
| `json_response` | `true` | Simple request-response tool calls answer `application/json`; the transport still falls back to `text/event-stream` if a handler emits anything before its final response, so nothing is lost. |

Both matter for the same reason: the stdio proxy (D24) forwards frames to `/mcp` and re-reads
`runtime.json` per frame, because Curio can restart underneath a long-lived MCP client. A
session-bound transport would give the proxy state it has no way to recover across that
restart.

One incidental finding, recorded because it contradicts a line in the parity inventory:
Inventory §1 expects a disabled `GET /mcp` to answer `503`. rmcp serves `GET` itself in
stateless mode and returns `405 Method Not Allowed`. The property the inventory line exists
to protect — that `/mcp` never answers with the dashboard's SPA HTML, which an MCP client
reports as a parse error with no trace of the cause — holds: `/mcp` is nested above the
catch-all and `POST` returns a proper JSON-RPC refusal. The status code is rmcp's to choose
and not worth fighting.
