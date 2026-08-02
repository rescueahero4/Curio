---
title: What Curio is
description: A local-first personal design inspiration library — the shape of the system and the decisions behind it.
---

Curio is a personal design inspiration library for AI-assisted design work. You capture
design references from the browser, a vision model describes and organizes them in your own
vocabulary, you compose design prompts from that vocabulary, and you catalog the projects
your AI tools produce back into the same library.

All of it runs on your machine. The browser is the screen, the tray is the switch, and one
SQLite file is the memory.

## The shape

One small binary — tray app, local HTTP server, SQLite — plus a SolidJS dashboard served
from inside that binary, and a Chrome extension for capture.

```
curio (single binary)
  tray (main thread) ──mpsc──▶ service thread (tokio)
                                 ├─ /api + SSE      → the dashboard
                                 ├─ /ws             → the extension
                                 ├─ /mcp            → AI agents
                                 └─ SQLite (WAL)    → the library
```

Four properties do most of the work:

- **One process.** The tray owns the main thread; everything real runs on one service
  thread. There is no daemon to supervise and no second process to leak.
- **One database file.** `library.db` in your data root is the whole backup story.
- **One origin.** The app serves its own UI from an ephemeral loopback port, so there is no
  CORS story and no bundled webview to keep patched.
- **One token.** A per-run bearer token in `runtime.json`, handed to the extension by a
  native-messaging helper and to the dashboard by a one-time nonce. It is destroyed on
  quit.

## Where the detail lives

| If you want | Read |
|---|---|
| The map of every domain and its owning document | [ARCH-00 Architecture Overview](../../architecture/00-architecture-overview/) |
| How the server, jobs worker and watcher fit together | [ARCH-01 Backend Architecture](../../architecture/01-backend-architecture/) |
| The schema, migrations and FTS design | [ARCH-02 Data Architecture](../../architecture/02-data-architecture/) |
| The token model, keychain handling and threat model | [ARCH-06 Security Architecture](../../architecture/06-security-architecture/) |
| What the product is meant to do, epic by epic | [PRD-01 Foundations](../../product/prd-01-foundations/) |

## Repository layout

| Path | What lives there |
|---|---|
| `crates/curio-core` | Domain types and rules. Sees no SQL, no HTTP, no MCP. |
| `crates/curio-db` | rusqlite, migrations, FTS5. The **only** crate that sees SQL. |
| `crates/curio-server` | axum router, middleware, SSE/WS, SPA embed, jobs worker, watcher, Anthropic transport. |
| `crates/curio-mcp` | MCP tool surface and the stdio proxy. |
| `crates/curio-tray` | `main.rs` — native loop, tray menu, service thread. Builds the `curio` binary. |
| `crates/curio-nmh` | Native-messaging micro-binary. Reads `runtime.json`, replies, exits. |
| `crates/curio-runtime` | The `runtime.json` shape, shared by the server and the NM host. |
| `crates/xtask` | The gate script and measurement tooling. |
| `web/spa` | SolidJS + Vite + Tailwind 4 dashboard. |
| `web/extension` | MV3 capture extension, plain TypeScript. |
| `web/site` | This site. Astro + Starlight, never shipped in the binary. |
| `packaging/` | macOS `.app`, Windows installer, MCPB bundle. |

The dependency direction is law, not convention: `curio-core` sees no SQL, `curio-db` is
the only crate that sees SQL, and server and MCP stay thin over core. It is asserted in CI
against `cargo tree` ([R-DEL-2](../../architecture/07-delivery-open-source/)).
