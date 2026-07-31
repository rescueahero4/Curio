---
id: ARCH-05
title: MCP Architecture
status: draft
version: 1.1.0
date: 2026-07-30
project: curio
supersedes: []
depends_on: [ARCH-00, ARCH-01, ARCH-02, ARCH-06]
governs: [mcp]
source_of_truth:
  - "docs/Architecture Solution Strategy.md"
  - "docs/local-first-rust-mcp-architecture-paper_1.md"
parity_reference: "Curiol (Bun/React implementation) + its PRD FR-1..FR-27"
---

> TL;DR: curio exposes its library to AI agents through one MCP server whose **v1 surface is the seven tools** the Bun app already ships (schemas preserved exactly); two vector/graph-era tools, `library_semantic_search` and `library_related_items`, are designed here but ship **post-v1 with the vector layer** (owner, 2026-07-31; D7 amended) — read the surface as **7 in v1, + 2 when the vector layer activates**. Two transports reach the same tools — Streamable HTTP at `/mcp` inside the main server, and a stdio subcommand that thinly proxies to it for Claude Desktop, packaged as an MCPB bundle. The Settings toggle gates both transports through the single `/mcp` gate — the stdio proxy forwards into it; "paused" blocks MCP *writes* but leaves reads working.

## At a glance

| | |
|---|---|
| Library | `rmcp` ≥ 2 (**floor 1.4.0**, CVE-2026-42559), served from crate `curio-mcp` |
| Tools | **7 parity tools in v1**; + 2 semantic tools post-v1 (vector activation), one shared tool router |
| HTTP transport | `StreamableHttpService` nested at `/mcp` in the axum router ([ARCH-01](01-backend-architecture.md)) |
| stdio transport | `curio --mcp-stdio` — thin proxy to the live instance's `/mcp` (D24), never opens the DB |
| Client packaging | MCPB bundle for Claude Desktop one-click install ([ARCH-07](07-delivery-open-source.md)) |
| Gating | `mcpEnabled` read per request at `/mcp`; gates both transports — the stdio proxy forwards into the same gate |

- Every tool call goes through `curio-core` service functions — the same validation and event paths REST uses. `curio-mcp` never touches SQL.
- Read tools work while the app is **paused**; write tools return a clean JSON-RPC error. Paused means paused for writes, not for consultation.
- Origin validation is curio's own middleware in front of `/mcp` — rmcp's post-CVE fix validates Host only ([ARCH-06](06-security-architecture.md)).
- Tool count and schema size are deliberately small: every registered tool costs client context tokens on every conversation.

## The contract

**Surface**

- **R-MCP-1** — In v1 the MCP server MUST expose exactly the seven parity tools in §Design detail; the two semantic tools join the surface post-v1 with the vector layer (R-MCP-2). The seven parity tools MUST preserve the Bun implementation's names, input schemas, output shapes, and behaviors exactly (Inventory §2), including: case-insensitive name→id resolution with unknown names silently dropped (`library_search`), whole-set replacement of tags/types stamping `last_edited_by='ai'` (`library_update_item`), `prompt_get` defaulting to `"latest"`, and `project_register` setting **no fingerprint** (an MCP-registered project is not rename-followable; identity is minted only on watcher adoption — Inventory §10.17).
- **R-MCP-2** — The two semantic tools (`library_semantic_search`, `library_related_items`) are **post-v1**: they ship with the vector layer's activation release (owner decision 2026-07-31, ARCH-00 register, D7 amended) and are not registered in v1. They MUST be read-only by construction and built on the sqlite-vec KNN virtual table and the `edges` graph tables defined in [ARCH-02](02-data-architecture.md) — no separate engine, no new write path. At that release, if sqlite-vec fails D0 verification **and** the Vec1 fallback also fails (D8), the two semantic tools are **withheld from the tool list** — never registered-but-erroring; the seven parity tools remain the surface regardless, and D15 reads "7 in v1, + 2 when the vector layer is active".
- **R-MCP-3** — Adding, removing, or renaming a tool is a contract change: it MUST be reflected here and in [ARCH-08](08-parity-matrix.md) before shipping. Tool descriptions and schemas MUST stay compact (target: whole `tools/list` response well under 2 KB of description text) — registered tools tax every client conversation's context window.

