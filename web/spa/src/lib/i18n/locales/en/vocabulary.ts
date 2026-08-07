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
  /**
   * Not `common.retry`. This button sits in the fallback for the whole page and asks for the
   * vocabulary again — the second row of the README's table, not the first.
   */
  retry: "Try again",

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
    /**
     * The header checkbox's `sr-only` text when everything shown is already picked. Longer
     * than the bulk bar's `Clear` on purpose — a pill has a row of controls around it to say
     * what it clears, and a screen reader reaching this one has the checkbox and nothing else.
     */
    clearSelection: "Clear the selection",
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
     * What one pass left behind, as one whole sentence per fact.
     *
     * A sentence per outcome rather than a count, a noun and a past participle assembled in
     * the order English happens to want them — the verb is last in Japanese and there is no
     * arrangement of those three fragments that puts it there.
     *
     * None of these carries the reasons, and none carries what goes *between* two of them.
     * The bar renders them as separate children of a `.banner`, which is already a flex row
     * with a gap, so the joining is layout rather than string-building. That matters more
     * than it looks: the reasons are server prose with no guaranteed full stop, and a
     * dictionary key holding the glue would have concatenated two of them into one word.
     */
    result: {
      deleted: template<{ count: number; noun: string }>("{{ count }} {{ noun }} deleted."),
      merged: template<{ count: number; noun: string; target: string }>(
        "{{ count }} {{ noun }} merged into {{ target }}.",
      ),
      refused: template<{ count: number; names: string }>("{{ count }} refused — {{ names }}."),
      nothing: template<{ count: number; names: string }>(
        "Nothing changed. {{ count }} refused — {{ names }}.",
      ),
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
    /*
     * "Type a tag first" was the old wording, and it is what is typed that was wrong: the
     * field beside this asks for a name, not for a tag. Corrected on both sides rather than
     * quietly repaired on the Japanese one.
     */
    needName: template<{ noun: string }>("Type a {{ noun }} name first."),
    paused: "Curio is paused. Resume from the tray icon to add names.",
    failed: "Could not add that.",
    /**
     * The same 409 as `errors.taken`, without the sentence that points at merge.
     *
     * A rename that collides has two terms in it and merge is the control below the message.
     * A create that collides has one: nothing was made, and this popover has no merge control
     * to send anyone to.
     */
    taken: "That name is already taken.",
  },

  /** Why a control is off. PRD §5: a disabled control says so. */
  blocked: {
    paused: "Curio is paused. Resume from the tray icon to edit the vocabulary.",
    busy: "Working…",
  },

  /** What is said when the server turns something away. See `components/vocabulary/errors.ts`. */
  errors: {
    generic: "That change did not go through.",
    paused: "Curio is paused. Resume from the tray icon.",
    /**
     * 409, answered here instead of forwarding the server's English.
     *
     * It names merge because merge is the action this refusal is really pointing at, and it
     * is in the same panel. What the server's own sentence adds is which name collided, and
     * on a rename that is the name still sitting in the field above this banner.
     */
    taken: "That name is already taken. Merge the two if they mean the same thing.",
  },
} as const;

export type Vocabulary = Translated<typeof vocabulary>;
