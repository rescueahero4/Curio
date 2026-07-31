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

The id derived from the pinned key is `oehjmjhhelijpkojhpichkfcgbdejhfa`.

## Build

```sh
npm install
npm run build      # → dist/, load unpacked in Chrome
npm run watch      # rebuild on change
```

## Development installs

An unpacked install has no registered native-messaging manifest, so `connectNative` fails
and the extension falls back: stored port → the legacy probe of 4321–4331 → the `/pair`
page handoff or a manual token paste (R-EXT-8). The probe can only find a **pinned** port,
so set `CURIO_PORT` when developing against an unpacked extension — an ephemeral port is
not discoverable by design (D10, D11).

## Status

Scaffold. The bootstrap ladder, the capture pipeline, and the popup land in **P5**. Each
module documents the contract it owes, because every ordering rule in the capture pipeline
was earned by a real bug — see `src/capture/pipeline.ts`.