**Transports**

- **R-MCP-4** — HTTP transport: `rmcp` `StreamableHttpService` MUST be mounted at `/mcp` in the main axum router, **above the SPA catch-all** (a disabled or unknown `/mcp` request must never receive SPA HTML — Inventory §10.6). Config: stateless mode, JSON responses preferred; the service factory runs **per request**, so DB handles, settings, and the event bus MUST be captured in `Arc` handles cloned into each instance — no per-request pool construction.
- **R-MCP-5** — stdio transport: `curio --mcp-stdio` is a **thin proxy** (D24). It reads `runtime.json` for the live instance's endpoint and token and forwards JSON-RPC frames to that instance's `/mcp`; it NEVER opens the database and starts no HTTP listener, tray, watcher, or job worker of its own. stdout is reserved exclusively for MCP frames; all logging goes to stderr (**stdout purity**). With no live instance (no `runtime.json`, or stale per [ARCH-01](01-backend-architecture.md)), it MUST return a clean JSON-RPC error telling the user to start Curio — never a hang, never a direct DB open.
- **R-MCP-6** — The stdio transport **IS gated** by `mcpEnabled`, via the single `/mcp` gate: the proxy forwards every frame to `/mcp`, so when MCP is disabled it receives the same 503 JSON-RPC error and passes it through unchanged — one gate, honest Settings copy (supersedes the old always-on stdio — [ARCH-08](08-parity-matrix.md) break #11).
- **R-MCP-7** — Claude Desktop packaging: because `claude_desktop_config.json` accepts stdio servers only, curio MUST ship an MCPB bundle (`manifest.json` + `mcpb pack`, binary-server form) wrapping the stdio subcommand, alongside the native `/mcp` HTTP endpoint for URL-capable clients. Bundle build and distribution: [ARCH-07](07-delivery-open-source.md).

**Gating and paused semantics**

- **R-MCP-8** — `mcpEnabled` MUST be read **per request**, never cached at router construction. When disabled, `POST /mcp` MUST return HTTP 503 with a JSON-RPC error body `{code: -32000, data: {reason: "mcp_disabled", ...}}`; the stdio proxy receives that same 503 JSON-RPC from `/mcp` and passes it through unchanged (R-MCP-6).
- **R-MCP-9** — `GET /mcp` and `DELETE /mcp` MUST return HTTP 405 with a JSON-RPC error body (stateless JSON mode uses POST only).
- **R-MCP-10** — **Paused ≠ disabled** (D25). When the app is paused (soft-disable, strategy §2/§6): read tools (`library_search`, `library_get_item`, `library_list_vocabulary`, `prompt_get`; post-v1 also `library_semantic_search`, `library_related_items`) MUST keep working; write tools (`library_create_item`, `library_update_item`, `project_register`) MUST return a clean JSON-RPC error `{code: -32000, data: {reason: "paused"}}` without side effects. [ARCH-01](01-backend-architecture.md)'s paused middleware passes `/mcp` through untouched — enforcement lives at **tool dispatch**, where read and write tools can be told apart. Paused means paused for MCP **writes**; consultation of the library is never interrupted by the pause toggle.
- **R-MCP-11** — Disabling MCP (or pausing) MUST NOT affect any other feature (FR-27); the toggle's blast radius is the `/mcp` surface only (both transports, since stdio proxies into it).

**Security and layering**

- **R-MCP-12** — curio's own Origin-validation middleware MUST sit in front of `/mcp` (rmcp's Host-only validation leaves the Origin gap open — rust-sdk #822). Token, Host, and Origin rules are owned by [ARCH-06](06-security-architecture.md); this doc only requires that `/mcp` is behind them.
- **R-MCP-13** — Every MCP tool MUST call `curio-core` service functions — the same code paths REST handlers use, with the same validation, threshold logic, event emission (`item.updated` etc.), and sidecar write-back. `curio-mcp` MUST NOT contain SQL or bypass core invariants. Blast radius of the MCP surface = blast radius of the core API, nothing more (Paper §4.2 / CVE lesson). The stdio proxy satisfies this trivially: it contains no tool code at all, so single-writer and event fan-out are preserved by construction (D24).
- **R-MCP-14** — rmcp version floor is **1.4.0** (Host-validation fix); the workspace pins major version 2. Downgrading below the floor is forbidden; cargo-deny advisories in CI ([ARCH-07](07-delivery-open-source.md)) enforce it.

## Design detail

### Transport / client matrix

```mermaid
flowchart LR
  subgraph Clients
    CC[Claude Code / Cursor /\nURL-capable clients]
    CD[Claude Desktop]
    INS[MCP Inspector / CLI]
  end
  subgraph curio process
    MW[ARCH-06 middleware\nHost + Origin + token] --> SVC["/mcp StreamableHttpService\n(stateless, per-request factory)"]
    SVC --> TR[Tool router - 7 tools in v1\n+2 semantic post-v1]
    TR --> CORE[curio-core services]
    CORE --> DB[(curio-db\nSQLite + vec0 + edges)]
  end
  subgraph curio --mcp-stdio
    STD[stdio proxy\nstdout = frames only\nnever opens the DB]
  end
  CC -- "Streamable HTTP\nrun-time port from runtime.json" --> MW
  INS -- Streamable HTTP --> MW
  CD -- "MCPB bundle → spawns\ncurio --mcp-stdio" --> STD
  STD -- "forwards to /mcp\n(endpoint+token from runtime.json)" --> MW
```

| Transport | Entry | Gated by `mcpEnabled` | Discovers server via |
|---|---|---|---|
| Streamable HTTP | `POST /mcp` (GET/DELETE → 405) | Yes, per request | `runtime.json` (port + token) |
| stdio | `curio --mcp-stdio` (proxy to `/mcp`, D24) | **Yes** — via the `/mcp` gate; 503 JSON-RPC passed through (break #11) | `runtime.json` (endpoint + token) |
| MCPB bundle | Claude Desktop one-click | Yes (wraps stdio) | manifest points at installed binary |

### Tool surface — parity seven (schemas fixed, Inventory §2)

| Tool | Input | Output | R/W |
|---|---|---|---|
| `library_search` | `{query?, design_types?, families?, tags?, limit ≤50 def 20}` | `{total, items:[{id, name, short_description, source_url, status, directory, design_types, tags, families:[{name, score}]}]}` | R |
| `library_get_item` | `{item_id}` | full item + **absolute** screenshot + sidecar paths | R |
| `library_list_vocabulary` | `{}` | families `{id, name, description, item_count}`; types, tags as name arrays | R |
| `library_create_item` | `{screenshot_path, source_url?, name?}` | copies file into the data root, queues assessment | W |
| `library_update_item` | `{item_id, name?, short_description?, image_recipe?, tags?, design_types?}` | tags/types replace whole sets; stamps `last_edited_by='ai'`; writes sidecar; emits `item.updated` | W |
| `prompt_get` | `{prompt_id def "latest"}` | serialized prompt text + structuredContent `{id, title}` | R |
| `project_register` | `{path, name?, prompt_id?}` | registers with origin `"mcp"`; **does NOT set a fingerprint** | W |

Filter names in `library_search` resolve case-insensitively to ids; unknown names are silently dropped, never errored — agents retry with `library_list_vocabulary` when results look thin.

### Tool surface — semantic two (new in the rewrite, post-v1)

These are the curio-vocabulary descendants of the strategy's `search_captures` / `related_captures` sketch (strategy §9, Phase 4), renamed to match the library domain. They ship with the vector layer's activation release (R-MCP-2), not in v1.

**`library_semantic_search`** — KNN over item embeddings, filterable by the same facets as `library_search`.

```json
in:  { "query": "string (embedded server-side)",
       "k": "int ≤ 50, default 10",
       "design_types": ["name"], "families": ["name"], "tags": ["name"] }
out: { "total": 0,
       "items": [ { "...same item shape as library_search...",
                    "distance": 0.0 } ] }
```

Item shape is identical to `library_search` items plus a `distance` field (lower = closer), so agents can treat the two search tools interchangeably. Facet names resolve exactly as in `library_search` (case-insensitive, unknown dropped). Embedding of the query string happens server-side behind the embedder trait ([ARCH-02](02-data-architecture.md)); if no embedder is configured, the tool MUST return a clear JSON-RPC error naming the missing configuration rather than empty results.

**`library_related_items`** — graph-and-vector neighborhood of one item.

```json
in:  { "item_id": "string",
       "max_hops": "int ≤ 3, default 2",
       "edge_kinds": ["tagged_with | typed_as | in_family | same_domain | from_prompt | produced_project"],
       "k": "int ≤ 50, default 10",
       "include_semantic": "bool, default true" }
out: { "items": [ { "id": "", "name": "", "short_description": "", "directory": "",
                    "relation": { "via": "graph | vector | both",
                                  "hops": 1, "edge_kinds": ["same_domain"],
                                  "distance": 0.0 } } ] }
```

The `edge_kinds` enum is exactly [ARCH-02](02-data-architecture.md) R-DA-15's six v1 kinds, underscore spelling; ARCH-02 owns that vocabulary — this doc cites it and adds nothing. Implementation is the single hybrid SQL statement ARCH-02 exists to make possible: bounded-depth recursive CTE over `edges` joined (when `include_semantic`) with vec0 KNN against the source item's embedding; results deduplicated, `via:"both"` when an item appears in each. Both tools are read-only by construction (R-MCP-2) and therefore remain available while paused (R-MCP-10).

### Request lifecycle (HTTP)

1. [ARCH-06](06-security-architecture.md) middleware: Host allowlist, Origin allowlist, bearer token → 403 on failure.
2. Method gate: GET/DELETE → 405 JSON-RPC (R-MCP-9).
3. `mcpEnabled` read from live settings → disabled → 503 JSON-RPC per R-MCP-8.
4. Stateless service factory constructs handler from `Arc` handles; tool dispatch.
5. Write tool + paused state → JSON-RPC "paused" error (R-MCP-10); otherwise `curio-core` executes, events fire, response returns as JSON.

The stdio proxy's forwarded requests carry the runtime token from `runtime.json`, so they pass step 1 like any client and share steps 2–5 in full; per R-MCP-6 the `mcpEnabled` gate (step 3) applies to them too — when disabled, the proxy passes the 503 JSON-RPC through unchanged (supersedes the old always-on stdio — ARCH-08 break #11). One pipeline, one gate, two doors (D24).

### Token-overhead discipline

Nine tools (seven in v1, plus the post-v1 semantic pair) is the budget, not the floor to grow from. Each tool's description is one sentence plus the schema; no examples embedded in descriptions; enums and defaults expressed in the schema itself. Rationale: MCP clients inject all tool schemas into model context on every conversation — a verbose surface silently taxes every user of every connected agent. New tool proposals must argue they can't be an argument on an existing tool.

## Parity obligations

- **FR-27** — optional MCP server, Settings toggle, disable affects nothing else (R-MCP-8, R-MCP-11).
- **PRD §8.6** — agent flows: search, read items/vocabulary, fetch prompts, create/update records, register a project; copy-paste flow must always work with MCP disabled.
- **Inventory §2** — all seven tool schemas and behaviors, both transports, per-request gating, 503/405 JSON-RPC shapes, stdout purity (the old stdio-ungated posture is superseded — ARCH-08 break #11), Settings showing both config snippets (HTTP + stdio) — the snippet display itself is owned by [ARCH-03](03-frontend-architecture.md).
- **Inventory §10.6** — `/mcp` above the UI catch-all; disabled GET `/mcp` returns JSON-RPC, never SPA HTML.
- **Inventory §10.12** — MCP update stamps `last_edited_by='ai'`.
- **Inventory §10.17** — `project_register` never mints a fingerprint.

## Open questions

| # | Question | Tag |
|---|---|---|
| 1 | rmcp 2.x exact pin: verify current minor on crates.io, confirm `StreamableHttpService` stateless-mode + JSON-response config API matches the paper-brief signature (`with_legacy_session_mode(false).with_json_response(true)`). | D0-verify, blocks: P4 |
| 2 | Embedder configuration surface (local model vs API, which Settings fields) is owned by [ARCH-02](02-data-architecture.md) (its OQ-2); this doc only needs the "unconfigured → clear error" rule confirmed against its final shape. | owner: ARCH-02, blocks: P4 |
| 3 | MCPB manifest schema and `mcpb pack` binary-server support: re-verify current tooling before P6 packaging (paper brief §7 notes the format renamed from `.dxt`). | D0-verify, blocks: P6 |
