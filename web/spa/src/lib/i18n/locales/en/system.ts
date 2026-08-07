/**
 * System strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/system.ts` is
 * type-checked against `System`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * Everything here is read by someone who is already stuck — a tab with no session, a URL
 * that does not exist, a request that came back refused. Two consequences for the copy:
 *
 * - **Say what happened, then what to do.** The `noSession` and `pair` screens each end on
 *   an action the reader can take — the state first, the way out last. Individual strings
 *   are free to be pure description; the screen is not. An apology is not an action.
 * - **Not every unusual state is an error.** `noSession` is the ordinary outcome of
 *   bookmarking the dashboard (R-FE-6a); its words are deliberately flat.
 *
 * `request` holds the sentences `lib/http.ts` writes for itself, and only those. The
 * messages the server sends back in `{error, message}` arrive already-worded and are not
 * translatable from here.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const system = {
  /* -- a tab with no session (R-FE-6a) ----------------------------------------------- */
  noSession: {
    title: "Open Curio from the tray",
    body: "This tab does not have a session. Curio's dashboard is opened from the tray icon, which hands the browser a one-time key — so a bookmarked link cannot get in on its own.",
    waiting: "Waiting for Curio to start…",
    /**
     * "Open Dashboard" is quoted rather than translated: it is the literal label on the
     * tray menu, which is built in `curio-tray` and ships in English on every locale
     * (D14). Naming it in Japanese would send the reader looking for a menu entry that is
     * not there.
     */
    running: "Curio is running. Choose “Open Dashboard” from its tray menu.",
  },

  /* -- the client-side 404 (R-FE-2) --------------------------------------------------- */
  notFound: {
    title: "Not found",
    back: "Back to the library",
  },

  /* -- the pairing fallback (R-FE-19, D11) -------------------------------------------- */
  pair: {
    title: "Authorize this browser",
    blurb:
      "The Curio extension normally connects on its own. Use this only if it cannot — typically a development install loaded unpacked.",
    /**
     * Repeats the heading on purpose. This page has exactly one action and a paragraph
     * explaining when to take it, and a two-word button under that heading reads as the
     * way *out* of the page rather than the point of it. The repetition is weight, not an
     * oversight — both languages want a full-width primary action here.
     */
    authorize: "Authorize this browser",
    done: "Authorized. The extension picks this up automatically — you can close this tab.",
    refused: "Curio refused the request. Open the dashboard from the tray and retry.",
    unreachable: "Could not reach Curio.",
  },

  /* -- what the transport says when a request does not land --------------------------- */
  request: {
    /** No response at all: the fetch itself threw, so there is no status to report. */
    unreachable: "Curio is not answering.",
    /** A 503 on a mutating route. Soft-disabled, not broken (D25, R-FE-8). */
    paused: "Curio is paused.",
    /**
     * The last resort, when the server refused without a sentence of its own. The status
     * is in the string because it is the only thing anyone can act on — it is what turns
     * "it broke" into something reportable. `HTTP` is spelled out rather than left implicit
     * in the parentheses: a bare three-digit number is a status code only to a reader who
     * already knows to read it as one.
     */
    failed: template<{ status: number }>("Request failed (HTTP {{ status }})."),
  },
} as const;

export type System = Translated<typeof system>;
