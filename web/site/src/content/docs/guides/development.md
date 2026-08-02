---
title: Running it in development
description: Prerequisites and the three required steps to run Curio from source.
---

There is no installer yet — E10 (packaging) has not started — so Curio is run from source.
Steps 1–3 below are required; the optional capabilities (API key, extension, MCP) are in
the [README](https://github.com/rescueahero4/Curio/blob/master/README.md), which is the
canonical, always-current version of this page.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs) | 1.95 | Pinned in `rust-toolchain.toml`; `rustup` picks it up automatically. |
| [Node](https://nodejs.org) | 20+ | Builds the dashboard and the extension. |
| Chrome / Edge / Brave | 116+ | Only for browser capture. That floor is where WebSocket traffic starts resetting the MV3 idle timer. |

No database to install and no services to start — SQLite is compiled in.

## 1. Install frontend dependencies

```sh
npm --prefix web/spa install
npm --prefix web/extension install
```

## 2. Build the dashboard

Do this **before** the first run. The dashboard is served from inside the binary and there
is no Vite dev proxy yet, so an unbuilt dashboard means the app boots and serves nothing.

```sh
npm --prefix web/spa run build
```

Debug builds read `web/spa/dist` from disk rather than baking it into the executable, so
re-running this build and refreshing the browser is enough for frontend iteration — no Rust
rebuild needed.

## 3. Run the app

```sh
cargo run --bin curio                       # terminal 1 — leave it running
npm --prefix web/spa run build -- --watch   # terminal 2 — rebuilds dist on save
```

`--bin curio` is not optional: the workspace builds two binaries — the app and the
native-messaging helper — so a bare `cargo run` cannot tell which one you meant.

This starts the tray icon, binds an ephemeral loopback port, migrates the database, and
opens your browser at the dashboard. Pass `-- --no-open` to skip the browser.

To quit, use the tray icon's Quit item: it runs the full shutdown sequence — worker, then
listener, then WAL checkpoint, then the token is destroyed.

## The gate

One script is the only definition of "green"
([R-DEL-6](../../architecture/07-delivery-open-source/)). CI invokes the same script and
never restates its steps, so a branch that passes locally passes on merge:

```sh
cargo gate
```

It runs `cargo fmt --check`, clippy with warnings denied, the workspace tests, the SPA and
extension typecheck/lint/build, `cargo-deny`, a file-length check, and a dependency-direction
check on `cargo tree`. If you want to change what the gate does, change `crates/xtask` —
not a workflow file.

## Working on this site

The site lives in `web/site` and is built separately from the gate: a docs deploy is not a
quality gate.

```sh
npm --prefix web/site install
npm --prefix web/site run dev
```

The architecture and product pages are **copies**, synced from `docs/` on every build by
`web/site/scripts/sync-docs.mjs` and gitignored. Edit `docs/`; a change made under
`web/site/src/content/docs/architecture/` is overwritten by the next build. Publishing a
new document means adding it to the `PAGES` allowlist in that script.
