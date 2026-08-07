import type { Option } from "~/components/library/OptionList";
import { type Locale, locale } from "~/lib/i18n";
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

/**
 * Which marks each language puts around a name, and how it strings quoted names together.
 *
 * The two tables move together and that is the point. English quotes with “ ” and still wants
 * its commas, so its list is a `conjunction`. Japanese quotes with 「 」, and a 、 between two
 * corner-bracketed items is not something Japanese writes — the brackets are the separator,
 * so the items are juxtaposed, which is what CLDR's `unit` type does for `ja` and nothing
 * else does. A third language would add one row to each and touch no code.
 */
const QUOTE: Record<Locale, [string, string]> = {
  en: ["“", "”"],
  ja: ["「", "」"],
};

const QUOTED_JOIN: Record<Locale, Intl.ListFormatType> = {
  en: "conjunction",
  ja: "unit",
};

/**
 * One `Intl.ListFormat` per language and type, kept, for the same reason `lib/format.ts`
 * keeps its formatters: the constructor is the expensive part, and reading `locale()` here
 * still tracks, so a list rendered in a component re-joins itself on a language change.
 *
 * `narrow` throughout: these lists sit inside a sentence that has already said what the
 * relationship between the names is, so the trailing "and" would be saying it twice.
 */
const JOIN = new Map<string, Intl.ListFormat>();

function joiner(tag: Locale, type: Intl.ListFormatType): Intl.ListFormat {
  const key = `${tag}:${type}`;
  let formatter = JOIN.get(key);
  if (!formatter) {
    formatter = new Intl.ListFormat(tag, { style: "narrow", type });
    JOIN.set(key, formatter);
  }
  return formatter;
}

/**
 * Several names, run together into one phrase.
 *
 * The separator is a translation like any other — English joins with a comma and a space,
 * Japanese with 、 — and `Intl.ListFormat` already knows both, so no dictionary key has to
 * carry a piece of punctuation. For a bare enumeration where nothing else in the sentence
 * could be mistaken for part of the list: the overflow tooltip on a card, and not much else.
 */
export function listOf(names: string[]): string {
  return joiner(locale(), "conjunction").format(names);
}

/**
 * The same names, each one delimited, for a list that lands inside a sentence.
 *
 * A tag is free text and can contain the very punctuation the list is joined with — a tag
 * literally named `warm, muted` inside `Added warm, muted, editorial to 40 items.` is two
 * names or three and the sentence cannot say which. Quoting each name answers it, and it has
 * to be *each* name: one pair of marks around the whole run reads as a single name that
 * happens to contain a separator, which is the same ambiguity wearing quotes.
 */
export function quotedList(names: string[]): string {
  const tag = locale();
  const [open, close] = QUOTE[tag];
  return joiner(tag, QUOTED_JOIN[tag]).format(names.map((name) => `${open}${name}${close}`));
}
