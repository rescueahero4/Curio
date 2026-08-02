# TEMPLATE — Authoring rules for Curio architecture documents (binding)

Audience note: the owner is non-technical but understands engineering terms. Lead with meaning, not jargon.

## Frontmatter (every doc, exactly this shape)
```yaml
---
id: ARCH-NN                  # stable id, used for cross-links
title: <Title>
status: draft                # draft → active → superseded (docs ship as draft, flip to active on approval)
version: 1.0.0
date: 2026-07-30
project: curio
supersedes: []
depends_on: [ARCH-00]        # ids this doc assumes you've read
governs: [<domain>]          # e.g. backend, frontend, extension, data, mcp, security, delivery
source_of_truth:
  # Planning inputs. Kept out of the published repo — docs/_plan/ is gitignored.
  - "docs/_plan/Architecture Solution Strategy.md"
  - "docs/_plan/local-first-rust-mcp-architecture-paper_1.md"
parity_reference: "Curiol (Bun/React implementation) + its PRD FR-1..FR-27"
---
```

## Structure (progressive reveal — strict order)
1. `> TL;DR:` blockquote — 2-4 sentences, plain language, what this doc decides.
2. `## At a glance` — one table or one Mermaid diagram + ≤6 bullets. A reader who stops here knows the shape.
3. `## The contract` — the durable rules: interfaces, invariants, budgets, decisions. Numbered rules (`R-XX-1`) so reviews and code can cite them. THIS is the part that must outlast the project — write rules, not code.
4. `## Design detail` — subsections with the reasoning and mechanics. Mermaid diagrams where a picture beats prose.
5. `## Parity obligations` — which FR-n and which parity-inventory invariants this doc owns (cite by number, e.g. "Inventory §10.24").
6. `## Open questions` — anything unresolved, each with an owner-decision needed or a D0-verification tag.

## Style
- Write rules in RFC-2119 language (MUST/SHOULD/MAY) where normative.
- No implementation code beyond tiny illustrative signatures/SQL/JSON. Contracts survive refactors; code samples rot.
- Mermaid for diagrams (```mermaid fenced). Tables for comparisons.
- Every cross-doc reference: `[ARCH-02](02-data-architecture.md)` — id + relative link.
- Do NOT restate another doc's rules; link to them. One fact, one home.
- Naming: the product/binary is **curio**; crates `curio-core`, `curio-db`, `curio-server`, `curio-mcp`, `curio-tray`, `curio-nmh`; web under `web/spa` and `web/extension`; packaging under `packaging/`.
- Target stack (fixed by decision, do not relitigate): Rust stable, axum 0.8, tokio current-thread, rusqlite (bundled) + sqlite-vec (D0-verify; fallback candidates: SQLite Vec1, FTS5-only-defer), rmcp ≥ 2 (floor 1.4.0), tray-icon + tao/winit event loop, SolidJS + Vite + Tailwind 4 + solid-router, TipTap (framework-agnostic core) or ProseMirror directly for the editor (doc must pick one and justify in one paragraph), MV3 extension in plain TS.
- Decisions already made by the owner: vector+graph ACTIVE in v1 rewrite scope; ephemeral port + runtime.json + native-messaging bootstrap replaces fixed ports + manual pairing; license MIT (permissive, recorded in ARCH-00 register).
- Doc length: 150-350 lines. Dense beats long.
