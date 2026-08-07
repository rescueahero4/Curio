/**
 * Projects strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/projects.ts` is
 * type-checked against `Projects`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const projects = {
  title: "Projects",

  /** The one sentence that has to land: entries arrive here on their own (FR-17). */
  subtitle: "Folders your AI tools wrote to the projects root. Detected automatically.",

  /**
   * Said by every control that a pause disables, on this screen and only this screen —
   * Settings has its own, because a paused Settings is refusing a different thing.
   */
  paused: "Curio is paused. Resume from the tray icon.",

  /**
   * Two forms because English has two. Japanese has one sentence for every number, so the
   * `one` and `other` values there are identical and that is correct, not an oversight.
   */
  count: {
    one: template<{ count: number }>("{{ count }} project"),
    other: template<{ count: number }>("{{ count }} projects"),
  },

  failed: "Curio could not read your projects.",
  /** Not `common.retry`: this asks for the whole page again, not for one failed write. */
  retry: "Try again",
  loading: "Looking for your projects…",

  empty: {
    title: "No projects yet.",
    /**
     * The empty state teaches the prompt-first route, because that is the one the app is
     * built around: compose a brief, send it to Claude, and the folder it writes is adopted
     * within about five seconds. Registering by hand is the exception and is offered below
     * the list, not here — an empty page is exactly where a user is most likely to mistake
     * the exception for the intended path.
     */
    body: "Start a prompt, paste it into your AI tool, and point it at your projects root. The folder it writes shows up here within about five seconds — there is no import step. Check which folder Curio is watching in Settings.",
    settings: "Open Settings",
  },

  register: {
    lead: "Somewhere else on this machine?",
    open: "Register a folder by hand",

    title: "Register a folder",
    blurb:
      "Point Curio at a folder anywhere on this machine. It has to exist already — Curio checks the path rather than creating it. Folders added this way are not followed if you later rename them; only folders Curio finds itself carry a marker file that survives a rename.",
    path: "Folder path",
    name: "Name",
    namePlaceholder: "Optional — defaults to the folder's own name",
    submit: "Register",
    saving: "Registering…",
    needPath: "Enter a folder path first.",
    failed: "Curio could not register that folder.",
  },

  card: {
    /**
     * The tile is the launch button, so it carries both the label a reader sees and the
     * reason it is disabled. Every state has its own sentence: "nothing happened" with no
     * explanation is the failure PRD §5 is written against.
     */
    launch: {
      label: "Launch ↗",
      opening: "Opening…",
      title: "Launch in a new tab",
      missingLabel: "Folder missing",
      missingTitle: "The folder is gone, so there is nothing to serve.",
      noPageLabel: "No page to launch",
      noPage:
        "There is no index.html here, or in a v1/v2/… subfolder, so there is no page to launch. Open the folder to see what the tool actually wrote.",
      blocked: template<{ url: string }>(
        "Your browser blocked the new tab. The project is at {{ url }}.",
      ),
      failed: "Curio could not open that project.",
    },

    /** The tooltip on a path that is itself the button, so it names where it goes. */
    reveal: template<{ path: string }>("Open {{ path }} in your file manager"),
    /**
     * Word for word what `POST /api/system/reveal` answers on its ordinary success, said
     * here instead so it has a Japanese counterpart. Phrased as a request, like the server's
     * (§10.22): nothing on either side can know whether a file manager actually opened.
     */
    revealAsked: template<{ path: string }>("Asked your file manager to open {{ path }}."),
    revealFailed: "Curio could not ask your file manager to open.",

    detected: template<{ when: string }>("detected {{ when }}"),
    opened: template<{ when: string }>("opened {{ when }}"),

    origin: {
      mcp: "added by an agent",
      manual: "added by you",
    },

    missing: {
      badge: "missing",
      /**
       * FR-19. Curio still never removes a missing folder's record on its own — the record
       * carries the prompt link, and nothing left on disk can rebuild it. What changed is
       * who gets to decide: the card says the short version and offers the two ways out,
       * rather than spending a paragraph arguing for a record the user may have meant to be
       * rid of.
       */
      title: "Can't find project folder",
      locate: "Locate",
      locateTitle: "Open the closest folder that is there",
      remove: "Remove",
      removeTitle: "Forget this project. The folder is not touched.",
      removeFailed: "Curio could not remove that project.",

      confirm: "Remove this project for good?",
      cost: "Its prompt link goes with it.",
      removing: "Removing…",
      keep: "Keep",
    },
  },

  prompt: {
    from: template<{ title: string }>('From "{{ title }}"'),
    untitled: "Untitled prompt",
    unlink: "Unlink this prompt",
    deleted: "Prompt deleted",
    clear: "Clear",
    failed: "Curio could not change the prompt link.",
  },
} as const;

export type Projects = Translated<typeof projects>;
