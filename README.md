# Curio

A local-first personal design inspiration library for AI-assisted design work.

Capture design references from the browser, let a vision model describe and organize
them in your own vocabulary, compose design prompts from that vocabulary, and catalog
the projects your AI tools produce. Everything runs on your machine: the browser is the
screen, the tray is the switch, one SQLite file is the memory.

> **Status: scaffold.** This repository is the Rust + SolidJS rewrite of Curio. The
> architecture is settled and contract-level ([`docs/architecture/`](docs/architecture/)),
> and the walking skeleton boots — but no product feature is implemented yet. See
> [Phase plan](docs/architecture/07-delivery-open-source.md) (R-DEL-21) for what lands when.

## Shape

One small binary — tray app, local HTTP server, SQLite — plus a SolidJS dashboard served
from inside that binary and a Chrome extension for capture.

```
curio (single binary)
  tray (main thread) ──mpsc──▶ service thread (tokio)
                                 ├─ /api + SSE      → the dashboard
                                 ├─ /ws             → the extension
                                 ├─ /mcp            → AI agents
                                 └─ SQLite (WAL)    → the library
```

- **One process.** The tray owns the main thread; everything real runs on one service thread.
- **One database file.** `library.db` in your data root is the whole backup story.
- **One origin.** The app serves its own UI at `http://127.0.0.1:<ephemeral port>` — no CORS, no bundled webview.
- **One token.** A per-run bearer token in `runtime.json`, handed to the extension by a
  native-messaging helper and to the dashboard by a one-time nonce.

## Privacy

**No telemetry.** Curio makes no network calls except the AI model calls you trigger with
your own API key. There is no analytics, no crash reporting, no update ping, no phone-home
of any kind. Adding one would require a major-version bump and an owner decision recorded
in the [decision register](docs/architecture/00-architecture-overview.md). Your API key
lives in the OS keychain — never in the database, the config file, the logs, or this repo.

## Build

Requires [Rust 1.95](https://rustup.rs) (pinned in `rust-toolchain.toml`) and Node 20+.

```sh
npm --prefix web/spa install     # once
cargo run                        # tray + server, proxying to the Vite dev server
npm --prefix web/spa run dev     # beside it, for frontend iteration
```

`cargo gate` runs the full quality gate — the same script CI runs, and the only definition
of "green" (R-DEL-6). See [CONTRIBUTING.md](CONTRIBUTING.md).

## Repository layout

| Path | What lives there |
|---|---|
| `crates/curio-core` | Domain types and rules. Sees no SQL, no HTTP, no MCP. |
| `crates/curio-db` | rusqlite, migrations, FTS5. The **only** crate that sees SQL. |
| `crates/curio-server` | axum router, middleware, SSE/WS, SPA embed, jobs worker, watcher. |
| `crates/curio-mcp` | MCP tool router and the stdio proxy. |
| `crates/curio-tray` | `main.rs` — native loop, tray menu, service thread. Builds the `curio` binary. |
| `crates/curio-nmh` | Native-messaging micro-binary. Reads `runtime.json`, replies, exits. |
| `crates/curio-runtime` | The `runtime.json` shape, shared by the server and the NM host. |
| `crates/xtask` | The gate script and measurement tooling. |
| `web/spa` | SolidJS + Vite + Tailwind 4 dashboard. |
| `web/extension` | MV3 capture extension, plain TypeScript. |
| `packaging/` | macOS `.app`, Windows installer, MCPB bundle. |
| `docs/architecture` | ARCH-00..08 — the contract these crates implement. |

## Documentation

The architecture documents are **contract-level**: they state interfaces, invariants and
budgets, and code must conform to their numbered rules. Start at
[ARCH-00 Architecture Overview](docs/architecture/00-architecture-overview.md), which maps
every domain to its owning document, then read the document for whatever you're changing.

## License

MIT — see [LICENSE](LICENSE).
