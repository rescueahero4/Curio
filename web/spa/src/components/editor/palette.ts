/**
 * The slash menu's two stages, as data (FR-13).
 *
 * Stage one is a palette of four words; stage two is whatever the library holds for the
 * word that was chosen. The stages are separated by a colon in the document itself —
 * `/aesthetic:warm` — rather than by hidden editor state, which is what keeps the whole
 * interaction visible: the user can see what they are filtering, backspace through the
 * colon to change their mind, and the caret never leaves the paragraph.
 *
 * Aliases exist because the vocabulary has two names for most of these already: a "style"
 * is a tag, an "aesthetic" is a family. Both spellings work rather than one being right.
 *
 * ## The four words are not translated, and the two sentences beside them are
 *
 * `label` and `aliases` are what the user types and what ends up in the paragraph — the run
 * really reads `/aesthetic:warm`. Translating them would mean a Japanese reader could see
 * 「ファミリー」 in the menu and have no way to type toward it, because the matcher only
 * ever compares against these words; it would also put a Japanese word into a document the
 * server parses. So they stay as they are, the way `/remind` stays `/remind` in a Japanese
 * Slack, and the row's second line — `noun` and `hint` — carries the meaning instead.
 *
 * Both of those are **thunks, resolved at render**. A label baked in at module load is
 * frozen in whichever language was current when this file was first imported, which for the
 * lazily-loaded editor route is whichever language the user was reading when they opened it.
 */

import type { ChipKind } from "~/components/editor/chips";
import { getVocabulary, listItems } from "~/lib/api";
import { t } from "~/lib/i18n";

export type PaletteKind = "aesthetic" | "style" | "type" | "item";

export interface PaletteEntry {
  kind: PaletteKind;
  /** The command token: typed by the user, matched here, written into the document. */
  label: string;
  /** What this inserts, as it reads inside a sentence. Resolved at render. */
  noun: () => string;
  /** The row's second line. Resolved at render. */
  hint: () => string;
  chip: ChipKind;
  aliases: readonly string[];
}

export const PALETTE: readonly PaletteEntry[] = [
  {
    kind: "aesthetic",
    label: "aesthetic",
    noun: () => t("editor.slash.kinds.aesthetic"),
    hint: () => t("editor.slash.hints.aesthetic"),
    chip: "familyChip",
    aliases: ["family", "families", "aesthetics", "vibe"],
  },
  {
    kind: "style",
    label: "style",
    noun: () => t("editor.slash.kinds.style"),
    hint: () => t("editor.slash.hints.style"),
    chip: "tagChip",
    aliases: ["tag", "tags", "styles"],
  },
  {
    kind: "type",
    label: "type",
    noun: () => t("editor.slash.kinds.type"),
    hint: () => t("editor.slash.hints.type"),
    chip: "typeChip",
    aliases: ["types", "kind"],
  },
  {
    kind: "item",
    label: "item",
    noun: () => t("editor.slash.kinds.item"),
    hint: () => t("editor.slash.hints.item"),
    chip: "itemRef",
    aliases: ["items", "reference", "ref"],
  },
];

/** One row in the stage-two picker. */
export interface PickerEntry {
  id: string;
  label: string;
  hint: string;
}

/** `/aesthetic:warm` → `{ head: "aesthetic", tail: "warm" }`. No colon means stage one. */
export function splitQuery(query: string): { head: string; tail: string | null } {
  const colon = query.indexOf(":");
  if (colon < 0) return { head: query, tail: null };
  return { head: query.slice(0, colon), tail: query.slice(colon + 1) };
}

/** The palette entries a partial word could still become. */
export function matchPalette(head: string): PaletteEntry[] {
  const needle = head.trim().toLowerCase();
  if (!needle) return [...PALETTE];
  return PALETTE.filter((entry) =>
    [entry.label, ...entry.aliases].some((name) => name.startsWith(needle)),
  );
}

/** The one palette entry a completed word means, alias or not. */
export function resolvePalette(head: string): PaletteEntry | null {
  const needle = head.trim().toLowerCase();
  return (
    PALETTE.find((entry) => [entry.label, ...entry.aliases].includes(needle)) ??
    // A prefix resolves only while it is unambiguous: `/ae:` is an aesthetic, `/t:` is not
    // yet anything, and guessing between type and tag would insert the wrong chip.
    single(matchPalette(needle))
  );
}

/** Everything the library can offer for one palette word. */
export async function loadPicker(kind: PaletteKind): Promise<PickerEntry[]> {
  if (kind === "item") {
    const page = await listItems({ limit: 200 });
    return page.items.map((item) => ({
      id: item.id,
      label: item.name,
      hint: item.short_description,
    }));
  }

  const vocabulary = await getVocabulary();

  if (kind === "aesthetic") {
    return vocabulary.families.map((family) => ({
      id: family.id,
      label: family.name,
      hint: family.description,
    }));
  }

  const terms = kind === "style" ? vocabulary.tags : vocabulary.design_types;
  return terms.map((term) => ({
    id: term.id,
    label: term.name,
    // Read once, when the list is fetched, rather than per render: these rows are already
    // server data and the menu is a moment. A language switch with the picker open leaves
    // this one line stale until it is reopened, which is the whole cost.
    hint:
      term.item_count === 1
        ? t("editor.slash.itemCount.one")
        : t("editor.slash.itemCount.other", { count: term.item_count }),
  }));
}

/** Filter a loaded list by what has been typed after the colon. */
export function filterPicker(entries: readonly PickerEntry[], tail: string): PickerEntry[] {
  const needle = tail.trim().toLowerCase();
  if (!needle) return [...entries];
  return entries.filter((entry) => entry.label.toLowerCase().includes(needle));
}

function single(entries: readonly PaletteEntry[]): PaletteEntry | null {
  return entries.length === 1 ? (entries[0] ?? null) : null;
}
