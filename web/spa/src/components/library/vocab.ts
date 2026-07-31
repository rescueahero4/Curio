import type { Option } from "~/components/library/OptionList";
import { refreshVocabulary, vocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

/**
 * Load the vocabulary once, from wherever needs it first.
 *
 * The store refreshes itself on `vocabulary.updated`, but nothing publishes that event on
 * page load — so the first page that needs names asks for them, and every page after finds
 * them already there.
 */
export function ensureVocabulary(): void {
  if (!vocabulary.loaded) void refreshVocabulary();
}

/** Families as choices. Ids, because the item→family link and the facets are both by id. */
export function familyOptions(): Option[] {
  return vocabulary.families.map((family) => ({
    id: family.id,
    label: family.name,
    count: family.item_count,
  }));
}

/**
 * Types and tags as choices.
 *
 * The id is the option's value because that is what the facets filter by; a bulk edit or an
 * item PATCH sends the **name** instead, which is why `nameOf` exists rather than callers
 * assuming one of the two.
 */
export function termOptions(kind: Exclude<VocabularyKind, "families">): Option[] {
  return terms(kind).map((term) => ({ id: term.id, label: term.name, count: term.item_count }));
}

export function nameOf(kind: Exclude<VocabularyKind, "families">, id: string): string | undefined {
  return terms(kind).find((term) => term.id === id)?.name;
}

export function familyName(id: string): string {
  return vocabulary.families.find((family) => family.id === id)?.name ?? id;
}

function terms(kind: Exclude<VocabularyKind, "families">) {
  return kind === "types" ? vocabulary.design_types : vocabulary.tags;
}
