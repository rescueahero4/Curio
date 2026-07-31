/**
 * Session bootstrap: nonce in, cookie out.
 *
 * The tray opens the dashboard at `/?t=<nonce>`. The nonce is single-use, lives 30
 * seconds, and authorizes exactly one thing — an exchange for an HttpOnly,
 * `SameSite=Strict`, session-scoped cookie (R-FE-6, R-SEC-5, D22).
 *
 * Three consequences worth stating, because each one removes a failure mode the previous
 * design had:
 *
 * - **The token never reaches this code.** Not in JavaScript, not in localStorage, not in
 *   a bookmarkable URL. An XSS bug in the dashboard cannot exfiltrate a credential that
 *   the dashboard never holds.
 * - **Reload survives.** The cookie authenticates every subsequent request including the
 *   SSE stream, so F5 is a non-event. `EventSource` cannot send headers, which is the
 *   specific reason this is a cookie and not a bearer header.
 * - **A bookmarked tab is not an error.** Arriving with no session and no nonce renders a
 *   quiet "Open Curio from the tray" screen that retries `/health` — never an error page
 *   (R-FE-6a).
 */

/** Where the session bootstrap ended up. */
export type SessionState =
  | { kind: "authenticated" }
  /** No cookie and no nonce: a bookmarked tab, or one restored after a restart. */
  | { kind: "no-session" }
  /** The server is not answering. Distinct from no-session: the app may be starting. */
  | { kind: "unreachable" };

const NONCE_PARAM = "t";

/**
 * Exchange a launch nonce for a session, if one is present in the URL.
 *
 * Strips the nonce from the address bar with `history.replaceState` whether or not the
 * exchange succeeded — a consumed nonce is useless, and one left in the URL survives into
 * browser history and into whatever the user pastes when asking for help.
 */
export async function bootstrapSession(): Promise<SessionState> {
  const url = new URL(window.location.href);
  const nonce = url.searchParams.get(NONCE_PARAM);

  if (nonce) {
    url.searchParams.delete(NONCE_PARAM);
    window.history.replaceState({}, "", url.pathname + url.search + url.hash);

    try {
      // Same-origin: relative URL only. The SPA must never assume an origin, because the
      // port changes every run (R-FE-4).
      const response = await fetch("/api/auth/exchange", {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ nonce }),
      });
      if (response.ok) return { kind: "authenticated" };
    } catch {
      return { kind: "unreachable" };
    }
  }

  return probeSession();
}

/**
 * Ask whether the cookie we already hold still works.
 *
 * A 401 means the app restarted: the token is per-run, so a session minted by a previous
 * run authenticates nothing (D21). The recovery is the same as never having had one —
 * the user opens the dashboard from the tray again.
 */
export async function probeSession(): Promise<SessionState> {
  try {
    const response = await fetch("/api/auth/session", { method: "GET" });
    if (response.ok) return { kind: "authenticated" };
    if (response.status === 401) return { kind: "no-session" };
    return { kind: "unreachable" };
  } catch {
    return { kind: "unreachable" };
  }
}

/**
 * Whether the server is up at all.
 *
 * `/health` is unauthenticated by contract (R-SEC-11), which is exactly what makes it
 * usable from the no-session screen: that screen has no credential and must not present
 * an error while the app is merely starting.
 */
export async function serverIsUp(): Promise<boolean> {
  try {
    const response = await fetch("/health");
    return response.ok;
  } catch {
    return false;
  }
}
