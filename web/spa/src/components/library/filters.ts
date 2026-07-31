import type { ItemFilter, ItemStatus } from "~/lib/types";

/**
 * The facet row's state, and the one rule that governs it.
 *
 * Facets AND across kinds and OR within one (FR-10): two tags widen, a tag plus a type
 * narrows. Keeping each kind as its own array is what makes that shape obvious here and in
 * the query string the server parses.
 */
export interface Facets {
  type: string[];
  family: string[];
  tag: string[];
  status: ItemStatus[];
  needs_review: boolean;
}

export const NO_FACETS: Facets = {
  type: [],
  family: [],
  tag: [],
  status: [],
  needs_review: false,
};

/** The multi-select kinds. `status` and `needs_review` have their own controls. */
export type FacetKind = "type" | "family" | "tag";

export function toggleFacet(facets: Facets, kind: FacetKind, id: string): Facets {
  const next = flip(facets[kind], id);
  return {
    ...facets,
    type: kind === "type" ? next : facets.type,
    family: kind === "family" ? next : facets.family,
    tag: kind === "tag" ? next : facets.tag,
  };
}

export function toggleStatus(facets: Facets, status: ItemStatus): Facets {
  return { ...facets, status: flip(facets.status, status) };
}

function flip<T extends string>(list: readonly T[], value: T): T[] {
  return list.includes(value) ? list.filter((entry) => entry !== value) : [...list, value];
}

/** How many choices are live. Drives the "Clear" affordance and the pill counts. */
export function facetCount(facets: Facets): number {
  return (
    facets.type.length +
    facets.family.length +
    facets.tag.length +
    facets.status.length +
    (facets.needs_review ? 1 : 0)
  );
}

export function toFilter(facets: Facets, search: string): ItemFilter {
  const filter: ItemFilter = {};
  if (facets.type.length) filter.type = facets.type;
  if (facets.family.length) filter.family = facets.family;
  if (facets.tag.length) filter.tag = facets.tag;
  if (facets.status.length) filter.status = facets.status;
  if (facets.needs_review) filter.needs_review = true;
  if (search) filter.q = search;
  return filter;
}

/**
 * Whether the view is narrowed at all.
 *
 * The store gates `item.created` on this (Inventory §10.25), and a blank search box does
 * not count — `toFilter` only sets `q` when there is something to search for, so an empty
 * object is the honest answer to "is anything filtered".
 */
export function isNarrowed(filter: ItemFilter): boolean {
  return Object.keys(filter).length > 0;
}

/**
 * A stable identity for a filter.
 *
 * Two things key off it: the feed reloads when it changes, and a `matching` selection is
 * voided when it changes (R-FE-11). Both need "the same query" to compare equal across
 * re-renders, which object identity does not give.
 */
export function filterKey(filter: ItemFilter): string {
  return JSON.stringify([
    filter.type ?? [],
    filter.family ?? [],
    filter.tag ?? [],
    filter.status ?? [],
    filter.needs_review ?? false,
    filter.q ?? "",
  ]);
}
