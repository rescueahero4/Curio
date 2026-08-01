/**
 * Finding Curio, and staying authenticated to it.
 *
 * ## Why there is a ladder at all
 *
 * The app binds an ephemeral port and publishes the truth in `runtime.json` (D10). A
 * browser extension cannot read files, so native messaging is the only channel through
 * which it can learn the port — and the token rides the same reply, which is what makes a
 * fresh install capture with zero configuration (R-EXT-4).
 *
 * Everything below `connectNative` is for installs where that channel does not exist: an
 * unpacked dev install, or a user who declined the installer's registration step. The probe
 * survives because it costs three small code paths and saves a support thread — but note
 * what it can and cannot do. **An ephemeral port is undiscoverable by design**, so probing
 * only ever finds a port someone deliberately pinned (D10, D11, R-EXT-8).
 *
 * ## 401 is not a pairing failure
 *
 * The token is per-run: quitting Curio kills it and starting it mints a new one (D21). So a
 * 401 means "the app restarted", not "the user did something wrong" — the fix is to
 * re-handshake **once** and retry, and only surface anything if that also fails
 * (R-EXT-18a). The old "Pairing token rejected" copy retired along with the pairing token.
 */

import {
  type Connection,
  clearConnection,
  readConnection,
  type ServerState,
  writeConnection,
} from "../shared/storage";

/** The native-messaging host's registered name. Matches the manifest `curio-nmh` writes. */
export const NM_HOST = "com.curio.nmh";

/** The legacy probe range, kept for pinned-port installs only (R-EXT-8). */
export const PROBE_PORTS = [4321, 4322, 4323, 4324, 4325, 4326, 4327, 4328, 4329, 4330, 4331];

/** Long enough for a loopback round trip, short enough that eleven of them are bearable. */
export const PROBE_TIMEOUT_MS = 800;

/** What the native-messaging host replies with. */
interface HostReply {
  port?: number;
  token?: string;
  state?: ServerState;
}

/**
 * Ask the native-messaging host where Curio is.
 *
 * One connect, one reply, then the host exits (R-EXT-4). Resolves `null` rather than
 * throwing when the host is not registered — that is the ordinary state of an unpacked
 * install, not an error worth propagating.
 */
export function askNativeHost(): Promise<Connection | null> {
  return new Promise((resolve) => {
    let settled = false;
    const finish = (value: Connection | null) => {
      if (settled) return;
      settled = true;
      resolve(value);
    };

    let port: chrome.runtime.Port;
    try {
      port = chrome.runtime.connectNative(NM_HOST);
    } catch {
      // No registration. The ladder's next rung handles it.
      finish(null);
      return;
    }

    port.onMessage.addListener((reply: HostReply) => {
      // `stale` means Curio is installed but not running — a real answer, and not one we
      // can act on beyond telling the user (FR-22: never launch it ourselves).
      if (typeof reply?.port === "number" && typeof reply?.token === "string") {
        finish({ port: reply.port, token: reply.token, state: reply.state ?? "running" });
      } else {
        finish(null);
      }
      port.disconnect();
    });

    // Fires when the host is absent, crashes, or exits after replying. Harmless in the
    // last case because `finish` is idempotent.
    port.onDisconnect.addListener(() => finish(null));

    // The host answers in milliseconds by design (R-EXT-3). A hang means something is
    // wrong with it, and waiting forever would leave the popup on "Checking…".
    setTimeout(() => finish(null), 2_000);

    port.postMessage({ type: "locate" });
  });
}

/**
 * Ask one port whether Curio is listening.
 *
 * `/health` is unauthenticated and cross-origin readable **by design**, precisely so this
 * check can exist without a token (Inventory §10.29).
 */
export async function probePort(port: number): Promise<Connection | null> {
  const controller = new AbortController();
  const timer = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);

  try {
    const response = await fetch(`http://127.0.0.1:${port}/health`, {
      signal: controller.signal,
    });
    if (!response.ok) return null;

    const health = (await response.json()) as { status?: string; port?: number };
    if (health?.status !== "ok") return null;

    // A port with no token: found, but not yet usable. The `/pair` handoff or a manual
    // paste supplies the token (R-EXT-8b), and the popup says so.
    return { port: health.port ?? port, token: "", state: "running" };
  } catch {
    return null;
  } finally {
    clearTimeout(timer);
  }
}

/**
 * Walk the ladder: native messaging, then the stored record, then the probe.
 *
 * The fallback rungs run **only** when `connectNative` fails (R-EXT-8). A successful
 * handshake must not be second-guessed by a probe that might find a different instance.
 */
export async function bootstrap(): Promise<Connection | null> {
  const native = await askNativeHost();
  if (native) {
    await writeConnection(native);
    return native;
  }

  // The stored record is tried before probing because it carries a token, which a probe
  // cannot produce.
  const stored = await readConnection();
  if (stored?.token) {
    const alive = await probePort(stored.port);
    if (alive) return stored;
  }

  for (const candidate of PROBE_PORTS) {
    const found = await probePort(candidate);
    if (found) {
      // Keep any token we already hold: the probe found the address, not the credential.
      const merged: Connection = { ...found, token: stored?.token ?? "" };
      await writeConnection(merged);
      return merged;
    }
  }

  return null;
}

/** The stored connection, bootstrapping one if there is none. */
export async function ensureConnection(): Promise<Connection | null> {
  return (await readConnection()) ?? (await bootstrap());
}

/** Thrown when the ladder ran out of rungs. The popup renders this as the gray state. */
export class NotRunningError extends Error {
  constructor() {
    super("Can't reach Curio — open it and try again");
    this.name = "NotRunningError";
  }
}

/** Thrown when Curio is up but refusing writes (R-EXT-11). */
export class PausedError extends Error {
  constructor() {
    super("Curio is paused");
    this.name = "PausedError";
  }
}

/**
 * Call Curio, re-handshaking once if the token turns out to be stale.
 *
 * The retry is deliberately **once**. A loop would turn "the app is gone" into an
 * indefinite spin, and the second failure is genuine information: the ladder ran and still
 * could not produce a working token.
 */
export async function authedFetch(
  path: string,
  init: RequestInit = {},
  onRetry?: () => void,
): Promise<Response> {
  const connection = await ensureConnection();
  if (!connection) throw new NotRunningError();

  const send = (using: Connection) =>
    fetch(`http://127.0.0.1:${using.port}${path}`, {
      ...init,
      headers: { ...(init.headers ?? {}), Authorization: `Bearer ${using.token}` },
    });

  let response = await send(connection);
  if (response.status !== 401) return response;

  // R-EXT-18a. Not a pairing failure — the app restarted and minted a new token (D21).
  onRetry?.();
  await clearConnection();
  const fresh = await bootstrap();
  if (!fresh?.token) throw new NotRunningError();

  response = await send(fresh);
  if (response.status === 401) throw new NotRunningError();
  return response;
}
