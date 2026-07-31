import { type Accessor, createEffect, createSignal, on } from "solid-js";
import { createEscapeLayer } from "~/lib/keyboard";

/**
 * The two selection modes, and only two (R-FE-11).
 *
 * `picked` is a set of ids the user assembled by hand. `matching` is the current filter —
 * every item it selects, including the ones no page has fetched yet, which is the whole
 * reason it cannot be expressed as a list of ids.
 *
 * Because `matching` *is* the filter, changing the filter does not narrow that selection;
 * it makes it mean something the user never asked for. So a filter change voids it. Picked
 * ids survive, because those the user chose individually and a filter change says nothing
 * about them.
 */
export function createSelection(order: Accessor<string[]>, filterKey: Accessor<string>) {
  const [picked, setPicked] = createSignal<string[]>([]);
  const [matching, setMatching] = createSignal(false);

  // Kept outside the reactive graph: the anchor is bookkeeping for the next shift-click,
  // and nothing renders differently because of it.
  let anchor: string | null = null;

  createEffect(on(filterKey, () => setMatching(false), { defer: true }));

  const any = () => matching() || picked().length > 0;

  function clear(): void {
    setPicked([]);
    setMatching(false);
    anchor = null;
  }

  createEscapeLayer("selection", () => {
    if (!any()) return false;
    clear();
    return true;
  });

  /**
   * Click, or shift-click.
   *
   * A shift-click takes the range from the anchor to the clicked card in the order the
   * grid is showing them, and **adds** it. Replacing would silently discard picks the user
   * made before reaching for shift.
   */
  function toggle(id: string, extend: boolean): void {
    setMatching(false);

    const ids = order();
    const from = anchor ? ids.indexOf(anchor) : -1;
    const to = ids.indexOf(id);

    if (extend && from >= 0 && to >= 0) {
      const [start, end] = from <= to ? [from, to] : [to, from];
      const span = ids.slice(start, end + 1);
      setPicked((current) => [...new Set([...current, ...span])]);
      return;
    }

    anchor = id;
    setPicked((current) =>
      current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
    );
  }

  return {
    picked,
    matching,
    any,
    clear,
    toggle,
    isSelected: (id: string) => matching() || picked().includes(id),
    /** Switch to "everything the filter matches". The count is the server's to know. */
    selectMatching: () => {
      setPicked([]);
      anchor = null;
      setMatching(true);
    },
    /** How many ids are picked. Meaningless in `matching` mode, where the answer is remote. */
    count: () => picked().length,
  };
}

export type Selection = ReturnType<typeof createSelection>;
