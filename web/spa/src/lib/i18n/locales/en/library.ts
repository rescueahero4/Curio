/**
 * Library strings, English.
 *
 * English is the source of truth: this file defines the keys, and `../ja/library.ts` is
 * type-checked against `Library`, so a key added here without a Japanese counterpart fails
 * the build rather than shipping as a blank label.
 *
 * The tree is an outline of the screen: the grid and its empty states, the filter row above
 * it, the bulk bar that appears under it, and — because they live in `components/library/`
 * rather than in a folder of their own — the panels of the single-item page (`item`).
 *
 * Two things are shared on purpose. `status` is one set of words for the four states an
 * assessment can be in, read by the badges on a card *and* by the Status filter that selects
 * them; a badge that disagreed with the filter which produced it would be worse than either
 * wording alone. `selectToggle` and `options` belong to controls the vocabulary screen also
 * borrows, and they are named for the control rather than for this screen.
 */

import { template } from "@solid-primitives/i18n";
import type { Translated } from "~/lib/i18n/translated";

export const library = {
  /* -- how far an item's assessment has got ------------------------------------------ */
  status: {
    /** Queued, not started. `items.detail.processing` is the *running* one — not this. */
    processing: "Waiting for assessment",
    ready: "Described",
    needsReview: "Needs review",
    failed: "Assessment failed",
  },

  /* -- the grid, and the three ways it can be empty ---------------------------------- */
  grid: {
    loadingMore: "Loading more…",
    errors: {
      load: "Could not load the library.",
      sessionExpired: "Curio restarted. Open the dashboard from the tray again.",
      unreachable: "Curio is not answering. Is it still running?",
      /**
       * Not `common.retry`: that one re-attempts a single failed write, and this asks for
       * the whole listing again. Two different offers, and they read differently in both
       * languages — see the table in the i18n README.
       */
      retry: "Try again",
    },
    /**
     * Three situations that must not read as one (PRD §5): still reading, nothing captured
     * yet, and nothing matching. Only the last of those is about the filter, and only the
     * middle one is worth teaching the extension in.
     */
    empty: {
      loading: "Reading the library…",
      fresh: {
        title: "Nothing here yet.",
        blurb:
          "Capture a page with the Curio extension, or use + Add Item in the top bar. Curio describes each one as it lands.",
      },
      filtered: {
        title: "Nothing matches this search.",
        blurb: "Clear a filter pill, or search for a word from a name or description.",
      },
    },
  },

  /* -- the filter row ---------------------------------------------------------------- */
  filters: {
    /** Each pill's own name, and what its list says when the vocabulary is still empty. */
    type: {
      title: "Design type",
      empty: "No design types yet. Curio adds them as it describes captures.",
    },
    family: {
      title: "Family",
      empty: "No families yet. They appear once Curio has something to group.",
    },
    tag: { title: "Tag", empty: "No tags yet." },
    status: { title: "Status", empty: "No statuses." },
    clear: "Clear filters",
    /** The one control here that leaves the page, which is why it is a link. */
    vocabulary: "Vocabulary",
    view: {
      /** Read aloud before the three view buttons; the icons say it to everyone else. */
      legend: "How the library is shown",
      comfortable: { label: "Comfortable grid", hint: "Larger cards, fewer per row" },
      dense: { label: "Dense grid", hint: "Fit more on screen" },
      list: { label: "List", hint: "One row per item, with capture dates" },
    },
  },

  /* -- the multi-select list inside every popover ------------------------------------ */
  options: {
    filter: "Filter this list",
    /** One box, two jobs — see `OptionList`. The label has to name both of them. */
    filterOrCreate: "Filter / Create",
    filterOrCreateLabel: template<{ noun: string }>("Filter or create a {{ noun }}"),
    noMatch: "Nothing matches that.",
    create: template<{ noun: string; name: string }>("Create new {{ noun }} “{{ name }}”"),
    /**
     * The singular each list offers to create with. These exist only to fill the two
     * strings above, which is why they live beside them rather than with the item panels
     * that pass them in.
     */
    nouns: {
      family: "Aesthetic Family",
      type: "Design Type",
      tag: "Tag",
    },
  },

  /** The selection checkbox, which the grid, the list and the vocabulary table all use. */
  selectToggle: {
    named: template<{ name: string }>("Select {{ name }}"),
    /** An item Curio has not named yet still has to be selectable by keyboard. */
    unnamed: "Select this item",
  },

  /* -- one item, as a card or as a row ----------------------------------------------- */
  card: {
    untitled: "Untitled",
    proposed: template<{ name: string }>("Proposed: {{ name }}"),
    /** The tooltip on a badge, which is the only place its kind is spelled out. */
    facets: {
      family: template<{ name: string }>("Aesthetic family: {{ name }}"),
      type: template<{ name: string }>("Design type: {{ name }}"),
    },
    /** The assessment failed: the reason, and the one action that changes it. */
    failed: {
      reason: "Curio could not describe this one.",
      paused: "Curio is paused. Resume from the tray icon to re-assess.",
      busy: "Asking Curio to look again…",
      queueing: "Queueing…",
      action: "Re-assess",
      problem: "Could not queue a re-assessment. Try again in a moment.",
    },
  },

  /* -- the bar that appears when something is selected ------------------------------- */
  bulk: {
    selected: template<{ count: number }>("{{ count }} selected"),
    allMatching: "Everything this filter matches",
    selectMatching: "Select everything that matches",
    clearSelection: "Clear selection",
    /** PRD §5: a disabled control says why it is off. */
    paused: "Curio is paused. Resume from the tray icon to edit in bulk.",

    tags: { title: "Tags", empty: "No tags yet — type one below.", fresh: "New tag" },
    types: {
      title: "Types",
      empty: "No design types yet — type one below.",
      fresh: "New type",
    },
    families: {
      title: "Families",
      empty: "No families yet. Create one on the Vocabulary page first.",
    },

    panel: {
      add: "Add",
      /** Taking a word off an item, which is not the same verb as deleting one. */
      remove: "Remove",
      needOne: "Choose at least one first.",
    },

    delete: {
      askMatching: "Delete everything this filter matches?",
      askOne: template<{ count: number }>("Delete {{ count }} item?"),
      askOther: template<{ count: number }>("Delete {{ count }} items?"),
      yes: "Yes, delete",
      no: "Keep them",
    },

    /**
     * What a run left behind, one whole sentence per outcome.
     *
     * Not a verb and a preposition glued around a list of names: English puts the verb
     * first and Japanese puts it last, so a sentence assembled from parts is wrong in one
     * of the two languages however carefully the parts are chosen. The `…One` / `…Other`
     * pairs are English's plural and nothing more — the Japanese is the same sentence
     * twice, counting in 件.
     */
    done: {
      addedOne: template<{ values: string; count: number }>(
        "Added {{ values }} to {{ count }} item.",
      ),
      addedOther: template<{ values: string; count: number }>(
        "Added {{ values }} to {{ count }} items.",
      ),
      removedOne: template<{ values: string; count: number }>(
        "Removed {{ values }} from {{ count }} item.",
      ),
      removedOther: template<{ values: string; count: number }>(
        "Removed {{ values }} from {{ count }} items.",
      ),
      deletedOne: template<{ count: number }>("Deleted {{ count }} item."),
      deletedOther: template<{ count: number }>("Deleted {{ count }} items."),
    },

    errors: {
      /**
       * R-FE-11: the refusal is only actionable with both numbers. No plural pair, because
       * a set that is over the cap is never one item.
       */
      overCap: template<{ matched: number; limit: number }>(
        "{{ matched }} items match; the cap is {{ limit }}. Nothing was changed — narrow the filter, or pick fewer.",
      ),
      paused: "Curio is paused. Resume from the tray icon.",
      failed: "The edit did not go through.",
    },
  },

  /** The clipboard write can be refused, and `CopyButton` says so in place of its label. */
  copyFailed: "Could not reach the clipboard",

  /* -- the single item's panels, which live in this folder --------------------------- */
  item: {
    fields: {
      name: "Name",
      description: "Short description",
      sourceUrl: "Source URL",
      openSource: "Open in a new tab",
      /** The same affordance, spelled out for a reader who cannot see the icon. */
      openSourceLabel: "Open the source URL in a new tab",
      imagePrompt: "Image prompt — what Curio would say to regenerate this look",
      paused: "Curio is paused. Edits are not being saved right now.",
      saved: "Saved.",
      /**
       * There is no Save button, so this line is the whole of the form's honesty: whose
       * words are on screen, and what a re-assessment will do to them.
       */
      byUser: "Edited by you. Re-assessment will keep your name.",
      byCurio: "Described by Curio. Anything you change here is kept.",
      errors: {
        save: "That edit did not save.",
        paused: "Curio is paused, so the edit was not saved. Resume from the tray.",
        unreachable: "Curio is not answering, so the edit was not saved.",
      },
    },

    links: {
      families: "Families",
      types: "Design types",
      tags: "Tags",
      /** The picker's accessible name — what adding here would add to. */
      addFamily: "Add a family",
      addType: "Add to design types",
      addTag: "Add to tags",
      remove: template<{ name: string }>("Remove {{ name }}"),
      emptyFamilies: "No families yet.",
      emptyTypes: "No design types yet.",
      emptyTags: "No tags yet.",
      proposed: "Violet means Curio proposed this family rather than matching an existing one.",
      undescribed:
        "A family here has no description yet. Curio matches new captures against that description, so add one on the Vocabulary page for it to affect anything.",
      errors: {
        unlinked: template<{ name: string }>(
          "“{{ name }}” was created but could not be linked. Reload and add it again.",
        ),
        create: template<{ name: string }>("Could not create “{{ name }}”."),
      },
    },

    grayZone: {
      title: "Needs review.",
      unplaced: "Curio could not place this one in a family. Choose where it belongs.",
      /** The score is formatted before it gets here, so both languages read one sentence. */
      near: template<{ name: string; score: string }>(
        "Curio was not sure enough to decide — {{ name }} scored {{ score }}, inside the band where it asks instead of guessing.",
      ),
      keep: template<{ name: string }>("Keep {{ name }}"),
      acceptProposal: template<{ name: string }>("Accept Curio's proposal: {{ name }}"),
      moveTo: "Move to",
      choose: "Choose a family…",
      paused: "Curio is paused. Resume from the tray icon to decide.",
      busy: "Saving the decision…",
      problem: "That decision did not save. Try again in a moment.",
    },

    toolbar: {
      copyFolder: "Copy folder path",
      copyBrief: "Copy brief",
      reassess: "Re-assess",
      reassessHint: "Re-assessment keeps any name you have edited yourself.",
      queueing: "Queueing…",
      /** Delete asks in place rather than in a dialog, so the question is one line. */
      askDelete: "Delete this item and its folder? The screenshot goes with it.",
      yes: "Yes, delete",
      no: "Keep it",
      deleting: "Deleting…",
      paused: "Curio is paused. Resume from the tray icon.",
      errors: {
        reassess: "Could not queue a re-assessment.",
        delete: "Could not delete this item.",
      },
    },

    handoff: {
      title: "Hand this to an agent",
      locating: "Asking Curio where this item lives…",
      /**
       * The file names are not words and stay as they are; Claude Code is a product name.
       * One sentence rather than a clause per `<code>`, because a sentence broken around
       * markup cannot be reordered and Japanese reorders it.
       */
      contents:
        "The folder holds screenshot.png and item.md. Paste the path into Claude Code and ask it to read them.",
      imagePrompt: {
        title: "Image prompt",
        copy: "Copy image prompt",
        none: "Curio has not written an image prompt for this one yet.",
      },
      noKey: {
        title: "Waiting for an API key.",
        blurb:
          "The screenshot is stored and this page is fully editable. Curio writes the name, description and families once a key is set in Settings — nothing is lost while it waits.",
      },
    },
  },
} as const;

export type Library = Translated<typeof library>;
