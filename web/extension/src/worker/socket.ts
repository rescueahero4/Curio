/**
 * The WebSocket to Curio, and the reason it exists at all.
 *
 * Two jobs, and the second is the surprising one:
 *
 * 1. **Learn the run state.** `running` or `paused`, pushed on every change, so the popup
 *    can disable capture with an explanation instead of posting into a 503 (R-EXT-11).
 * 2. **Keep the service worker alive.** MV3 kills a worker after ~30 s idle, and active
 *    WebSocket traffic resets that timer on Chrome 116+ — which is the entire reason the
 *    manifest declares a 116 floor. The 20-second ping is not politeness; it is the
 *    keepalive (R-EXT-10).
 *
 * ## Auth is the first message, not a header
 *
 * A browser `WebSocket` can send neither headers nor a body on connect, so the token cannot
 * ride in `Authorization`. Putting it in the query string would write it into every log
 * that records a request line. So the server gives the client five seconds to send the
 * token as the first message and closes otherwise (D23, R-BE-32).
 *
 * ## Pausing does not close the socket
 *
 * The state is announced, not enforced by disconnection. A disconnected extension cannot
 * tell "paused" from "not running", and FR-22 requires the popup to say which.
 */

import { readConnection, type ServerState, writeConnection } from "../shared/storage";

/** The canonical MV3 keepalive interval (R-EXT-10). */
export const KEEPALIVE_MS = 20_000;

/** Reconnect backoff, in milliseconds, then hold at the last value. */
export const BACKOFF_MS = [1_000, 2_000, 5_000, 15_000, 30_000];

/** What the server sends: `hello` on connect, `state` on every change. */
interface ServerFrame {
  type?: string;
  state?: ServerState;
  version?: string;
}

let socket: WebSocket | null = null;
let keepalive: ReturnType<typeof setInterval> | null = null;
let attempt = 0;

/**
 * The backoff for a given attempt, clamped to the last step.
 *
 * Exported for its test: an off-by-one here produces either a hot reconnect loop against a
 * closed port or a half-minute of dead time after the app restarts.
 */
export function backoffFor(attemptNumber: number): number {
  const index = Math.min(attemptNumber, BACKOFF_MS.length - 1);
  return BACKOFF_MS[Math.max(0, index)] ?? 30_000;
}

function teardown(): void {
  if (keepalive !== null) {
    clearInterval(keepalive);
    keepalive = null;
  }
  socket = null;
}

/**
 * Open the socket, or do nothing if one is already open.
 *
 * Idempotent because it is called from several events — install, popup open, capture — and
 * an MV3 worker that just woke has no memory of whether it was already connected.
 */
export async function connect(): Promise<void> {
  if (
    socket &&
    (socket.readyState === WebSocket.OPEN || socket.readyState === WebSocket.CONNECTING)
  ) {
    return;
  }

  const connection = await readConnection();
  if (!connection?.token) return;

  let opened: WebSocket;
  try {
    opened = new WebSocket(`ws://127.0.0.1:${connection.port}/ws`);
  } catch {
    void markStale();
    scheduleReconnect();
    return;
  }
  socket = opened;

  opened.addEventListener("open", () => {
    attempt = 0;
    // First message, within five seconds. Anything else and the server closes on us.
    opened.send(connection.token);
    keepalive = setInterval(() => {
      if (opened.readyState === WebSocket.OPEN) opened.send("ping");
    }, KEEPALIVE_MS);
  });

  opened.addEventListener("message", (event) => {
    void handleFrame(event.data);
  });

  opened.addEventListener("close", () => {
    teardown();
    // The socket dropping is the earliest and cheapest evidence that Curio is gone, and
    // until this was written down it was evidence nobody kept: the last state ever recorded
    // was `running`, so the popup went on saying so with the app closed and hid Reconnect
    // because it believed there was nothing to reconnect to.
    void markStale();
    scheduleReconnect();
  });

  opened.addEventListener("error", () => {
    // `close` follows an `error`, so the reconnect is scheduled there. Closing here too
    // would double-schedule it.
    try {
      opened.close();
    } catch {
      /* already closing */
    }
  });
}

/**
 * Record that the connection is no longer live.
 *
 * The counterpart to [`handleFrame`], and the half that was missing. A frame can only ever
 * move the state between `running` and `paused`, because both arrive *over* the socket —
 * so the one transition the server can never announce is the socket going away. Left to
 * `handleFrame` alone the stored state was write-once-optimistic: `running`, forever.
 *
 * Kept idempotent and non-destructive. The port and token stay put because they are still
 * the best guess at where Curio was, and a reconnect that finds the app still there should
 * cost nothing.
 */
async function markStale(): Promise<void> {
  const connection = await readConnection();
  if (!connection || connection.state === "stale") return;
  await writeConnection({ ...connection, state: "stale" });
}

/**
 * Record a state push.
 *
 * Persisted rather than held in a variable: the worker can die between this frame and the
 * popup opening, and a module-level global would not survive that (R-EXT-9).
 */
async function handleFrame(data: unknown): Promise<void> {
  if (typeof data !== "string") return;

  let frame: ServerFrame;
  try {
    frame = JSON.parse(data) as ServerFrame;
  } catch {
    return;
  }

  if (frame.state !== "running" && frame.state !== "paused") return;

  const connection = await readConnection();
  if (!connection) return;
  if (connection.state === frame.state) return;

  await writeConnection({ ...connection, state: frame.state });
}

function scheduleReconnect(): void {
  const delay = backoffFor(attempt);
  attempt += 1;
  // `setTimeout` in an MV3 worker does not survive termination. That is acceptable and
  // even correct: a terminated worker is woken by the next event, which reconnects.
  setTimeout(() => {
    void connect();
  }, delay);
}

/** Close deliberately, without arming a reconnect. */
export function disconnect(): void {
  const open = socket;
  teardown();
  attempt = 0;
  try {
    open?.close();
  } catch {
    /* already closed */
  }
}
