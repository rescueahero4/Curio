/**
 * Shell strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/shell.ts` is
 * type-checked against `Shell`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * The shell is the one namespace every route pays for, so it is also the one place where a
 * string is read by someone who has not chosen to be here — the paused strip and the
 * missing-key strip both interrupt whatever the reader came for. They are written as whole
 * sentences for that reason: a banner that has to be assembled out of fragments is a banner
 * that reads differently in the two languages, and these two are the app admitting a
 * limitation. Neither can afford to sound machine-made.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const shell = {
  /**
   * The mark is a link home, and the only thing naming it is the artwork. The accessible
   * name says both what it is and where it goes, because "Curio" alone would announce a
   * logo rather than a destination.
   */
  brand: {
    home: "Curio — Library",
  },

  /* -- the three top-level destinations --------------------------------------------- */
  nav: {
    label: "Sections",
    library: "Library",
    projects: "Projects",
    prompts: "Prompts",
  },

  /**
   * One key for the field's name and its placeholder. They are the same words on purpose —
   * the label is `sr-only`, so the placeholder is what a sighted reader sees the field
   * called, and letting them drift would give the two audiences different names for it.
   */
  search: {
    label: "Search the library",
  },

  /* -- the right-hand controls ------------------------------------------------------ */
  actions: {
    settings: "Settings",
    /** The glyph belongs to the label: where a Japanese button puts its `+` is its own call. */
    addItem: "+ Add Item",
    newProject: "New Project",
    newProjectStarting: "Starting…",
    /** The last-resort message when the failure arrives without one of its own. */
    newProjectFailed: "A new prompt could not be started.",
    /**
     * Why the two action buttons are disabled. It sits on `title`, so it has to work as a
     * sentence read on its own, with no button label in front of it.
     */
    pausedReason: "Curio is paused. Resume from the tray icon.",
  },

  /**
   * The paused strip (R-FE-8, D25). It states the condition, then what still works — that
   * order matters, because a reader who stops after the first clause should already know
   * their library has not gone anywhere.
   */
  paused: {
    title: "Curio is paused.",
    body: "Browsing, search and everything already captured stay available. New captures and edits are refused until you choose Resume from the tray icon.",
  },

  /**
   * The missing-key strip (FR-26). Four separate lines rather than one sentence, because
   * the second is conditional and the third is a link: each has to stand on its own in both
   * languages, and a clause that only makes sense after the clause before it cannot.
   *
   * The three "wait" words here — queued, waits, waiting — are deliberately three different
   * words. Repeating one of them three times in four lines is what the Japanese has to
   * avoid too, by its own means.
   */
  missingKey: {
    title: "Queued — needs an API key.",
    body: "Captures still land and stay browsable; Curio waits to describe them.",
    waiting: template<{ count: number }>("{{ count }} waiting."),
    /**
     * The link text, and the one line here that carries no terminal punctuation. The whole
     * string is the link body, so a full stop inside it would be underlined — and a
     * Japanese 。 sits in the left half of a full-width box, which draws the underline out
     * past the last character into open space.
     */
    addKey: "Add a key in Settings",
    drains: "The queue drains on its own once you do.",
  },

  /**
   * The save badge's default. A caller with something more exact to say passes its own
   * label and owns that string; this is what the badge says when nobody does.
   */
  saved: {
    label: "Saved changes",
    undo: "Undo",
  },
} as const;

export type Shell = Translated<typeof shell>;
