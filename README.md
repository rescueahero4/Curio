# Curio

A local-first personal design inspiration library for AI-assisted design work.

Capture design references from the browser, let a vision model describe and organize
them in your own vocabulary, compose design prompts from that vocabulary, and catalog
the projects your AI tools produce. Everything runs on your machine: the browser is the
screen, the tray is the switch, one SQLite file is the memory.

> **Status: E0–E9 complete, E10 (packaging) in progress.** The app runs, captures,
> assesses, and answers MCP. The release pipeline that builds the installers exists but has
> not yet cut a tag, so for now it is run from source — see
> [Running it in development](#running-it-in-development). Epic-by-epic status lives in
> [the PRD](docs/PRD-01-Foundations.md); what lands when is in the
> [phase plan](docs/architecture/07-delivery-open-source.md).

## Install

Once a release is tagged, installers are attached to it on the
[Releases page](https://github.com/rescueahero4/Curio/releases).

| Platform | File | What it does |
|---|---|---|
| Windows 10/11 | `curio-windows-x64-setup.exe` | Per-user install, no admin prompt. Registers the browser helper. |
| macOS 11+ | `curio-macos-universal.dmg` | Universal (Intel + Apple Silicon). Drag to Applications. |

The filenames carry no version so the site can link the newest build directly (D36); the
release page states the version, and so does the installed app under `--version`.

### Both platforms will warn you about the download

These builds are **not signed with a paid certificate**, so Windows SmartScreen and macOS
Gatekeeper flag them. That is what an unsigned build looks like, not a sign that something
is wrong with the file — and there is no free way around it on macOS, where Apple gates
signing behind a $99/year membership with no open-source exemption
([D34](docs/architecture/00-architecture-overview.md)).

- **Windows** — "Windows protected your PC" → **More info** → **Run anyway**.
- **macOS** — the app is blocked on first launch. **System Settings → Privacy & Security**,
  scroll to the message about Curio, **Open Anyway**.

If you would rather not take our word for it, everything here builds from source in two
commands — that path is below, and it is the same one CI runs.

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

---

# Running it in development

Run these in order. Steps 1–3 are required; steps 4–6 add optional capabilities and can be
done in any order afterwards.

## Prerequisites

| Tool | Version | Notes |
|---|---|---|
| [Rust](https://rustup.rs) | 1.95 | Pinned in `rust-toolchain.toml`; `rustup` picks it up automatically. |
| [Node](https://nodejs.org) | 20+ | Builds the dashboard and the extension. |
| Chrome / Edge / Brave | 116+ | Only for browser capture. That floor is where WebSocket traffic starts resetting the MV3 idle timer. |

No database to install and no services to start — SQLite is compiled in.

## 1. Install frontend dependencies (once)

```sh
npm --prefix web/spa install
npm --prefix web/extension install
```

## 2. Build the dashboard

**Do this before the first run.** The dashboard is served from inside the binary
and there is no Vite dev proxy yet, so an unbuilt dashboard means the app boots and serves
nothing.

```sh
npm --prefix web/spa run build
```

> **Frontend iteration:** debug builds read `web/spa/dist` from disk rather than baking it
> into the executable, so re-running the build above and refreshing the browser is enough —
> no Rust rebuild needed.

## 3. Run the app

```sh
cargo run --bin curio                       # terminal 1 — leave it running
npm --prefix web/spa run build -- --watch   # terminal 2 — rebuilds dist on save

```

`--bin curio` is not optional: the workspace builds two binaries — the app itself and the
native-messaging helper — so a bare `cargo run` cannot tell which one you meant and stops
with `could not determine which binary to run`.

This starts the tray icon, binds an ephemeral loopback port, migrates the database, and
opens your browser at the dashboard. To skip the browser-open:

```sh
cargo run --bin curio -- --no-open
```

**Where things land:**

| What | Windows | macOS |
|---|---|---|
| Your library (`library.db`, `items/`, `prompts/`, `skills/`) | `%USERPROFILE%\Curio` | `~/Curio` |
| `runtime.json` — port and per-run token | `%LOCALAPPDATA%\Curio` | `~/Library/Application Support/Curio` |
| `curio.lock` — quit token | `%LOCALAPPDATA%\Curio` | `~/Library/Application Support/Curio` |

`runtime.json` is deleted when Curio quits. Its absence is how everything else knows the
app is not running.

**To quit:** use the tray icon's Quit item, which runs the full shutdown sequence — worker,
then listener, then WAL checkpoint, then the token is destroyed.

### The console window is a debug-build feature, not a bug

A debug build opens a terminal window alongside the tray icon, printing lines like:

```
INFO curio_server: library opened schema=4
INFO curio_server: listening on 127.0.0.1 port=51693
```

That window **is** the log output, which is the point of a debug build. A release build sets
the Windows GUI subsystem and shows no console at all (R-DEL-9) — so if you want the
experience a user would get:

```sh
cargo build --release --bin curio
```

Then run `target/release/curio.exe` (macOS: `target/release/curio`) directly. It is also
about six times smaller, because the release profile optimises for size and strips symbols.

One log line is worth recognising: **`reclaiming a stale runtime.json from a previous run`**
means the last instance did not shut down cleanly — usually because it was killed rather
than quit from the tray. It is self-healing and safe to ignore; Curio holds the
single-instance lock, so any `runtime.json` it finds must belong to a dead process. Seeing
it after every clean Quit would be a real problem.

## 4. Add an Anthropic API key — optional, enables assessment

Without a key, captures still land and stay browsable. They sit at "Queued — needs an API
key" and the queue drains by itself the moment you add one. Two ways:

```sh
export ANTHROPIC_API_KEY=sk-ant-...       # macOS / Linux
$env:ANTHROPIC_API_KEY = "sk-ant-..."     # Windows PowerShell
```

Or paste it into **Settings → API key** in the dashboard, which stores it in the OS
keychain (DPAPI on Windows, Keychain on macOS). It is never written to the database, the
config file, or any log.

## 5. Load the browser extension — optional, enables capture

```sh
npm --prefix web/extension run build          # produces web/extension/dist
cargo run --bin curio-nmh -- --register       # tells the browser the helper exists
```

Then in Chrome: **`chrome://extensions`** → turn on **Developer mode** → **Load unpacked**
→ choose `web/extension/dist`.

The extension finds Curio by itself — no pairing step and no token to copy. Click its
toolbar icon; the popup should show a green dot and "Curio is running".

To undo the registration later:

```sh
cargo run --bin curio-nmh -- --unregister
```

## 6. Enable the MCP server — optional, lets AI agents read your library

Off by default. Turn it on in **Settings → MCP**, then point a client at it.

For Claude Code, one line does the whole registration:

```sh
claude mcp add --scope user curio -- <path-to-curio> --mcp-stdio
```

**Copy the real line from Settings → MCP** rather than typing this one. Nothing installs
`curio` onto your `PATH`, and Claude Code spawns the command without a shell, so a bare
`curio` resolves for nobody — the registration is accepted and then every connection dies
with "connection closed". Settings fills in the running executable's absolute path, which is
what makes the command work. Re-copy it if you move or reinstall Curio.

`--scope user` registers Curio for every project rather than the folder you ran it in, and
everything after `--` is the command Claude Code spawns.

The two transports underneath, for anything else:

- **stdio** — `curio --mcp-stdio`. It forwards to the running app rather than opening the
  database itself, so Curio must already be running. It re-reads `runtime.json` per frame,
  which is why it keeps working across restarts on an ephemeral port. Claude Desktop uses
  this one, as JSON in `claude_desktop_config.json`; Settings shows the snippet.
- **HTTP** — `http://127.0.0.1:<port>/mcp`. The port is in `runtime.json`, and Settings
  shows the command. Curio takes a new port each run unless you pin one in `config.json`,
  so a registration made this way goes stale at the next restart.

---

# Testing

## The gate — the only definition of "green"

```sh
cargo gate
```

Eight steps, fail-fast and cheapest-first: format → clippy → Rust tests → dashboard
typecheck/lint/build → extension typecheck/build → licences and advisories → file length →
dependency direction. **This is exactly what CI runs**; nothing is restated there, so if it
passes here it passes there.

Subsets while iterating:

```sh
cargo gate -- --rust-only     # skip the two npm builds
cargo gate -- --web-only      # only the dashboard and the extension
```

The licence and advisory step skips itself rather than installing tools behind your back.
To enable it once:

```sh
cargo install cargo-deny
```

## Rust tests

```sh
cargo test --workspace                                   # everything
cargo test -p curio-core                                 # domain rules: thresholds, prompts, retry policy
cargo test -p curio-db                                   # storage, migrations, FTS, sidecars
cargo test -p curio-server                               # routes, middleware, worker, images
cargo test -p curio-db --test real_library               # opens a real shipped library (NFR-6)
cargo test -p curio-server --test assessment_pipeline    # capture → assessed, against a stub API
```

Run `assessment_pipeline` whenever you touch the AI path. It boots the whole service
against a stubbed Anthropic API and asserts a capture reaches `ready` with tags, a family
and a sidecar — plus that the request Curio *built* had the right shape: two cache
breakpoints, the documented token budget, and a downscaled image.

## Frontend tests

```sh
npm --prefix web/spa run gate            # typecheck + lint + build
npm --prefix web/extension run gate      # typecheck + lint + build
npm --prefix web/spa run format          # auto-fix lint and formatting
```

## Other checks

```sh
cargo xtask files        # file-length budget: 500 hard, over 400 needs a PR justification
cargo xtask deps         # crate boundaries — enforced, not advisory
cargo xtask footprint    # idle memory against the 25 MB budget
```

## Manual end-to-end validation

The automated suite cannot spend a real API key, click a browser toolbar button, or use the
tray menu. For those, follow
**[docs/tests/manual-e2e-test-guide.md](docs/tests/manual-e2e-test-guide.md)** — a
step-by-step guide written to need no technical background.

---

## Environment variables

| Variable | Effect |
|---|---|
| `ANTHROPIC_API_KEY` | The model key. Overrides the keychain. |
| `CURIO_DATA_ROOT` | Use a different library folder — handy for testing against a scratch library. |
| `CURIO_PORT` | Pin the port instead of using an ephemeral one. Needed only for the extension's legacy probe fallback. |
| `CURIO_NO_OPEN=1` | Don't open the browser at boot; same as `--no-open`. |
| `RUST_LOG` | Log level, e.g. `RUST_LOG=debug`. Logs go to stderr. |

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
| `web/site` | The public landing page (Astro). Deployed to GitHub Pages; never shipped in the binary. |
| `packaging/` | macOS `.app`, Windows installer, MCPB bundle. |
| `docs/architecture` | ARCH-00..08 — the contract these crates implement. |
| `docs/tests` | Manual test guides. |

## Documentation

Documentation lives here in `docs/` and is read on GitHub — the landing page at
**https://rescueahero4.github.io/Curio/** publishes none of it, so this tree is the only
copy.

The architecture documents are **contract-level**: they state interfaces, invariants and
budgets, and code must conform to their numbered rules. Start at
[ARCH-00 Architecture Overview](docs/architecture/00-architecture-overview.md), which maps
every domain to its owning document, then read the document for whatever you're changing.

For what the product is meant to do, read [the PRD](docs/PRD-01-Foundations.md).

## License

MIT — see [LICENSE](LICENSE).
