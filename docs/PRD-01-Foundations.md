---
id: PRD-01
title: Curio — Product Requirements (Rust + SolidJS rewrite)
status: in-progress
version: 1.2.0
date: 2026-08-01
delivery: "E0 complete; E1–E9 built and green; E10 not started (see §6)"
owner: Robert Bagares
companion: "docs/architecture/00-architecture-overview.md (ARCH-00..08 own the how; this doc owns the what)"
supersedes: "Curiol docs/01-PRD-Foundations.md (FR numbering continues from it)"
---

# Curio — PRD-01: Foundations (Rewrite)

> **TL;DR.** Curio is a local-first design inspiration library: capture any design reference in one click, let AI describe and organize it in your own taste vocabulary, compose gold-standard design prompts from that vocabulary, and catalog what your AI tools build. The rewrite ships the **same product** as one small Rust binary with a SolidJS UI — lighter, simpler, tray-resident, fully offline-capable, open source. Nothing cloud, nothing hosted, nothing over-engineered.

## 0. Why this rewrite

| | Today (Bun/React) | This PRD (Rust/SolidJS) |
|---|---|---|
| Run experience | Terminal window must stay open | Double-click app, lives in tray, Pause/Resume |
| Footprint | JS runtime resident (~200 MB allowance) | ≤ 25 MB idle RSS budget (NFR-1) |
| Port & pairing | Fixed port walk + manual token paste | Ephemeral port, automatic extension pairing |
| Codebase | Organically grown, some 600–900-line files | Lean workspace, ≤ 400-line source files (NFR-8) |
| Audience | Personal tool | Credible open-source project (NFR-10..13) |

Feature scope goes **down, not up**: semantic (vector/graph) search is deferred to post-v1 (owner, 2026-07-31 — reverses ARCH-00 D7). Everything the running app does today is otherwise preserved.

## 1. Goals & non-goals

**Goals**

1. Capture → visible record ≤ 10 s of user effort; assessment ≤ 30 s typical.
2. Behave like a native app: double-click launch, tray presence, user's own browser as UI, Windows + macOS.
3. AI metadata good enough to keep the library curated with minimal correction (bar: ≥ 80 % of AI tags/descriptions accepted unedited, self-measured after rubric tuning); rubric editable as a markdown skill file.
4. Compose a complete gold-standard prompt in ≤ 5 minutes; zero-integration handoff (clipboard + filesystem paths) always works.
5. A codebase a stranger can read, run, and contribute to in an evening.

**Non-goals** (each with its destination)

- Vector/graph semantic search — *post-v1; schema seams stay designed-in per ARCH-02, never activated in v1*.
- Cloud sync, accounts, hosting, telemetry — *never*.
- Design generation inside the app — *never; external AI tools own it*.
- Safari/Firefox extension — *later port; Chromium-family first*.
- Mobile — *never in this codebase*.
- Automated project shipping/zipping — *manual stays fine*.

## 2. Users & stories

Primary persona: a product/UX designer running an AI-assisted prototyping workflow (Claude Code / Claude Desktop / Cowork), comfortable supplying an Anthropic API key. Secondary: any agent (MCP or plain filesystem) consuming the library.

| ID | Story | FRs |
|---|---|---|
| US-1 | As a designer browsing the web, I capture the page I'm looking at in one click and trust it lands in my library described in my own vocabulary. | FR-2..7, FR-20..22, FR-28 |
| US-2 | As a designer, I drop any image into the library and get the same AI curation as a web capture. | FR-2..7 |
| US-3 | As a curator, I review what the AI wrote, fix anything in place, and resolve ambiguous family matches with one decision. | FR-7, FR-8, FR-9 |
| US-4 | As a curator, I select many items and edit vocabulary or re-tag with AI in bulk — and can cancel without losing finished work. | FR-10, FR-11 |
| US-5 | As a designer starting new work, I compose a prompt from my library with slash-commands and hand it to my AI tool with paths it can read from disk. | FR-12..16 |
| US-6 | As a designer, the folders my AI tool writes show up in Curio by themselves, and launch in my browser with one click. | FR-17..19 |
| US-7 | As a user, I install Curio like a normal app; it starts at login if I want, pauses from the tray, opens its dashboard without ceremony, and quits cleanly. | FR-23..25, FR-29, FR-30 |
| US-8 | As an agent, I query and update the library through MCP tools — or with no integration at all, by reading sidecar files. | FR-1, FR-5, FR-27 |
| US-9 | As a privacy-conscious user, everything works offline except AI calls, which queue honestly instead of failing silently. | FR-1, FR-26 |
| US-10 | As an open-source contributor, I can build, test, and understand the repo without tribal knowledge. | NFR-8..13 |

