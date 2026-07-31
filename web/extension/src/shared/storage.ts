/**
 * Durable state — and there is very little of it, on purpose.
 *
 * MV3 kills the service worker after roughly 30 seconds idle, so **assume the worker dies
 * at any moment** (R-EXT-9). No module-level variable survives an event, which means
 * anything that must outlive one belongs here and nowhere else.
 *
 * Exactly three keys: `port`, `token`, `state`.
 *
 * **Capture mode is deliberately not stored.** A popup restored from a previous session
 * would otherwise show a stale mode and capture something the user did not ask for; so the
 * mode is passed with each request and anything that is not exactly `"full"` resolves to
 * fold (R-EXT-12, Inventory §7). Absent state means the safe state.
 */

/** Whether the app is accepting captures (R-EXT-11). */
export type ServerState = "running" | "paused" | "stale";

export interface Connection {
  port: number;
  token: string;
  state: ServerState;
}

const KEYS = ["port", "token", "state"] as const;

export async function readConnection(): Promise<Connection | null> {
  const stored = await chrome.storage.local.get(KEYS as unknown as string[]);
  const { port, token, state } = stored as Partial<Connection>;
  if (typeof port !== "number" || typeof token !== "string") return null;
  return { port, token, state: state ?? "running" };
}

export async function writeConnection(connection: Connection): Promise<void> {
  await chrome.storage.local.set(connection);
}

/**
 * Forget the connection.
 *
 * Called when the app is gone or its token stopped working. The token is **per-run**: a
 * restart mints a new one, so a held token is stale rather than wrong, and a 401 means
 * "re-bootstrap", never "tell the user pairing failed" (D21, R-EXT-18a).
 */
export async function clearConnection(): Promise<void> {
  await chrome.storage.local.remove(KEYS as unknown as string[]);
}
