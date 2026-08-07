/**
 * Settings strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/settings.ts` is
 * type-checked against `Settings`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * The nesting mirrors the page: one object per section, in the order the sections render,
 * with the page-level furniture — the heading, the two states the fetch can be in, the
 * table of contents — above them. `paused` and `save` sit at the top because they belong to
 * no section and every section shows them.
 *
 * The language section is deliberately absent. It ships with the i18n core and its strings
 * live in `common.language`, because they are read by someone who has not yet chosen a
 * language they can read.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const settings = {
  title: "Settings",
  loading: "Reading your settings…",
  unreadable: {
    message: "Curio could not read your settings.",
    /** Not `common.retry`: this asks for the whole page again, not for one failed write. */
    retry: "Try again",
  },

  /* -- the table of contents, and the two facts at the foot of it --------------------- */
  nav: {
    /** The `aria-label` on the nav element; there is no visible heading above the list. */
    label: "Settings sections",
    groups: {
      general: "General",
      assessment: "Assessment",
      agents: "Agents",
    },
    /**
     * The only nav entry that is not its section's own heading. "Anthropic API key" is
     * three words too many for a 176px column, and the section it points at says the rest.
     */
    apiKey: "Anthropic key",
    port: template<{ port: number }>("Port {{ port }}"),
    /** Why the port above is a different number every run. */
    ephemeralPort: "chosen at launch",
  },

  /* -- what a control says when Curio is paused (R-FE-8) ----------------------------- */
  paused: {
    /** On the control itself, before anything is attempted: a `title`, or a toggle's hint. */
    reason: "Curio is paused. Resume from the tray icon to change settings.",
    /** After a write was refused. Past tense, because by then something has happened. */
    notSaved: "Curio is paused, so this did not save. Resume from the tray icon and try again.",
  },

  save: {
    /** The badge's label after an undo. "Saved changes" would be describing the wrong write. */
    reverted: "Put back",
    /** The fallback when the failure is not something the server explained. */
    failed: "Curio could not save that.",
  },

  paths: {
    title: "Paths",
    blurb: "Where your library lives, and where Curio looks for new projects.",
    dataRoot: {
      label: "Data root",
      hint: "Your library, notes and prompts all live here. This can't be moved from inside the app.",
    },
    projectsRoot: {
      /** "(watched)" is load-bearing — Curio reads this folder, it never writes into it. */
      label: "Projects root (watched)",
      hint: "Curio watches this folder. Add a folder inside it and it becomes a project within a few seconds. Press Enter to save. The folder has to exist already, and Curio needs a restart to start watching a new one.",
    },
  },

  startup: {
    title: "Startup",
    blurb: "Have Curio up and running as soon as you turn on your computer.",
    toggle: "Start Curio when I log in",
    /**
     * Two whole sentences rather than one sentence with an optional middle. The server
     * sometimes explains why and sometimes does not, and a clause spliced in behind a colon
     * only reads as English — in Japanese the explanation lands somewhere else entirely.
     *
     * `reason` arrives from the server already written as a full sentence, and stays in the
     * language the server wrote it in.
     */
    unsupported:
      "Curio can't set this up on your system. You can still add Curio to your computer's startup items yourself.",
    unsupportedReason: template<{ reason: string }>(
      "Curio can't set this up on your system: {{ reason }} You can still add Curio to your computer's startup items yourself.",
    ),
  },

  apiKey: {
    title: "Anthropic API key",
    blurb:
      "Curio needs this to review what you capture. Without it, saving still works — reviews just wait until you add one.",
    none: "No key is set.",
    /** The masked key the server sends back — `sk-ant-…`, never the key itself. */
    set: template<{ key: string }>("A key is set: {{ key }}"),
    replace: "Replace the key",
    add: "Set a key",
    hint: "Press Enter to save. Your key is stored securely and never shown again, so replacing it can't be undone.",
    clear: "Clear key",
    clearing: "Clearing…",
    /** Why the button is unavailable when nothing is paused. */
    nothingToClear: "There is no key to clear.",
    cleared: "Key cleared. Reviews will wait until you add a new one.",
    clearPaused: "Curio is paused, so the key was not cleared. Resume from the tray icon.",
    clearFailed: "Curio could not clear the key.",
  },

  models: {
    title: "Models",
    blurb:
      "Which AI models Curio uses. Names aren't checked as you type, so a typo shows up later as a review that failed.",
    vision: {
      label: "Vision",
      hint: "Looks at your screenshots and writes the review.",
    },
    utility: {
      label: "Utility",
      hint: "Handles the smaller background jobs.",
    },
  },

  rubric: {
    title: "Assessment rubric",
    blurb:
      "What you want Curio to look for in everything you capture. Edit it however you like — your changes are kept through updates.",
    open: "Open the rubric",
    /** Curio asks the OS to open the file; it never learns whether an editor appeared. */
    opening: "Asking…",
    paused: "Curio is paused, so it did not ask your editor to open. Resume from the tray icon.",
    failed: "Curio could not reach your editor.",
  },

  thresholds: {
    title: "Confidence thresholds",
    blurb:
      "How sure Curio has to be before filing something on its own. When it isn't sure enough, it asks you instead.",
    lower: {
      label: "Lower",
      hint: "Below this, Curio suggests a new family rather than guessing.",
    },
    upper: {
      label: "Upper",
      hint: "At or above this, Curio files the item without asking.",
    },
  },

  mcp: {
    title: "MCP server",
    blurb:
      "Lets AI tools like Claude search your library and read your saved items. Off means off — nothing can reach it.",
    toggle: "Let agents reach this library over MCP",
    off: "The toggle is off. These will still set Curio up, but nothing will work until you turn it on.",
    claudeCode: {
      label: "Claude Code",
      hint: "Paste this into a terminal, or into Claude Code itself. Curio needs to be running when Claude connects. If you ever move or reinstall Curio, come back and run this again.",
    },
    claudeCodeHttp: {
      label: "Claude Code, over HTTP",
      hint: "The same thing, connected directly. Only worth using if you've set a fixed port — otherwise this stops working the next time Curio restarts.",
    },
    claudeDesktop: {
      label: "Claude Desktop",
      hint: "Paste this into claude_desktop_config.json. Claude Desktop has no command that can do it for you. The toggle above still applies.",
    },
  },

  /** The copy control on a config block. `label` is the client's name, above the snippet. */
  snippet: {
    copy: template<{ label: string }>("Copy the {{ label }} config"),
    copied: template<{ label: string }>("{{ label }} config copied"),
    refused: "Your browser would not let Curio use the clipboard. Select the text and copy it.",
  },
} as const;

export type Settings = Translated<typeof settings>;
