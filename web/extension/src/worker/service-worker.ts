/**
 * The MV3 service worker.
 *
 * **P5.** The bootstrap ladder, the WebSocket, and the capture pipeline land there. What
 * follows is the contract each part owes, recorded where whoever builds it will read it.
 *
 * ## Bootstrap: native messaging first
 *
 * `runtime.connectNative` → `curio-nmh` reads `runtime.json` → one reply
 * `{port, token, state}` → the host exits (R-EXT-4). One connect, one reply, no long-lived
 * channel. This closes the last manual setup step: the token rides the same reply, so a
 * fresh install captures with zero configuration.
 *
 * The fallback ladder runs **only** when `connectNative` fails — an unpacked install, or a
 * declined installer step (R-EXT-8): stored port → the legacy probe of 4321–4331 with an
 * 800 ms timeout → the `/pair` handoff or a manual token paste. The probe survives because
 * it costs three small code paths and saves a support thread, but note what it can and
 * cannot do: an ephemeral port is undiscoverable by design, so the probe only ever finds a
 * **pinned** port (D10, D11).
 *
 * ## The socket
 *
 * `ws://127.0.0.1:<port>/ws`, authenticated by sending the token as the **first message**
 * within 5 seconds (D23). Not a header — browser `WebSocket` cannot set one — and not a
 * query string, because those land in logs.
 *
 * Then an application-level ping every 20 seconds. This is the canonical MV3 keepalive:
 * active WebSocket traffic resets the worker's idle timer on Chrome 116+, which is the
 * whole reason for the version floor.
 *
 * **Pausing does not close the socket.** The state is announced, not disconnected
 * (R-EXT-11), and a paused app stops capture at the source — the buttons disable with an
 * explanation rather than posting into a 503.
 *
 * ## 401 means the app restarted
 *
 * Not a pairing failure (R-EXT-18a, D21). Re-run the NM handshake **once**, retry with the
 * fresh token, and show "Curio restarted — reconnecting…" meanwhile. Only if that also
 * fails does the extension surface "Can't reach Curio". The old "Pairing token rejected"
 * copy retired along with the pairing token itself.
 */

import { type Connection, clearConnection, readConnection } from "../shared/storage";

/** How often to ping. The canonical MV3 keepalive interval (R-EXT-10). */
export const KEEPALIVE_MS = 20_000;

/** The native-messaging host's registered name. */
export const NM_HOST = "com.curio.nmh";

chrome.runtime.onInstalled.addListener(() => {
  // A fresh install holds nothing, and a stale record from a previous version would send
  // the first capture at a port nobody is listening on.
  void clearConnection();
});

chrome.runtime.onMessage.addListener((message, _sender, respond) => {
  if (message?.type === "status") {
    void readConnection().then((connection: Connection | null) => {
      respond({ connection });
    });
    // Keep the channel open for the async reply.
    return true;
  }
  return false;
});
