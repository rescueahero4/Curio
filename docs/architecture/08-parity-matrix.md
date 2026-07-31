---
id: ARCH-08
title: Parity Matrix — what carries over, what changed, and where it lives
status: draft
version: 1.1.0
date: 2026-07-30
project: curio
supersedes: []
depends_on: [ARCH-00]
governs: [parity]
source_of_truth:
  - "Curiol PRD (docs/01-PRD-Foundations.md) FR-1..FR-27"
  - "Parity inventory (mined from the running Bun/React implementation)"
parity_reference: "Curiol (Bun/React implementation) + its PRD FR-1..FR-27"
---

# Parity Matrix

> **TL;DR:** This is the accountability document. Every product requirement (FR-1..27) and every behavioral invariant mined from the running app is mapped to the architecture document that now owns it. Anything the rewrite *deliberately* does differently is listed in one table with its reason — if a difference isn't in that table, it's a bug.

## At a glance

- 27 functional requirements → all owned; none dropped.
- 32 mined behavioral invariants ("Inventory §10.n") → all owned or explicitly superseded.
- 11 deliberate breaks, every one traceable to an owner decision (D-numbers in [ARCH-00](00-architecture-overview.md)).
- Use this doc in review: a PR touching an invariant cites its row; QA verifies rows, not vibes.

## The contract

- **R-PM-1** Every FR and every Inventory §10 invariant MUST have exactly one owning doc/rule here. Adding a feature or breaking an invariant REQUIRES updating this matrix in the same PR.
- **R-PM-2** A difference from the old app's observable behavior that is not listed under **Deliberate breaks** is a defect, regardless of how reasonable it looks.
- **R-PM-3** QA's Done-authority (project convention) extends to this matrix: a milestone is not Done while any of its rows is unowned or contradicted by code.
- **R-PM-4** A row MAY name a **primary owner** plus facet owners (the usual split is rules vs UI). The primary owner is listed first and **wins conflicts**; facet owners appear in parentheses with their facet named. A row with one owner needs no annotation.

## FR ownership

