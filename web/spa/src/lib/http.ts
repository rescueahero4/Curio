/**
 * The one place a request leaves the dashboard.
 *
 * Three rules are enforced here rather than trusted to thirty call sites:
 *
 * - **Relative URLs, no Authorization header.** The server listens on an ephemeral port
 *   that changes every run, so a hardcoded origin is a bug that only appears on the second
 *   launch (R-FE-4). The session is an HttpOnly cookie the browser attaches by itself; this
 *   code never holds a credential and so cannot leak one (D22).
 * - **Three statuses are not failures**, and flattening them into one generic error is
 *   precisely how an app starts lying to its user: `503` on a mutation is *paused*
 *   (D25, R-FE-8), `409` may carry the server's `matched` and `limit` and must be reported
 *   with both numbers intact (R-FE-11), and `401` means the run this session belonged to
 *   has ended (D21).
 * - **Paused is observed, not polled.** `/health` deliberately reports `status: "ok"` while
 *   paused, because it is unauthenticated and the pause state is not for any origin that
 *   can reach loopback. So the only honest signal is a mutation being refused — which is
 *   exactly what this module watches for.
 */

import { createSignal } from "solid-js";
import type { OverCap } from "~/lib/types";

const MUTATING_METHODS = new Set(["POST", "PUT", "PATCH", "DELETE"]);

const [pausedSignal, setPaused] = createSignal(false);

/** Whether the server is refusing mutations. Drives the banner, and nothing else. */
export const paused = pausedSignal;

/** Every failed request the dashboard makes, with the status still attached. */
export class ApiError extends Error {
  readonly status: number;
  readonly body: unknown;
  /** A 503 on a mutating route: soft-disabled, not broken (R-FE-8). */
  readonly isPaused: boolean;

  constructor(status: number, body: unknown, message: string, isPaused: boolean) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.body = body;
    this.isPaused = isPaused;
  }

  /** The app restarted; the session belonged to the previous run (D21). */
  get sessionExpired(): boolean {
    return this.status === 401;
  }

  /** The server is unreachable, as distinct from answering with a refusal. */
  get unreachable(): boolean {
    return this.status === 0;
  }

  /**
   * The cap refusal, with both of the server's numbers.
   *
   * Callers surface `matched` and `limit` together — "1,204 items match; the cap is 500"
   * is a decision the user can act on, where "too many items" is a dead end (R-FE-11).
   */
  get overCap(): OverCap | null {
    if (this.status !== 409) return null;
    const body = this.body as Partial<OverCap> | null;
    if (typeof body?.matched !== "number" || typeof body?.limit !== "number") return null;
    return { matched: body.matched, limit: body.limit };
  }
}

/**
 * Issue one request.
 *
 * `path` is always relative and always begins with `/`. There is no base-URL option on
 * purpose: adding one is the first step towards assuming an origin.
 */
export async function request<T>(path: string, init: RequestInit = {}): Promise<T> {
  const method = (init.method ?? "GET").toUpperCase();
  const mutating = MUTATING_METHODS.has(method);

  let response: Response;
  try {
    response = await fetch(path, init);
  } catch {
    throw new ApiError(0, null, "Curio is not answering.", false);
  }

  if (response.ok) {
    // A mutation that lands proves the server is taking writes again, so the banner comes
    // down on the same evidence that put it up — no extra round trip to ask.
    if (mutating) setPaused(false);
    return (await readBody(response)) as T;
  }

  const body = await readBody(response);
  const isPaused = mutating && response.status === 503;
  if (isPaused) setPaused(true);

  throw new ApiError(response.status, body, describe(response, body, isPaused), isPaused);
}

export function get<T>(path: string): Promise<T> {
  return request<T>(path);
}

export function post<T>(path: string, body?: unknown): Promise<T> {
  return sendJson<T>("POST", path, body);
}

export function patch<T>(path: string, body?: unknown): Promise<T> {
  return sendJson<T>("PATCH", path, body);
}

export function put<T>(path: string, body?: unknown): Promise<T> {
  return sendJson<T>("PUT", path, body);
}

export function del<T>(path: string): Promise<T> {
  return request<T>(path, { method: "DELETE" });
}

/** Multipart, for the one endpoint that carries a file. */
export function postForm<T>(path: string, form: FormData): Promise<T> {
  // No `content-type` header: the browser has to set it, because only it knows the
  // multipart boundary it generated.
  return request<T>(path, { method: "POST", body: form });
}

function sendJson<T>(method: string, path: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method,
    headers: body === undefined ? undefined : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
}

async function readBody(response: Response): Promise<unknown> {
  if (response.status === 204) return null;
  const text = await response.text();
  if (!text) return null;
  try {
    return JSON.parse(text);
  } catch {
    return text;
  }
}

function describe(response: Response, body: unknown, isPaused: boolean): string {
  if (isPaused) return "Curio is paused.";

  // `message` first, `error` only as a fallback. The server sends both: `error` is a
  // machine code the caller branches on (`conflict`, `over_cap`), `message` is the
  // sentence written for the person reading the screen — and it is the one that says
  // "merge into it instead" rather than the bare word "conflict".
  const detail = (body as { error?: unknown; message?: unknown } | null) ?? null;
  const message = detail?.message ?? detail?.error;
  if (typeof message === "string" && message) return message;

  return response.statusText || `Request failed (${response.status}).`;
}
