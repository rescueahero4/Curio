/**
 * Items strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/items.ts` is
 * type-checked against `Items`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * Two surfaces live here — the single-item page (`detail`) and the manual add dialog
 * (`add`) — plus the handful of connection failures both of them can hit (`errors`). Those
 * are shared rather than duplicated because they say the same thing about the same thing:
 * Curio itself, not the item you were reaching for. They are not in `common` because no
 * other namespace needs them, and `common` is not a lost-and-found.
 */

import type { Translated } from "~/lib/i18n/translated";

export const items = {
  /* -- the single-item page --------------------------------------------------------- */
  detail: {
    /** The arrow stays in the markup; only the destination is a word. */
    back: "Library",
    /** Badge on an item that is still queued, next to a live dot. */
    processing: "Assessment in progress",
    /** Shown when a delete arrives over the event stream while the page is open. */
    gone: {
      message: "This item was deleted.",
      back: "Back to the library",
    },
    errors: {
      open: "Could not open that item.",
      notFound: "That item is not in the library. It may have been deleted.",
    },
  },

  /* -- the manual add dialog -------------------------------------------------------- */
  add: {
    /** Both the heading and the dialog's accessible name — they name the same thing. */
    title: "Add an item",
    blurb: "Drop an image, paste one, or choose a file. Curio describes it once it lands.",
    /** The backdrop, which is a way out of the dialog and has to say so out loud. */
    close: "Close without adding",
    dropzone: "Drop an image here, paste, or choose a file",
    sourceUrl: "Source URL (optional)",
    name: "Title (optional — Curio suggests one)",
    /**
     * Why the submit button is off. PRD §5: a disabled control that does not explain
     * itself is indistinguishable from a broken one.
     */
    blocked: {
      paused: "Curio is paused. Resume from the tray icon to add items.",
      noFile: "Add a screenshot first — every item needs one.",
    },
    busy: "Adding…",
    submit: "Add item",
    errors: {
      create: "Could not add the item.",
      paused: "Curio is paused. Resume from the tray icon and try again.",
      /**
       * A 413 off the body limit in `routes/mod.rs`, which is sized for a very tall
       * full-page stitch — real captures do reach it. Axum rejects those before any
       * handler runs and answers in plain text, so there is no JSON message to show and
       * the generic path would surface the raw reason phrase, "Payload Too Large".
       */
      tooLarge: "That screenshot is too big to add.",
    },
  },

  /* -- Curio is not there, on either surface ---------------------------------------- */
  errors: {
    sessionExpired: "Curio restarted. Open the dashboard from the tray again.",
    unreachable: "Curio is not answering. Is it still running?",
  },
} as const;

export type Items = Translated<typeof items>;
