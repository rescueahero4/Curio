/**
 * The MV3 service worker: the extension's only long-lived actor, which is also killed
 * roughly every thirty seconds.
 *
 * That contradiction shapes everything here. **Assume the worker dies at any moment**
 * (R-EXT-9): no module-level variable survives an event, so anything that must outlive one
 * lives in `chrome.storage.local`. The WebSocket is what keeps it warm — active traffic
 * resets the idle timer on Chrome 116+, which is why the manifest declares that floor
 * (R-EXT-10).
 *
 * ## What lives where
 *
 * * [`connection`](./connection) — finding Curio and staying authenticated to it: the
 *   native-messaging handshake, the fallback ladder, and the one-shot 401 re-handshake.
 * * [`socket`](./socket) — the WebSocket, the 20 s keepalive, and the paused/running state.
 * * [`capture`](../capture/pipeline) — the capture pipeline, whose ordering is normative.
 *
 * This file is the wiring: which events exist, and what each one does.
 *
 * ## 401 means the app restarted
 *
 * Not a pairing failure (R-EXT-18a, D21). The token is per-run, so a stale one is expected
 * every time the user quits and reopens Curio. `authedFetch` re-handshakes once and retries
 * before anything reaches the user, and only a second failure surfaces "Can't reach Curio".
 */

import { type CaptureMode, capture, resolveMode } from "../capture/pipeline";
import {
  type Connection,
  clearConnection,
  readConnection,
  writeConnection,
} from "../shared/storage";
import {
  authedFetch,
  bootstrap,
  ensureConnection,
  NotRunningError,
  PausedError,
} from "./connection";
import { connect } from "./socket";

export { NM_HOST } from "./connection";
export { KEEPALIVE_MS } from "./socket";

/** Re-run the ladder from scratch and bring the socket up. */
function coldStart(): void {
  void clearConnection()
    .then(() => bootstrap())
    .then(() => connect());
}

chrome.runtime.onInstalled.addListener(() => {
  // A fresh install holds nothing, and a stale record from a previous version would send
  // the first capture at a port nobody is listening on.
  coldStart();
});

// Chrome restarted. The port and the token from the last session are both dead — the token
// by design (D21), the port because it is ephemeral (D10).
chrome.runtime.onStartup.addListener(coldStart);

/** What the popup and the pairing content script send. */
type Request =
  | { type: "status" }
  | { type: "refresh" }
  | { type: "capture"; mode?: unknown }
  | { type: "open"; target: "projects" | "new-project" }
  | { type: "pairing-token"; token: string };

chrome.runtime.onMessage.addListener((message: Request, _sender, respond) => {
  switch (message?.type) {
    case "status":
      void status().then(respond);
      return true;

    case "refresh":
      void refresh().then(respond);
      return true;

    case "capture":
      // Anything that is not exactly "full" resolves to fold. A popup restored from a
      // previous session may hold a stale value, and the safe failure is a smaller capture
      // rather than a 60-frame scroll nobody asked for (R-EXT-12).
      void runCapture(resolveMode(message.mode)).then(respond);
      return true;

    case "open":
      void openAuthenticated(message.target).then(respond);
      return true;

    case "pairing-token":
      void acceptPairingToken(message.token).then(respond);
      return true;

    default:
      return false;
  }
});

/** The popup's first question. Cheap: reads storage, does not probe. */
async function status(): Promise<{ connection: Connection | null }> {
  const connection = await readConnection();
  // Opening the popup is a good moment to make sure the socket is up: the worker may have
  // been killed at any point since the last one.
  if (connection?.token) void connect();
  return { connection };
}

/** Re-run the ladder because the user asked. */
async function refresh(): Promise<{ connection: Connection | null }> {
  await clearConnection();
  const connection = await bootstrap();
  if (connection?.token) void connect();
  return { connection };
}

/** Run a capture, turning every failure into copy a person can act on. */
async function runCapture(
  mode: CaptureMode,
): Promise<{ ok: boolean; itemId?: string; truncated?: boolean; error?: string }> {
  try {
    const result = await capture(mode, (message) => {
      // Best effort: the popup may already be closed, which is not an error.
      chrome.runtime.sendMessage({ type: "progress", message }).catch(() => {});
    });
    return { ok: true, itemId: result.itemId, truncated: result.truncated };
  } catch (error) {
    if (error instanceof PausedError) {
      return { ok: false, error: "Curio is paused — resume it from the tray icon." };
    }
    if (error instanceof NotRunningError) {
      return { ok: false, error: "Can't reach Curio — open it and try again" };
    }
    return {
      ok: false,
      error: error instanceof Error ? error.message : "That didn't work.",
    };
  }
}

/**
 * Open a Curio page in a tab that is already signed in.
 *
 * A plain `chrome.tabs.create` lands on the no-session screen: the SPA's session is a
 * cookie the extension does not hold. So the worker spends its token on a **one-time
 * nonce** and puts that in the URL, which the SPA trades for the cookie on load
 * (R-EXT-19, R-FE-6a, R-SEC-5).
 */
async function openAuthenticated(
  target: "projects" | "new-project",
): Promise<{ ok: boolean; error?: string }> {
  try {
    const connection = await ensureConnection();
    if (!connection?.token) throw new NotRunningError();

    const response = await authedFetch("/api/auth/nonce", { method: "POST" });
    if (!response.ok) throw new NotRunningError();

    const { nonce } = (await response.json()) as { nonce?: string };
    if (!nonce) throw new NotRunningError();

    // "New Project" means "start a brief", so it lands on Prompts. The popup opens the
    // list rather than minting a prompt: a POST from the extension would leave an untitled
    // draft behind every time someone clicked through and changed their mind.
    const path = target === "new-project" ? "/prompts" : "/projects";
    const separator = path.includes("?") ? "&" : "?";
    await chrome.tabs.create({
      url: `http://127.0.0.1:${connection.port}${path}${separator}t=${encodeURIComponent(nonce)}`,
    });
    return { ok: true };
  } catch (error) {
    return {
      ok: false,
      error: error instanceof Error ? error.message : "Couldn't open Curio.",
    };
  }
}

/**
 * Accept a token from the `/pair` handoff.
 *
 * The fallback path only (R-EXT-8) — a normally installed extension gets its token from the
 * native-messaging host and this never runs. The content script has already applied its
 * gates, so what arrives here is *shaped* like a token; whether it is one, only the server
 * can say, and the first authenticated call is where that is answered.
 */
async function acceptPairingToken(token: string): Promise<{ ok: boolean }> {
  const existing = (await readConnection()) ?? (await bootstrap());
  if (!existing) return { ok: false };

  await writeConnection({ ...existing, token });
  void connect();
  return { ok: true };
}
