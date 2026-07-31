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
 */

import type { ChipKind } from "~/components/editor/chips";
import { getVocabulary, listItems } from "~/lib/api";

export type PaletteKind = "aesthetic" | "style" | "type" | "item";

export interface PaletteEntry {
  kind: PaletteKind;
  label: string;
  hint: string;
  chip: ChipKind;
  aliases: readonly string[];
}

export const PALETTE: readonly PaletteEntry[] = [
  {
    kind: "aesthetic",
    label: "aesthetic",
    hint: "A family from your vocabulary, with its full description",
    chip: "familyChip",
    aliases: ["family", "families", "aesthetics", "vibe"],
  },
  {
    kind: "style",
    label: "style",
    hint: "A tag — the word on its own",
    chip: "tagChip",
    aliases: ["tag", "tags", "styles"],
  },
  {
    kind: "type",
    label: "type",
    hint: "A design type — the word on its own",
    chip: "typeChip",
    aliases: ["types", "kind"],
  },
  {
    kind: "item",
    label: "item",
    hint: "A reference, carrying the folder your tool can read",
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
    hint: term.item_count === 1 ? "1 item" : `${term.item_count} items`,
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
