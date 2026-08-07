/**
 * Vocabulary strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/vocabulary.ts` is
 * type-checked against `Vocabulary`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * The page is three collections behind three tabs, so almost every sentence on it names one
 * of them — "Search tags", "No design types yet", "Delete 3 families?". `kinds` is where
 * those nouns live, and every sentence that needs one is a whole template with a `{{ noun }}`
 * slot rather than a fragment glued to a variable. English can get away with the glue;
 * Japanese puts the particle after the noun and the verb at the end, so a sentence assembled
 * as `"Search " + noun` has nowhere for either to go.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const vocabulary = {
  title: "Vocabulary",
  blurb:
    "Every name Curio uses to describe your library. Rename one and every item follows; merge two when they turned out to mean the same thing.",
  loading: "Reading the vocabulary…",

  /** Shared by the header checkbox's `sr-only` text and the bulk bar's button. */
  clearSelection: "Clear the selection",

  /** Said by the row panel and by the bulk bar, about one word or about thirty. */
  confirmDelete: "Yes, delete",

  tabs: {
    label: "Vocabulary collections",
    families: "Families",
    types: "Design types",
    tags: "Tags",
  },

  /**
   * The three collections as nouns inside a sentence, not as tab labels.
   *
   * Two forms because English inflects and a bar reading "1 families" reads as a bug. The
   * Japanese is one word twice: 件 does the counting, so there is nothing for a second form
   * to say.
   */
  kinds: {
    families: { one: "family", other: "families" },
    types: { one: "design type", other: "design types" },
    tags: { one: "tag", other: "tags" },
  },

  /** The two things a term has, wherever they are asked for — column, row panel, add form. */
  fields: {
    name: "Name",
    description: "Description",
  },

  /** Who coined a name: the filter, its options, and the cell that answers it. */
  origin: {
    label: "Named by",
    anyone: "Anyone",
    ai: "Curio",
    user: "You",
  },

  table: {
    shown: template<{ shown: number; total: number }>("{{ shown }} of {{ total }}"),
    search: {
      placeholder: "Search",
      label: template<{ noun: string }>("Search {{ noun }}"),
    },
    columns: {
      items: "Items",
      actions: "Actions",
    },
    selectAll: template<{ count: number }>("Select all {{ count }} shown"),
    empty: {
      filtered: "Nothing here matches those filters.",
      none: template<{ noun: string }>(
        "No {{ noun }} yet. Curio adds them as it describes captures, and you can add your own above.",
      ),
    },
  },

  row: {
    noDescription: "No description",
    rename: "Rename",
    unchanged: "The name has not changed.",
    descriptionHint:
      "Curio reads this when it decides what belongs here, and it is what a family chip expands to in a prompt. Describing the feel is worth more than listing examples.",
    saveDescription: "Save description",
    /** The count is in the question because it is the reassurance: the items are not going. */
    confirm: template<{ name: string; count: number }>(
      "Delete {{ name }}? The {{ count }} items stay — they just lose this word.",
    ),
    keep: "Keep it",
  },

  merge: {
    into: "Merge into",
    choose: "Choose…",
    empty: "There is nothing else to merge this into.",
    action: template<{ name: string; target: string }>("Merge {{ name }} into {{ target }}"),
    hint: template<{ name: string }>(
      "Everything tagged {{ name }} keeps its items and takes the other name.",
    ),
  },

  bulk: {
    selected: template<{ count: number }>("{{ count }} selected"),
    progress: template<{ done: number; total: number }>("{{ done }} of {{ total }}…"),
    mergeEmpty: "There is nothing left to merge these into.",
    merge: template<{ count: number; target: string }>("Merge {{ count }} into {{ target }}"),
    /**
     * The same question twice, because English cannot say it once: "this word" and "these
     * words" are the only difference, and the count that decides between them is the part a
     * user can get wrong. The component picks; both keys take the same arguments so the call
     * site is one expression. Both Japanese values are the same sentence.
     */
    confirmOne: template<{ count: number; noun: string }>(
      "Delete {{ count }} {{ noun }}? The items stay — they just lose this word.",
    ),
    confirmOther: template<{ count: number; noun: string }>(
      "Delete {{ count }} {{ noun }}? The items stay — they just lose these words.",
    ),
    keep: "Keep them",
    clear: "Clear",

    /**
     * What one pass left behind.
     *
     * A whole sentence per outcome rather than a count, a noun and a past participle
     * assembled in the order English happens to want them — the verb is last in Japanese and
     * there is no arrangement of those three fragments that puts it there.
     */
    result: {
      deleted: template<{ count: number; noun: string }>("{{ count }} {{ noun }} deleted."),
      merged: template<{ count: number; noun: string; target: string }>(
        "{{ count }} {{ noun }} merged into {{ target }}.",
      ),
      refused: template<{ count: number; names: string; why: string }>(
        "{{ count }} refused — {{ names }}. {{ why }}",
      ),
      nothing: template<{ count: number; names: string; why: string }>(
        "Nothing changed. {{ count }} refused — {{ names }}. {{ why }}",
      ),
      /** What goes between the refused names. A comma in English, a 、 in Japanese. */
      separator: ", ",
      /**
       * What goes between two finished sentences — the outcome and the refusals, or two
       * distinct reasons. English puts a space after a full stop and Japanese does not put
       * one after 。, which is the sort of thing a reader notices without being able to say
       * what is wrong.
       */
      spacer: " ",
    },
  },

  add: {
    label: "Add",
    title: "Add to the vocabulary",
    /**
     * The menu's own names for the collections, which are not the tab labels: a tab is a
     * plural heading over a list, a menu item is the singular thing about to be made.
     */
    kinds: {
      families: "Aesthetic Family",
      types: "Design Type",
      tags: "Tag",
    },
    back: "Back to the list of collections",
    heading: template<{ noun: string }>("New {{ noun }}"),
    descriptionHint: "What Curio matches against, and what the chip expands to in a prompt.",
    submit: "Add",
    busy: "Adding…",
    needName: template<{ noun: string }>("Type a {{ noun }} first."),
    paused: "Curio is paused. Resume from the tray icon to add names.",
    failed: "Could not add that.",
  },

  /** Why a control is off. PRD §5: a disabled control says so. */
  blocked: {
    paused: "Curio is paused. Resume from the tray icon to edit the vocabulary.",
    busy: "Working…",
  },

  /** What is said when the server turns something away and does not say why itself. */
  errors: {
    generic: "That change did not go through.",
    paused: "Curio is paused. Resume from the tray icon.",
  },
} as const;

export type Vocabulary = Translated<typeof vocabulary>;