## 3. Functional requirements

Numbering continues the Curiol PRD (FR-1..27 retained; **Δ** = amended here; FR-28+ new). One line each; acceptance lives in §6's epics, §7's done bar, and ARCH-08's invariant matrix. Behavioral fidelity to the running app is governed by ARCH-08 (deliberate breaks listed there; everything else must match).

**Library & ingestion**
- FR-1 **Δ** All user data (DB, screenshots, sidecars, config, prompts, projects) lives under one configurable local root; run state and secrets move to the OS app-data dir (ARCH-08 break #8).
- FR-2 Ingestion via extension POST, drag-drop, paste, and file picker; a screenshot is mandatory for every item.
- FR-3 New items appear immediately as `processing` and update in place when assessment completes (SSE, no polling).
- FR-4 Visual assessment is a single structured-output vision call returning name, description, tags, types, per-family scores, optional new-family proposal, optional image recipe.
- FR-5 Write-back is deterministic app code, atomic across DB + `item.md` sidecar; sidecars are regenerated projections agents can read with `cat`.
- FR-6 Family assignment follows the two user-configurable thresholds (assign ≥ upper; gray zone in between; propose new below lower).
- FR-7 Gray-zone items carry a badge and a one-decision resolution UI (keep / move / accept proposal).
- FR-8 Every metadata field is editable in place with autosave; user edits stamp `last_edited_by = user`.
- FR-9 Re-assess re-runs FR-4 on demand.
- FR-10 **Δ** Filter by type, family, tags (multi-select, AND across facets / OR within), needs-review, and full-text search; infinite scroll with stable keyset pagination.
- FR-11 **Δ** Bulk operations (vocab edit, AI re-tag augment/replace, delete) over an explicit or filter-defined selection, capped at 500 with honest refusal (never silent trimming); cancellable; ≥ 8 items uses the Batch API. Includes vocabulary management (rename / merge-keeping-links / delete) and the AI dedupe consistency pass (advisory, never auto-applied).

**Prompt Helper**
- FR-12 Editor loads the gold-standard template (Brief → Intent → Guardrails Always/Never → Design Direction → Important → Output) as deletable ghost sections.
- FR-13 **Δ** `/aesthetic`, `/style`, `/type`, `/item` slash-commands open live multi-select pickers; insertions become chips.
- FR-14 **Δ** Item chips serialize to absolute directory paths; family chips to name + full description. Serialization is server-side and authoritative; a `prompts/{id}.md` snapshot is written on save.
- FR-15 Copy Prompt places serialized text on the clipboard; Send to Claude copies first, then best-effort launches the configured target ("asked to open", never "opened").
- FR-16 Prompts autosave and are listed/reopenable; a sent prompt claims the next detected project (6 h window).

**Projects**
- FR-17 The projects root is watched; a new top-level folder appears in the catalog ≤ 5 s, identity carried by a `.curio-project` marker file.
- FR-18 Launch serves the folder through the local server (jailed) and opens it in the default browser.
- FR-19 A deleted folder marks the record `missing`, never silently removes it.

**Extension**
- FR-20 Popup shows live server status, pairing state, and the four actions (fold capture, full capture, Open Projects, New Project).
- FR-21 Two capture modes, one pipeline: fold (default, one frame) and full (scroll-stitch, 20k px cap); page chrome suppressed before measuring; teardown unconditional — on failure and on worker death (content-side watchdog).
- FR-22 Down state is honest ("not running", "paused"); the extension never launches the app. **Δ** adds the paused state (ARCH-08 break #4).
- FR-28 *(new)* Pairing is automatic via the native-messaging bootstrap; the `/pair` page remains as the manual fallback for unpacked installs. Manual token paste of a per-run token is gone (ARCH-08 break #2).

**Shell & platform**
- FR-23 Ships as a signed, double-clickable app per platform with a tray icon. **Δ** Tray menu is exactly: Status · Pause/Resume · Open Dashboard · Start at Login · Quit (ARCH-00 D14).
- FR-24 Single instance enforced; **Δ** port is ephemeral with `runtime.json` discovery; optional pinned port for dev (ARCH-00 D10/D11).
- FR-25 Closing tabs never stops the server; Quit checkpoints and exits cleanly.
- FR-26 Browse/filter/edit work fully offline; AI actions queue with honest messaging and drain when a key/network appears.
- FR-29 *(new)* Pause = soft-disable: mutations refuse with a clean error, reads keep working everywhere (UI banner, MCP reads alive, capture refused) (ARCH-00 D25).
- FR-30 *(new)* Open Dashboard uses a one-time nonce; the session survives reload; a token-less visit gets a static "Open Curio from the tray" screen, never an error (ARCH-00 D5/D22).

**MCP**
- FR-27 **Δ** Optional MCP server (Settings toggle, read per request): the 7 parity tools over Streamable HTTP and a stdio proxy for Claude Desktop, packaged as an MCPB bundle. The 2 semantic tools move to post-v1 with the vector layer. The toggle now gates **both** transports — the stdio proxy forwards to `/mcp` (ARCH-00 D24), so "off" means off everywhere (deliberate break from the old always-on stdio; recorded in ARCH-08).

## 4. Non-functional requirements

**Performance & footprint** *(measured, not asserted — tooling per ARCH-01 §budget)*
- NFR-1 Idle RSS ≤ 25 MB; empty tray+server shell ≤ 12 MB at D0 gate.
- NFR-2 Cold start (double-click → grid rendered) ≤ 5 s required, ≤ 2 s target; idle CPU ≈ 0 %; wakeups low single digits/s; never inhibits sleep.
- NFR-3 Library of 5,000 items: grid scroll stays 60 fps-class; search results < 100 ms locally.
- NFR-4 Ingestion cost a few cents/item: image downscaled ≤ 1568 px, prompt caching on rubric+vocabulary, Batch API for bulk.

**Reliability & data safety**
- NFR-5 One SQLite file (WAL) is the source of truth; every mutation is one transaction incl. sidecar + FTS; migration failure is a loud boot failure, never a broken half-state.
- NFR-6 The existing `~/Curio` data root and database open unmodified in the rewrite (lineage continuity, ARCH-00 D20).
- NFR-7 Loopback-only, token-authenticated, no telemetry, keys in OS keychain; the security contract is ARCH-06.

**Codebase quality** *(the open-source bar)*
- NFR-8 Source files target ≤ 400 lines; CI fails any file > 500; a 400–500-line file needs a one-line justification in its PR. One responsibility per file; UI fully componentized; no hardcoded values where a token/constant/config exists.
- NFR-9 Comments are rare and load-bearing (the *why*, never the *what*); the code and these docs are the documentation strategy. Public crate items get one-line rustdoc; no comment blocks narrating implementation.
- NFR-10 One gate command runs everything CI runs: `cargo fmt --check`, `clippy -D warnings`, `cargo test`, `cargo deny` (licenses + advisories), Biome lint/format on web code, `tsc --noEmit`, SPA + extension builds, file-length check. Green gate = mergeable.
- NFR-11 Tests follow the behavior, not the coverage number: every ARCH-08 invariant an epic claims gets a test named for it; logic libs port with their unit tests; one E2E smoke per surface (SPA, extension, MCP).
- NFR-12 Repo hygiene: MIT license, README that gets a stranger running in ≤ 5 minutes, CONTRIBUTING, SECURITY (private disclosure), CI badges, conventional commits, no secrets ever committed (per ARCH-07).
- NFR-13 Naming and layout follow ARCH-07's workspace (`crates/curio-*`, `web/spa`, `web/extension`, `packaging/`); dependency direction rules are CI-checked (`curio-db` is the only crate that sees SQL).

## 5. UX requirements

The rewrite **preserves the existing design language** — it is the product's identity, not incidental:

- **Visual character**: Awwwards-style directory. Warm light neutral ground (`#fafaf9`), border-defined white cards, screenshot-first 16:10 grid, quiet monochrome chrome where the accent is ink itself; exactly three semantic tints (amber caution, green confirmation, violet AI-proposal). Small type with tabular numerals; pill-shaped controls; 120 ms hover motion, reduced-motion respected. Design tokens are extracted verbatim from the current `styles.css` into the SPA theme — no re-invention, no hardcoded hex in components (NFR-8).
- **Brand assets reused as-is**: the magpie-and-gem mark (canonical vector in `BrandMark.tsx` / `extension/icons/mark.svg`), ink rounded-square app/extension icons (16/32/48/128), favicon, "Curio" wordmark. The AI persona is "Curio" in all copy ("Proposed by Curio").
- **Information architecture unchanged**: top bar (mark · Library/Projects/Prompts pills · centered search (Cmd/Ctrl-K) · Settings gear · + Add Item · New Project); filter pill row under it; Vocabulary reached from Library/Settings, not top nav; Prompt Helper as a white sheet on a gray desk, full-bleed.
- **Copy tone is a requirement**: honest status everywhere — "Queued — needs an API key", "Asked Claude Code to open — paste there", "Cancelled after N items. Everything already changed was kept." Empty states teach the next action. No fake spinners, no dead clicks; every disabled control says why (title or adjacent text).
- Per-page layouts, states, keyboard/focus behavior, and micro-interactions match the running app; ARCH-03 owns the enumerated contracts (this doc does not restate them). Where the old page was a monolith (Settings 889 lines, Vocabulary 794, ItemDetail 549, BulkBar 606), the rewrite delivers the same UX as composed components (NFR-8).

## 6. Epic list

> **Delivery status — updated 2026-08-01.** E0 is partly discharged, **E1, E2, E3, E4, E5
> and E9 are built and green**; E10 has not started. The status column
> below is the record. What "built" means here is specific: the gate passes
> (`fmt`, `clippy -D warnings`, 415 Rust tests, SPA typecheck/lint/build, extension build,
> file-length, dependency-direction), and the behaviour was exercised against a running
> binary, not only against unit tests.
>
> **E0 is now complete (2026-08-01), but it was not when E1–E6 merged — and that is
> recorded rather than tidied away.** R-DEL-20 is blanket: no Phase-1 work merges until
> every D0 row has a recorded result. This build crossed that gate with six rows
> unrecorded, and row 6 was the live risk — P1 shipped enforcing `Sec-Fetch-Site`
> reject-on-cross-site, which ARCH-06 OQ-1 says to verify *first*. It has since been
> verified and passes, but the sequence was wrong. Verifying row 13 afterwards found a
> shipped bug: `/ws` had never been reachable. The full account is the process finding in
> [D0-report](architecture/D0-report.md); it is not restated here.
>
> | Epic | Status | Note |
> |---|---|---|
> | E0 | ✅ **Complete** | All 13 release-0 rows carry a recorded result. Twelve verified; row 11 closed by decision (MSIX dropped, D29); row 7 re-targeted to P4 (rmcp 3.x, D28). macOS verification is a scheduled retroactive obligation (D30), not a gap. |
> | E1 | ✅ Done | Auth stack, nonce→cookie session, `/pair` fallback, pause soft-disable, SSE, `/ws`, both serve jails. |
> | E2 | ✅ Done | Exit criterion met: a real shipped `library.db` opens and round-trips on the existing chain. |
> | E3 | ✅ Done | Tokens, session bootstrap, shell, Settings as nine composed sections. |
> | E4 | ◐ Done bar AI wiring | Grid, selection/bulk, ItemDetail, Vocabulary. E7 now serves the endpoints the **AI re-tag** popover and **ConsistencyPass** were waiting on (`POST /api/bulk/retag`, `POST /api/bulk/dedupe`, `GET /api/bulk/dedupe/latest`); the two components still need connecting to them. |
> | E5 | ✅ Done | TipTap headless under Solid — which also closed E0's D0 row for it. |
> | E6 | ✅ Done | Watcher with marker identity, prompt claim, missing-not-deleted; Projects UI. |
> | E7 | ✅ Done | The queue drains. Worker loop (claim/park/refund/cancel), Anthropic transport, vision assessment with two cache breakpoints, image downscale, bulk retag (serial + Batch API), vocabulary dedupe, `verify-key`. Verified end to end against a stub API: a capture reaches `ready` with tags, a family, and a sidecar, unprompted. Two decisions recorded — D32 (JPEG container) and D33 (text re-tag). |
> | E8 | ✅ Done | NM registration (`curio-nmh --register`), the discovery ladder with a one-shot 401 re-handshake, the WebSocket keepalive, the capture pipeline with unconditional teardown and a content-side watchdog, and the popup's three states. Validated headed in real Chromium end to end. Found and fixed one real bug in the process: the manifest never declared `nativeMessaging`, so `connectNative` could not have worked. |
> | E9 | ✅ Done | Seven tools over rmcp 3.1 behind a `Library` trait, both gates per-request, `/mcp` above the SPA catch-all, and the stdio proxy. **D0 row 7 is answered**: rmcp 3.x does still expose the stateless / JSON-response configuration R-MCP-4 assumes. MCPB bundle outstanding. |
> | E10 | ⬜ Not started | NSIS/MSI installer (D29), OSS hygiene, budget pass. |


Phases and exit criteria are owned by ARCH-07 R-DEL-21; epics map onto them. Convention: an epic spanning server + UI lands its server semantics in P2 and its UI in P3. Stories are `S<epic>.<n>`; sub-tasks inline; a story's acceptance = the ARCH-08 rows for its FRs plus the §7 demo lines it enables. No estimates — risk is the sizing signal.

Dependency graph (E0 gates everything):

```
E0 → E1 → E2 ∥ E3 → E7 → E4 · E5 · E6 (parallel) → E8 · E9 → E10
          (E8 pairing/capture UI needs only E1; its end-to-end demo needs E7)
```

**E0 — Verification spike (D0)** · risk: High · blocks all
- S0.1 Tray crates on both OSes: sub-tasks · tray-icon+event loop POC Win/mac · Pause/Resume menu flip · measure empty-shell RSS (≤ 12 MB gate).
- S0.2 Toolchain proofs: · TipTap-headless-core mounted from Solid · keychain crate spike (DPAPI/Keychain) · rmcp pin + Inspector handshake · release-profile size report.
- *(sqlite-vec verification moved post-v1 with the vector layer.)*

**E1 — Shell & server core (P1)** · risk: Med · depends E0
- S1.1 Workspace scaffold: · cargo workspace + crates per ARCH-07 · gate script (NFR-10) · CI + file-length check.
- S1.2 Tray + service thread: · main-thread loop + mpsc seam · single instance (mutex/flock) · quit token + graceful shutdown.
- S1.3 Server bootstrap: · axum on `127.0.0.1:0` (+ pinned-port override) · `runtime.json` atomic write/reclaim · `/health`.
- S1.4 Auth: · bearer/cookie middleware + Host/Origin allowlist · nonce endpoint · `/pair` fallback.
- S1.5 Modes & push: · pause soft-disable (FR-29) · SSE skeleton · `/ws` skeleton.

**E2 — Data layer (P2)** · risk: Med · depends E1
- S2.1 `curio-db`: · rusqlite + WAL pragmas · shipped schema, migration chain from v4 · monotonic ULIDs · FTS5 always-on (ARCH-08 break #6).
- S2.2 Domain ops: · item/vocab/prompt/project queries with merge/prune/gray-zone invariants (ARCH-08 §10.11–17) · sidecar + snapshot writers in-transaction.
- Exit: existing `library.db` opens and round-trips losslessly.

**E3 — SPA foundation (P3)** · risk: Med · depends E1; parallel E2
- S3.1 Solid scaffold: · Vite + Tailwind tokens from old `styles.css` · solid-router routes + error boundaries · embedded-assets build.
- S3.2 Session & live data: · nonce→cookie bootstrap + no-session screen (FR-30) · shared SSE client + reconnect · API client.
- S3.3 Shell: · header/nav/search (Cmd-K) · AddItemDialog · missing-key banner.
- S3.4 Settings page: · composed sections (paths / pairing / API key with verify / model slots / rubric / thresholds / MCP toggle+snippets / startup) · blur-commit + per-section save badge with undo.

**E4 — Library & curation (P3)** · risk: Med · depends E2, E3
- S4.1 Grid: · FilterBar pills + density toggle · ItemCard states/badges · infinite scroll · SSE insert/update rules (§10.25).
- S4.2 Selection & bulk: · selection lib port (picked/matching, cap-refusal) · BulkBar as composed sub-components · bulk edit/delete flows.
- S4.3 ItemDetail: · autosave + SSE guard · gray-zone decision card · copy actions · agent-path card.
- S4.4 Vocabulary page: · tabs/sort/filter/search · rename/merge/delete with link-preserving copy · ConsistencyPass advisory-merge panel · componentized (NFR-8).

**E5 — Prompt Helper (P3)** · risk: High (editor) · depends E2, E3
- S5.1 Editor core: · TipTap headless in Solid · template ghost sections · toolbar (markdown-serializable formatting only).
- S5.2 Slash flow: · palette + entity picker (shared keyboard hook) · four chip nodes with label fallback.
- S5.3 Serialize & send: · server serializer parity · Copy/Send-to-Claude ordering invariant (§10.22) · sent-claim + "watching" banner.
- S5.4 Prompts list page: · rows with reuse/delete (disarm guard) · empty state naming the template.

**E6 — Projects (P3)** · risk: Low · depends E2, E3
- S6.1 Watcher: · notify-based watch + marker-file identity (§10.17) · missing/relocate reconciliation.
- S6.2 UI + launch: · cards with Launch/Open folder/PromptLink · jailed static serving.

**E7 — AI pipeline (P2–P3)** · risk: Med · depends E2
- S7.1 Jobs: · queue/claim/park/refund semantics (§10.10) · MissingApiKey queueing (FR-26) · cancellation.
- S7.2 Assessment: · ingest endpoint (multipart → item + enqueue) · vision call, two cache breakpoints, structured output · threshold application — E7 owns FR-6's write path end-to-end (E2 stores links only) · image downscale/thumbnail (sharp-equivalent optional).
- S7.3 Bulk AI: · retag serial/batch paths · dedupe (reason-first schema) · verify-key; Settings model slots.

**E8 — Extension + NMH (P5)** · risk: Med · depends E1 (E2+E7 for the end-to-end capture demo)
- S8.1 `curio-nmh`: · runtime.json read + PID check · NM manifests/registry (installer-written) · stdout purity.
- S8.2 Worker: · NM bootstrap + 401 re-handshake · WS + 20 s keepalive + paused state · capture pipeline port with watchdog teardown (FR-21).
- S8.3 Popup: · status/pairing states · four actions + toasts · paused UI.

**E9 — MCP (P4)** · risk: Low · depends E2
- S9.1 · 7 tools over rmcp/axum · per-request enable gate + paused=writes-refused · stdio proxy subcommand · MCPB bundle.

**E10 — Ship as open source (P6–P7)** · risk: Low · depends all
- S10.1 Packaging: · macOS bundle (LSUIElement, sign+notarize, SMAppService) · Windows installer (Run key, NM keys, EcoQoS) · clean uninstall.
- S10.2 OSS hygiene: · LICENSE/README/CONTRIBUTING/SECURITY · issue templates · release workflow + changelog.
- S10.3 Budget pass: · measure vs NFR-1..3 · record or consciously revise.

## 7. Done bar (demonstration)

Checked boxes were demonstrated on 2026-08-01 against a running binary on Windows 11.
Unchecked ones name what they are still waiting on — none is merely unverified.

- [ ] Fresh machine: install → tray icon → dashboard opens with no terminal anywhere; empty state teaches the first capture. *(waits on E10 packaging; the tray, dashboard and empty states themselves work from `cargo run`)*
- [x] Machine with a Bun-app-created `~/Curio` (schema v4): the rewrite opens it intact and round-trips losslessly (NFR-6). *(`cargo test -p curio-db --test real_library`, plus a live boot that stepped a fresh library 0 → 4)*
- [x] Chrome: install extension → it pairs itself → fold-capture a page → card appears processing → assessed with family/score badges ≤ 30 s. *(headed Playwright, real Chromium, extension loaded unpacked with its pinned id: the native-messaging handshake supplies port and token unprompted, a fold capture of an http fixture lands as a `processing` card with a first-fold thumbnail, and the page is left exactly as found. The assessment half is covered separately by `assessment_pipeline.rs` against a stub API — no real key was spent. **One caveat**: `activeTab` is granted only by a real click on the browser action, which Playwright cannot perform, so the run used a test copy of the extension with a broader host permission; the shipped manifest is unchanged.)*
- [x] Kill the API key: capture still lands, card says "Queued — needs an API key"; add key in Settings; queue drains unprompted. *(E7: a missing key parks the job on a 30 s timer without charging an attempt, and the item stays `processing` rather than failing — FR-26)*
- [x] Compose a prompt with `/aesthetic` + `/item` chips → Send to Claude → paste into Claude Code → it reads the referenced folders; resulting project folder appears in Curio ≤ 5 s and launches. *(chips serialize to an absolute path plus reading instructions; a sent prompt claimed the next folder the watcher saw, within the 5 s budget)*
- [x] Tray Pause: captures refuse politely, browsing and MCP reads still work; Resume is instant. *(mutations return `503 + Retry-After`, reads and SSE continue — D25; the MCP half waits on E9)*
- [◐] MCP Inspector + Claude Desktop (MCPB): all 7 tools answer; disabled toggle returns the clean 503. *(E9: `tools/list` returns all seven over rmcp 3.1, `library_search` returns the item the extension had just captured with absolute paths, and a disabled surface answers JSON-RPC rather than SPA HTML. **Two gaps**: the MCPB bundle is not built, and a disabled `GET /mcp` answers `405` rather than `503` — rmcp serves that verb itself in stateless mode. The property that mattered, never HTML, holds.)*
- [x] `git clone` → one documented command → green gate; no source file > 500 lines (files > 400 carry a PR justification); idle RSS measured ≤ 25 MB. *(`cargo gate` green; largest source file 444 lines; 2.2 MB private commit with the full server running)*

## 8. Open questions & governance

- OQ-1 Post-v1 trigger for the vector/graph layer (owner; revisit after v1 ships — ARCH-02/05 hold the design).
- OQ-2 Grid virtualization: adopt only if NFR-3 fails on real data at P7 (owner; default: don't).

Governance: this PRD changes by PR under the same process as the ARCH set (ARCH-07 R-DEL-18). Scope conflicts resolve in ARCH-00's decision register; behavioral-fidelity conflicts resolve in ARCH-08. This PRD owns *what* ships; it never overrides an ARCH contract on *how*.