| FR | Requirement (short) | Owner | Notes |
|---|---|---|---|
| FR-1 | Single data root | [ARCH-02](02-data-architecture.md) R-DA-1..3 | Layout unchanged |
| FR-2 | Ingest via extension/drop/paste/picker | [ARCH-01](01-backend-architecture.md) (primary: routes; [ARCH-04](04-extension-architecture.md) capture facet, [ARCH-03](03-frontend-architecture.md) Tier-0 UI facet) | R-PM-4 split |
| FR-3 | Immediate `processing` card, in-place update | [ARCH-03](03-frontend-architecture.md) (primary; [ARCH-01](01-backend-architecture.md) SSE-emit facet) | R-PM-4 split |
| FR-4 | Single structured vision call | [ARCH-01](01-backend-architecture.md) AI layer | Two cache breakpoints preserved |
| FR-5 | Deterministic write-back incl. sidecar | [ARCH-02](02-data-architecture.md) R-DA-4 | |
| FR-6 | Threshold rules from Settings | [ARCH-01](01-backend-architecture.md) | decideFamilies semantics verbatim |
| FR-7 | Gray-zone badge + decision UI | [ARCH-03](03-frontend-architecture.md) | One-way doors per Inventory §10.13 |
| FR-8 | Everything editable, autosave, last_edited_by | [ARCH-03](03-frontend-architecture.md) (primary: UX; [ARCH-01](01-backend-architecture.md) stamping-rules facet, §10.12) | R-PM-4 split |
| FR-9 | Re-assess on demand | [ARCH-01](01-backend-architecture.md) | |
| FR-10 | Facet filters + FTS search | [ARCH-01](01-backend-architecture.md) (primary: API; [ARCH-02](02-data-architecture.md) R-DA-10 FTS facet) | LIKE fallback retired (break #6) |
| FR-11 | Bulk ops, cancellable, Batch API | [ARCH-01](01-backend-architecture.md) jobs contract | Freeze-at-enqueue, refunds, ≥8 batch; incl. vocabulary mgmt + dedupe pass (PRD-01 FR-11 Δ) |
| FR-12..16 | Prompt Helper (template, slash menus, paths, copy/send, autosave) | [ARCH-03](03-frontend-architecture.md) (primary: editor; [ARCH-01](01-backend-architecture.md) serializer/launch facet) | Send-to-Claude ordering §10.22 |
| FR-17..19 | Projects watch/launch/missing | [ARCH-01](01-backend-architecture.md) watcher contract | Marker-file identity §10.17 |
| FR-20 | Popup status via /health | [ARCH-04](04-extension-architecture.md) | /health fields are a contract (§10.29) |
| FR-21 | Fold/full capture, restore always | [ARCH-04](04-extension-architecture.md) | Pipeline invariants §10.23–24 verbatim |
| FR-22 | Clear "not running", never auto-launch | [ARCH-04](04-extension-architecture.md) | Plus new `paused` state (break #4) |
| FR-23 | Packaged app, tray Open + Quit | [ARCH-01](01-backend-architecture.md) (primary: tray/lifecycle; [ARCH-07](07-delivery-open-source.md) packaging facet) | Menu composition per D14 (break #5) |
| FR-24 | Single instance + port conflict handling | [ARCH-01](01-backend-architecture.md) | Reinterpreted under D10/D11 — ephemeral default + optional pin (break #1) |
| FR-25 | Tabs are clients; Quit stops cleanly | [ARCH-01](01-backend-architecture.md) shutdown sequence | Quit-token separation §10.3 |
| FR-26 | Offline-first; AI actions queue | [ARCH-01](01-backend-architecture.md) (primary: jobs/MissingApiKey; [ARCH-02](02-data-architecture.md) R-DA-13 embed-queue facet) | R-PM-4 split |
| FR-27 | Optional MCP, independent | [ARCH-05](05-mcp-architecture.md) | Per-request gating; stdio now gated too (break #11) |

## Inventory §10 invariant ownership

| §10.n | Invariant | Owner |
|---|---|---|
| 1 | Extension origin pinned to manifest key | [ARCH-04](04-extension-architecture.md) (primary; [ARCH-06](06-security-architecture.md) server-allowlist facet) |
| 2 | Origin check follows request Host + loopback regex | [ARCH-06](06-security-architecture.md) |
| 3 | Quit token never in /api CORS allowHeaders | [ARCH-06](06-security-architecture.md) |
| 4 | Project-serve refusal rules (dotfiles + reserved files) | [ARCH-06](06-security-architecture.md) |
| 5 | Settings response strips secrets structurally | [ARCH-06](06-security-architecture.md) |
| 6 | /mcp above SPA catch-all; disabled = 503 JSON-RPC | [ARCH-05](05-mcp-architecture.md) |
| 7 | Two vision cache breakpoints; no `effort` on utility | [ARCH-01](01-backend-architecture.md) |
| 8 | Dedupe schema `reason`-first; empty merge = withdrawal | [ARCH-01](01-backend-architecture.md) |
| 9 | Monotonic ULIDs | [ARCH-02](02-data-architecture.md) R-DA-5 |
| 10 | parkJob refunds the attempt | [ARCH-01](01-backend-architecture.md) |
| 11 | Bulk frozen at enqueue; over-cap refuses, never trims | [ARCH-01](01-backend-architecture.md) |
| 12 | last_edited_by stamping table | [ARCH-01](01-backend-architecture.md) |
| 13 | Gray-zone one-way doors; human score = 1.0 | [ARCH-01](01-backend-architecture.md) (primary: rules; [ARCH-03](03-frontend-architecture.md) UI facet) |
| 14 | Merge keeps MAX(score)/MIN(gray_zone); orphan pruning | [ARCH-02](02-data-architecture.md) R-DA-11 |
| 15 | Second-precision times; sent_at ms | [ARCH-02](02-data-architecture.md) R-DA-6 |
| 16 | deletePrompt nulls projects.prompt_id first | [ARCH-02](02-data-architecture.md) R-DA-11 |
| 17 | Marker-file identity rules | [ARCH-01](01-backend-architecture.md) watcher |
| 18 | Legacy env + data-root migration | [ARCH-02](02-data-architecture.md) R-DA-3 |
| 19 | Sidecars write-only, DB wins | [ARCH-02](02-data-architecture.md) R-DA-4 |
| 20 | Thumb-404 fallback; sharp-equivalent optional | [ARCH-01](01-backend-architecture.md) images |
| 21 | Pairing handoff DOM contract | [ARCH-03](03-frontend-architecture.md) (primary: page contract; [ARCH-04](04-extension-architecture.md) pickup-gates facet) — fallback path per break #2 |
| 22 | Send-to-Claude ordering; "asked to open" phrasing | [ARCH-01](01-backend-architecture.md) (primary: ordering/serializer rules; [ARCH-03](03-frontend-architecture.md) UI facet) |
| 23 | Popup listener/disable discipline | [ARCH-04](04-extension-architecture.md) |
| 24 | Capture ordering + unconditional teardown | [ARCH-04](04-extension-architecture.md) |
| 25 | item.created ignored under active filter | [ARCH-03](03-frontend-architecture.md) |
| 26 | Bulk delete keeps bar mounted via notice | [ARCH-03](03-frontend-architecture.md) |
| 27 | FTS optional / LIKE fallback | **Superseded** by [ARCH-02](02-data-architecture.md) R-DA-10 (break #6) |
| 28 | GET /api/projects repairs as it reads | [ARCH-01](01-backend-architecture.md) |
| 29 | /health cross-origin-readable with its fields | [ARCH-06](06-security-architecture.md) (primary: exposure rules; [ARCH-04](04-extension-architecture.md) consumer facet) |
| 30 | job.updated partial-merge contract | [ARCH-01](01-backend-architecture.md) (primary: emit shape; [ARCH-03](03-frontend-architecture.md) merge-UI facet) |
| 31 | 64 MB body cap ↔ 20k px stitch cap | [ARCH-01](01-backend-architecture.md) (primary: body cap; [ARCH-04](04-extension-architecture.md) stitch-cap facet) |
| 32 | Dependency roles (reference) | [ARCH-07](07-delivery-open-source.md) |

## Deliberate breaks

| # | Old behavior | New behavior | Why | Decision |
|---|---|---|---|---|
| 1 | Fixed port 4321, walk 4322–4331, `persistPort`, extension port-probing | Ephemeral `127.0.0.1:0` + `runtime.json`; optional config `port` override for dev/power users | Kills the port-conflict class; probing can't find an ephemeral port anyway | D10, D11 |
| 2 | Manual pairing token (mint at first run, paste into popup, `/pair` DOM handoff) | Per-run token via native-messaging bootstrap; `/pair` page survives only as the unpacked-install fallback | Token never crosses a web-observable channel | D10, D11, D12 |
| 3 | `/api/*` mostly unauthenticated (only ingest/quit checked) | Every `/api/*` and `/mcp` request requires the bearer token; SPA bootstraps via one-time nonce | Paper §4.4 threat model | D12 |
| 4 | Extension: health polling + POST only; no push channel; no paused concept | WS `/ws` + 20 s keepalive + `state` in handshake; paused stops capture cleanly | MV3 worker lifetime + soft-disable semantics | D2, D13, D23 |
| 5 | Tray (planned) menu incl. Open Projects / New Prompt | Five-item strategy menu (Status · Pause/Resume · Open Dashboard · Start at Login · Quit) | Tray is a switch, not a nav bar | D14 |
| 6 | FTS5 optional with LIKE fallback | FTS5 always present (bundled SQLite); fallback retired | We control the build now | R-DA-10 |
| 7 | No vectors, no graph (PRD non-goal) | Vector/graph designed-in but deferred post-v1 (owner 2026-07-31); v1 ships no embeddings — matching the old app | Design retained so activation is a migration, not a redesign | D7 (amended) |
| 8 | `curio.lock` + `.secrets.json` inside the data root | Run state + secrets in OS app-data dir | Data root is user-shareable/servable; §10.4 leak precedent | R-DA-2 |
| 9 | Settings reply includes `restartRequired` when the port changed | Field dropped — the port is ephemeral (or pinned at boot); no settings change requires a restart | The flag described a world with a persisted port; D10 removed that world | D10, D11 |
| 10 | Strategy §2 (as drafted): paused short-circuits **all** routes with 503 | Paused blocks mutations only; browse, search, SSE, and MCP read tools continue | A paused library you can still read is strictly better than a dead one; blanket 503 not implemented | D25 |
| 11 | MCP stdio transport always available regardless of the `mcpEnabled` toggle | The toggle gates both transports: the stdio proxy forwards to the gated `/mcp` (D24) | One gate, honest Settings copy | D24, PRD-01 FR-27 |

## Design detail — how to use this document

In planning: chunk work by FR rows; a story's acceptance criteria cite rows. In review: the diff's owner-doc rules + this matrix are the checklist. In QA: breaks table = expected diffs from the old app during side-by-side testing; anything else observed is a defect (R-PM-2). This document changes append-often: new rows land with features, but existing rows change only when an owning doc's rule changes version — which keeps it accurate without becoming a maintenance burden.

## Open questions
None — this document holds no decisions of its own; it indexes them.
