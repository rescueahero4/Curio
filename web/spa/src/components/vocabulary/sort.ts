import { createSignal } from "solid-js";
import type { VocabEntry } from "~/components/vocabulary/VocabRow";

/** The three columns worth ordering by. Description is prose and does not sort usefully. */
export type SortKey = "name" | "items" | "origin";

export type SortDir = "asc" | "desc";

/**
 * Which way a column wants to run the first time it is clicked.
 *
 * A name sorted Z→A is nobody's first question, and a count sorted 1→200 buries the words
 * worth describing well under the ones worth merging away. So each column opens the way it
 * is usually wanted and reverses on the second click, rather than every column opening
 * ascending and making half of them a two-click affair.
 */
const OPENS: Record<SortKey, SortDir> = {
  name: "asc",
  items: "desc",
  origin: "asc",
};

/**
 * The table's ordering, as one piece of state.
 *
 * This replaces a two-option `<select>` — "Most used" and "Name" — with the same two
 * orderings reachable from the headers they describe, plus their reverses and a third
 * column the select never offered. The default is unchanged: most used first.
 */
export function createSort() {
  const [key, setKey] = createSignal<SortKey>("items");
  const [dir, setDir] = createSignal<SortDir>("desc");

  return {
    key,
    dir,

    /**
     * What the `<th>` announces.
     *
     * `aria-sort` is the only thing that tells a screen reader the table is ordered at all —
     * the arrow is decoration and is `aria-hidden` with every other icon in this app — and
     * it belongs on the header cell rather than the button inside it.
     */
    ariaSort: (column: SortKey): "ascending" | "descending" | "none" => {
      if (key() !== column) return "none";
      return dir() === "asc" ? "ascending" : "descending";
    },

    toggle(column: SortKey): void {
      if (key() === column) {
        setDir((was) => (was === "asc" ? "desc" : "asc"));
        return;
      }
      setKey(column);
      setDir(OPENS[column]);
    },
  };
}

/**
 * Order two entries.
 *
 * Name is the tiebreak for every other column, and it is never itself tied — two terms
 * cannot share a name, because that is exactly what the server's 409 on rename prevents.
 * Without it, a table sorted by item count would shuffle its 47 one-item tags on every
 * re-render, which reads as the page losing its place.
 */
export function compare(left: VocabEntry, right: VocabEntry, key: SortKey, dir: SortDir): number {
  const sign = dir === "asc" ? 1 : -1;

  switch (key) {
    case "name":
      return sign * left.name.localeCompare(right.name);
    case "items":
      return sign * (left.item_count - right.item_count) || left.name.localeCompare(right.name);
    case "origin":
      return (
        sign * left.created_by.localeCompare(right.created_by) ||
        left.name.localeCompare(right.name)
      );
  }
}
