/**
 * Editor strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/editor.ts` is
 * type-checked against `Editor`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * The namespace is laid out as the screen is: the list, then the document, then the rail
 * above it, then the two things that happen to a whole document, then the `/` menu that
 * writes into it.
 *
 * `slash.kinds` and `chip.role` are the same four concepts named twice, and the split
 * exists for **English**, not for Japanese: one reads inside a sentence ("Pick from your
 * aesthetics") and wants a plural, the other is a bare label on a chip and wants a
 * singular. Japanese does not inflect for number, so its two values coincide — which is
 * the expected outcome here, not a sign the split was unnecessary.
 *
 * **What is deliberately not here:** the slash palette's four command words. `aesthetic`,
 * `style`, `type` and `item` are typed into the document — the run really says
 * `/aesthetic:warm` — so they are tokens the matcher reads, not labels. See `palette.ts`.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const editor = {
  /* -- the prompt list ---------------------------------------------------------------- */
  list: {
    title: "Prompts",
    new: "New prompt",
    newHint: "Start from the template",
    loading: "Reading your prompts…",
    /** The stamp comes from `lib/format`, which follows the interface language on its own. */
    edited: template<{ when: string }>("Edited {{ when }}"),
    reuse: "Reuse",

    empty: {
      title: "No prompts yet.",
      /**
       * One key rather than three, because the slash sits mid-clause in both languages and
       * a sentence spliced around it would be wrong in one of them.
       *
       * The four words at the end are the palette's command tokens, which is the whole
       * reason this sentence lists them: it is the only place that teaches which word to
       * type. English gets that for free, since the tokens *are* English words. A language
       * that names those four things differently has to carry both — see `../ja/editor.ts`.
       */
      blurb:
        "New prompt starts you on Curio's gold-standard template — a scaffold you fill in " +
        "and delete from freely. Type / anywhere in it to pull an aesthetic, style, type, " +
        "or item out of your library.",
    },

    failed: {
      delete: "That prompt could not be deleted.",
      /** "Created", not "started": a failure should name the operation, not the invitation. */
      create: "A new prompt could not be created.",
    },
  },

  /* -- the document, and the page it lies on ------------------------------------------ */
  document: {
    opening: "Opening the prompt…",
    note:
      "Inserted chips expand when the prompt is serialized: an aesthetic carries its full " +
      "description, and a reference carries the absolute folder path, so the receiving " +
      "agent can read the screenshot and its sidecar straight off disk.",

    /** `Markdown` is capitalised throughout: it is the format's name, not a common noun. */
    source: {
      reading: "Reading the Markdown…",
      empty: "This prompt is empty.",
    },

    failed: {
      save: "The last edit did not save.",
      source: "The Markdown could not be read.",
    },
  },

  /* -- the rail ----------------------------------------------------------------------- */
  toolbar: {
    label: "Prompt document",
    undo: "Undo",
    redo: "Redo",

    block: {
      title: "Block type",
      text: "Text",
      heading: template<{ level: number }>("Heading {{ level }}"),
    },

    bold: "Bold",
    italic: "Italic",
    code: "Inline code",
    bulletList: "Bulleted list",
    orderedList: "Numbered list",
    quote: "Quote",
    codeBlock: "Code block",
    divider: "Divider",

    insert: {
      label: "Insert",
      title: "Insert from the library",
    },

    source: {
      /** A format name, not a word. It stays Latin in both languages. */
      label: "Markdown",
      hint: "Show the text this prompt becomes when it is copied",
      /** Why every editing control is off while the Markdown view is up. */
      blocked: "The Markdown view is read-only — switch back to edit the document",
    },
  },

  /* -- what happens to a whole document ----------------------------------------------- */
  actions: {
    busy: "Curio is working on the last request",

    copy: {
      label: "Copy Prompt",
      hint: "Serialize this prompt and copy it to the clipboard",
      done: "Copied to clipboard",
      refused: "The clipboard refused. Nothing was copied.",
    },

    delete: {
      label: "Delete prompt",
      /** Shared with the prompt list's Delete pill: one sentence, one meaning, one key. */
      hint: "Delete this prompt",
      /** The disarm. It says what happens to the prompt, not that a dialog is closing. */
      keep: "Keep",
      confirm: "Delete for good",
    },

    failed: "Something went wrong.",
  },

  /* -- the `/` menu ------------------------------------------------------------------- */
  slash: {
    label: "Insert from your library",
    /** `kind` is one of `slash.kinds`, which are written to sit inside this sentence. */
    pick: template<{ kind: string }>("Pick from your {{ kind }}"),
    loading: "Reading your library…",
    none: "Nothing here matches yet.",

    hint: {
      tick: "Tab to tick",
      insert: "Enter to insert",
      insertCount: template<{ count: number }>("Enter to insert {{ count }}"),
      close: "Esc to close",
    },

    /** What stage two is picking from, as it reads inside `slash.pick`. Plural in English. */
    kinds: {
      aesthetic: "aesthetics",
      style: "styles",
      type: "types",
      /** A `/item:` run inserts an `itemRef`, so this names the chip, not the library row. */
      item: "references",
    },

    /** The second line of each palette row: what the word will actually insert. */
    hints: {
      aesthetic: "A family from your vocabulary, with its full description",
      style: "A tag — the word on its own",
      type: "A design type — the word on its own",
      item: "A reference, carrying the folder your tool can read",
    },

    /** The second line of a tag or type row. English inflects; Japanese counts with 件. */
    itemCount: {
      one: "1 item",
      other: template<{ count: number }>("{{ count }} items"),
    },
  },

  /* -- the chips themselves ----------------------------------------------------------- */
  chip: {
    /** Read aloud in place of the glyph, and shown on hover. Singular, bare noun. */
    role: {
      family: "Aesthetic",
      tag: "Style",
      type: "Type",
      reference: "Reference",
    },
  },
} as const;

export type Editor = Translated<typeof editor>;
