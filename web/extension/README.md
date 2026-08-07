# Curio Capture (MV3)

Captures the current page into the local Curio library.

## The `key` field is load-bearing

`manifest.json` pins a public `key`, which fixes the extension's id — and therefore its
`chrome-extension://…` origin, which `curio-server` allowlists. **This is one fact in two
files** (R-EXT-2, Inventory §10.1):

| Where | What |
|---|---|
| `web/extension/manifest.json` | the `key` |
| `crates/curio-server/src/security/origin.rs` | `EXTENSION_ORIGIN` |

A change to either without the other breaks capture with a 403 that looks like a pairing
problem. `curio-server` has a test that re-derives the id from this exact key, so the two
cannot drift silently — but the release checklist treats it as blocking regardless.

The id derived from the pinned key is `fclkgbgdifbddhlhnlhcfnijhcepdhke`. Don't transcribe it
by hand anywhere — `origin.rs` re-derives it from this manifest, and the last time two copies
were maintained by hand they drifted and capture died in production while the test stayed
green.

## Build

```sh
npm install
npm run build      # → dist/, load unpacked in Chrome
npm run watch      # rebuild on change
```

## Development installs

**Do this once and skip the rest of this section:**

```sh
cargo run --bin curio-nmh -- --register
```

That registers the native-messaging host against every Chromium-family browser on the
machine, and the extension then bootstraps the same way a shipped install does — it learns
the port *and* the token from `curio-nmh`, survives restarts, and needs no pinned port. The
manifest `key` above pins the extension id, so an unpacked build has the **same id** the
host allowlists; nothing about being unpacked excludes it from native messaging. Only the
missing registration does, and `cargo run` never performs one.

Re-run it if you move or rebuild to a different target directory — the manifest names an
absolute path to the binary, and a stale path fails silently.

### If you skip registration: the `/pair` escape hatch

Without a registered host, `connectNative` fails and the extension falls back down the
ladder in `src/worker/connection.ts`: stored connection → the legacy probe of ports
4321–4331 against `/health` (R-EXT-8). The probe can only find a **pinned** port, so you
also need `CURIO_PORT=4321 cargo run` — an ephemeral port is undiscoverable by design
(D10, D11).

That leaves the extension knowing *where* Curio is but holding no token, because `/health`
is unauthenticated and returns none. The popup shows **"Curio needs pairing"**. To resolve
it, open `http://127.0.0.1:<port>/pair` and click **Authorize this browser**.

There is no link to that page in Settings. There used to be, and it was removed: the state
is unreachable without a pinned port, so it is a development configuration rather than
something a user can encounter or act on.

**Expect to repeat it.** The token is per-run (D21), so every restart of Curio invalidates
it, and this path has no way to mint a new one — `authedFetch` re-handshakes once, the
ladder returns a port with an empty token, and the capture fails. Registering the host is
what makes restarts heal themselves.

### When the pairing page cannot help either

`/pair` needs the extension to already know the port. `acceptPairingToken` in
`src/worker/service-worker.ts` attaches the token to an existing connection and returns
`{ok: false}` when there is none — it never reads the port from the page, even though the
handoff is running on it. So with **no NM registration and no pinned port** the token is
picked up and discarded, and the extension cannot connect at all.

That combination is every macOS install today: nothing invokes `--register` there (the open
P6 gap in `packaging/README.md`) and the default port is ephemeral. Closing that gap — the
tray registering the host on first launch — is what retires this whole section, along with
`/pair`, its content script, and `POST /api/pair/authorize`.

### Summary: when `/pair` is the only fix

All four must hold at once. Miss any one and it is either unnecessary or useless:

| # | Condition | If not |
|---|---|---|
| 1 | Native-messaging host not registered | The extension re-handshakes on its own |
| 2 | Port pinned via `CURIO_PORT` or `config.json` | Nothing is found; `/pair` also fails |
| 3 | Curio running | Nothing to pair with — launch it |
| 4 | No valid token held | Already connected |

Reconnect in the popup cannot resolve this: it re-runs the same ladder, which has no rung
that produces a token without native messaging.

## Status

Scaffold. The bootstrap ladder, the capture pipeline, and the popup land in **P5**. Each
module documents the contract it owes, because every ordering rule in the capture pipeline
was earned by a real bug — see `src/capture/pipeline.ts`.
