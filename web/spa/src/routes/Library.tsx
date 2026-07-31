/**
 * The library grid — the dashboard's home.
 *
 * **P3.** The behaviors this route owes are contracts rather than design choices, and each
 * one is numbered, so they are listed here where whoever builds it will read them:
 *
 * - 200 ms debounced search; density toggle persisted in `localStorage` under `curio.dense`
 *   (R-FE-9).
 * - Infinite scroll via `IntersectionObserver` with a 400 px `rootMargin`, driving
 *   **keyset** pagination on a `created_at|id` cursor — never an offset, because a capture
 *   arriving mid-scroll would shift an offset page under the reader (R-FE-9).
 * - `item.created` is **ignored while any filter or search is active**, and prepends only
 *   in the unfiltered view. `item.updated` replaces in place; `item.deleted` removes
 *   (R-FE-10, Inventory §10.25).
 * - Two selection modes, `picked(ids)` and `matching` (= the current filter). Shift-click
 *   ranges from the anchor. A filter change **voids** a `matching` selection but keeps
 *   picked ids. Escape clears. The 500 cap is a **named refusal** carrying the server's
 *   `matched` and `limit` — never a silent trim (R-FE-11).
 */
export function Library() {
  return (
    <section class="flex flex-col gap-3">
      <h1 class="text-xl font-semibold">Library</h1>
      <p style={{ color: "var(--color-muted)" }}>
        The grid, filters, search, and bulk operations land in P3. The server is running and the
        shell is wired — this is the scaffold.
      </p>
    </section>
  );
}
