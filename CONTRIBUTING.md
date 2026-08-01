# Contributing to Curio

Thanks for helping. This document is short because most of what governs the code lives in
[`docs/architecture/`](docs/architecture/) — those documents are the contract, and this one
just tells you how to build, check, and land a change.

## Two commands to a running app

```sh
npm --prefix web/spa install
npm --prefix web/spa run build
cargo run --bin curio
```

`--bin curio` is required: the workspace builds two binaries (`curio` and `curio-nmh`), so a
bare `cargo run` cannot tell which you meant. The SPA build is required too — see the note
below. Run the Vite dev server beside it for visual work:

```sh
npm --prefix web/spa run dev
```

> **Not yet, and deliberately recorded rather than left to be discovered:** R-DEL-3 also
> asks that *debug* builds proxy to the Vite dev server, so frontend iteration never needs
> a Rust rebuild. That proxy is **not implemented in the scaffold**. Today the binary serves
> whatever `web/spa/dist` held when it was compiled, and the Vite dev server runs on its own
> port (5173) with no session cookie — so it is useful for visual work and not for anything
> that talks to the API. The proxy lands with P3, when there is a dashboard whose iteration
> speed it would actually protect. Until then: `npm --prefix web/spa run build && cargo run --bin curio`.

For the extension, build it and load `web/extension/dist` unpacked in Chrome:

```sh
npm --prefix web/extension install
npm --prefix web/extension run build
```

Nothing in the dev loop needs signing, packaging, or network access beyond crates.io and npm.

## One gate

```sh
cargo gate
```

That is the **only** definition of "green" (R-DEL-6). CI invokes the same script and restates
nothing, so if it passes locally it passes in CI. In order, fail-fast:

1. `cargo fmt --check`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test --workspace`
4. SPA: typecheck, lint, production build
5. Extension: typecheck, build
6. `cargo deny check licenses advisories`
7. **File length** — any source file over 500 lines fails; 400–500 passes only with a
   justification in the PR description
8. **Dependency direction** — R-DEL-2's rules asserted against the real dependency graph

Useful subsets while iterating: `cargo xtask gate --rust-only`, `cargo xtask gate --web-only`.

## The rules that will bite you first

These are not style preferences. Each one is a numbered rule in an architecture document,
and the gate or a reviewer will hold you to it.

- **`curio-core` sees no SQL, no axum, no rmcp.** It defines traits; `curio-db` implements
  the storage ones. `curio-db` is the only crate that may depend on rusqlite. `curio-server`
  and `curio-mcp` stay thin — routing, transport, serialization — and call `curio-core` for
  anything that decides something. `curio-nmh` depends on no heavyweight crate at all: Chrome
  spawns it per connection, so its startup time is user-visible latency. (R-DEL-2)
- **500 lines per file.** This shapes how you decompose a module, so decide the split when you
  create the file, not when the gate rejects it.
- **Contract-level docs.** A pull request that violates a numbered rule either changes the
  code or changes the doc — in the same PR, with the doc's `version` bumped (R-DEL-18).
  Silent divergence between docs and code is a defect on its own.
- **Parity is accountability.** A difference from the old app's observable behavior that is
  not listed under [ARCH-08 §Deliberate breaks](docs/architecture/08-parity-matrix.md) is a
  defect, however reasonable it looks (R-PM-2).
- **No secrets, ever.** No tokens, API keys, or PII in commits, logs, fixtures, or test data.
  `.env*` and `.secrets.json` are gitignored from day one (R-DEL-17).

## Which document owns what

| Changing… | Read |
|---|---|
| the process, routes, jobs, watcher, AI calls | [ARCH-01 Backend](docs/architecture/01-backend-architecture.md) |
| schema, migrations, sidecars, the data root | [ARCH-02 Data](docs/architecture/02-data-architecture.md) |
| the dashboard | [ARCH-03 Frontend](docs/architecture/03-frontend-architecture.md) |
| capture, the popup, the NM host | [ARCH-04 Extension](docs/architecture/04-extension-architecture.md) |
| MCP tools or transports | [ARCH-05 MCP](docs/architecture/05-mcp-architecture.md) |
| auth, middleware, serve jails, secrets | [ARCH-06 Security](docs/architecture/06-security-architecture.md) |
| build, CI, packaging, licensing | [ARCH-07 Delivery](docs/architecture/07-delivery-open-source.md) |

Reading order for a new contributor: ARCH-00 → the PRD → ARCH-01 → the document your change
touches → ARCH-08 to see which invariants you now own.

## Commits and pull requests

Commit messages follow conventional-commit style — `feat:`, `fix:`, `docs:`, `refactor:`,
`test:`, `chore:` — because the changelog is generated from them and release notes are the
changelog delta (R-DEL-13). A message that doesn't parse gets fixed at review, not afterwards.

In the pull request, cite the rule IDs your change implements or touches. Reviewers cite them
back. If you touched `curio-server` middleware, the serve jails, `runtime.json`, `curio-nmh`,
or an MCP tool, complete the security checklist in the PR template (R-SEC-16).

## Making a decision

Curio uses an **ADR-lite** process: there is no `adr/` directory. A decision is a new row in
the register in [ARCH-00](docs/architecture/00-architecture-overview.md) — an ID, the decision,
the rationale, and the **reversal trigger** (the observable condition under which it should be
revisited). Add the row in the PR that acts on it. A decision big enough to need pages gets its
own document with the next ARCH-NN id (R-DEL-19).

## A note on the D0 spike

The verification spike is release-0: every claim in
[ARCH-07 §D0 index](docs/architecture/07-delivery-open-source.md) must have a recorded result —
pass, or fallback chosen — before Phase-1 work merges to main (R-DEL-20). Results go in
[`docs/architecture/D0-report.md`](docs/architecture/D0-report.md). If you find that a recorded
claim no longer holds, open a **D0 claim re-verification** issue; budgets and pins are claims
until measured, and they expire.
