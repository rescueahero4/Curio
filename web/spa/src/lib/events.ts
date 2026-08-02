/**
 * The push channel: exactly one `EventSource` for the whole app.
 *
 * One stream, a registry of per-name handlers, and a 2-second reconnect (R-FE-7,
 * Inventory §3). "Exactly one" is a rule rather than an optimisation: a component that
 * opens its own stream doubles every event, and the duplicate arrives at a *different*
 * moment — so bugs caused by it look like race conditions rather than like duplication.
 *
 * Authentication is the session cookie, which the browser attaches automatically.
 * `EventSource` cannot send headers, and that constraint is the reason the session is a
 * cookie at all (D22).
 *
 * Two consumer-side rules that live where the data lands rather than here, noted because
 * they are easy to miss:
 *
 * - `job.updated` may be a **full row or a partial** `{id, result}` during bulk progress.
 *   Merge by id (Inventory §10.30) — a 240-item retag would otherwise re-send the whole
 *   row on every tick.
 * - `item.created` must be **ignored by the library grid while any filter or search is
 *   active** (Inventory §10.25). Prepending a card that does not match the active filter
 *   is a lie about what the user is looking at.
 */

/** The event names the server publishes (R-BE-15). */
export type EventName =
  | "item.created"
  | "item.updated"
  | "item.deleted"
  | "project.detected"
  | "project.updated"
  | "project.removed"
  | "job.updated"
  | "vocabulary.updated";

export const EVENT_NAMES: readonly EventName[] = [
  "item.created",
  "item.updated",
  "item.deleted",
  "project.detected",
  "project.updated",
  "project.removed",
  "job.updated",
  "vocabulary.updated",
] as const;

type Handler = (payload: unknown) => void;

const RECONNECT_MS = 2_000;

/** The single shared stream. */
export class EventStream {
  #source: EventSource | null = null;
  #handlers = new Map<EventName, Set<Handler>>();
  #reconnect: ReturnType<typeof setTimeout> | null = null;

  /** Subscribe to one event name. Returns the unsubscribe function. */
  on(name: EventName, handler: Handler): () => void {
    let handlers = this.#handlers.get(name);
    if (!handlers) {
      handlers = new Set();
      this.#handlers.set(name, handlers);
    }
    handlers.add(handler);
    return () => {
      handlers?.delete(handler);
    };
  }

  /** Open the stream. Idempotent. */
  connect(): void {
    if (this.#source) return;

    const source = new EventSource("/api/events");
    this.#source = source;

    for (const name of EVENT_NAMES) {
      source.addEventListener(name, (event) => {
        const handlers = this.#handlers.get(name);
        if (!handlers?.size) return;

        let payload: unknown;
        try {
          payload = JSON.parse((event as MessageEvent<string>).data);
        } catch {
          // A malformed frame must not take the stream down with it: the next event is
          // probably fine, and a dead stream is a silently stale dashboard.
          console.error(`curio: could not parse a ${name} event`);
          return;
        }
        for (const handler of handlers) handler(payload);
      });
    }

    // `hello` on connect and `ping` every 20 s are stream-level keepalives. They are
    // tolerated silently — they carry nothing and exist to keep the connection warm.
    source.addEventListener("error", () => {
      this.disconnect();
      this.#reconnect = setTimeout(() => this.connect(), RECONNECT_MS);
    });
  }

  disconnect(): void {
    this.#source?.close();
    this.#source = null;
    if (this.#reconnect) {
      clearTimeout(this.#reconnect);
      this.#reconnect = null;
    }
  }
}

/** The app's one stream (R-FE-7). Components subscribe; they never open their own. */
export const events = new EventStream();
