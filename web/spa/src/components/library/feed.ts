import { type Accessor, createEffect, createMemo, createSignal, on, onCleanup } from "solid-js";
import { filterKey, isNarrowed } from "~/components/library/filters";
import { listItems } from "~/lib/api";
import { ApiError } from "~/lib/http";
import { t } from "~/lib/i18n";
import { setFilterActive, setItemsPage } from "~/lib/stores";
import type { ItemFilter } from "~/lib/types";

/** R-FE-9. Far enough ahead that the next page lands before the sentinel is on screen. */
const ROOT_MARGIN = "400px";

/**
 * The paged read behind the grid.
 *
 * Pagination is **keyset** on the server's `next_cursor` and nothing else — no page number,
 * no offset. A capture arriving mid-scroll shifts every offset by one, which shows the
 * reader a row they have already seen or skips one they have not; a cursor names a row, so
 * a new item at the top cannot move it.
 *
 * Fetched pages land in the shared item store rather than in local state, so an SSE
 * `item.updated` patches the same array the grid is reading (R-FE-5, ARCH-03 store
 * discipline). This module never writes an item itself; it hands whole pages over.
 */
export function createItemFeed(filter: Accessor<ItemFilter>) {
  const [cursor, setCursor] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  const [ready, setReady] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);

  const key = createMemo(() => filterKey(filter()));

  // Every in-flight request carries the generation it was issued under. A response from a
  // filter the user has already changed must not land — it would show results for a query
  // that is no longer on screen.
  let generation = 0;

  createEffect(on(key, () => void load("replace")));

  async function load(mode: "replace" | "append"): Promise<void> {
    const mine = ++generation;
    const current = filter();

    if (mode === "replace") {
      setReady(false);
      setCursor(null);
      setFilterActive(isNarrowed(current));
    }

    setLoading(true);
    setProblem(null);
    try {
      const page = await listItems({
        ...current,
        cursor: mode === "append" ? cursor() : null,
      });
      if (mine !== generation) return;

      setItemsPage(page.items, mode);
      setCursor(page.next_cursor);
      setReady(true);
    } catch (error) {
      if (mine !== generation) return;
      setProblem(explainFeed(error));
    } finally {
      if (mine === generation) setLoading(false);
    }
  }

  return {
    /** True once a page for the current filter has landed — empty is then a real answer. */
    ready,
    loading,
    problem,
    /** More to fetch. `null` from the server means this was the last page. */
    more: () => cursor() !== null,
    loadMore: () => {
      if (loading() || !cursor()) return;
      void load("append");
    },
    retry: () => void load(cursor() ? "append" : "replace"),
  };
}

/**
 * The infinite-scroll sentinel (R-FE-9).
 *
 * Returned as a ref callback so the observer's lifetime is the element's: Solid runs the
 * callback under the component's owner, which is what makes `onCleanup` here disconnect on
 * unmount rather than leaking an observer per visit to the Library.
 */
export function createSentinel(onReach: () => void) {
  return (element: HTMLElement) => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (entries.some((entry) => entry.isIntersecting)) onReach();
      },
      { rootMargin: ROOT_MARGIN },
    );
    observer.observe(element);
    onCleanup(() => observer.disconnect());
  };
}

/**
 * The last line returns the server's own words, which arrive in one language — English —
 * whatever the dashboard is set to. Everything Curio's front end has to say about a failure
 * is translated; a message the API wrote is passed through rather than guessed at.
 */
function explainFeed(error: unknown): string {
  if (!(error instanceof ApiError)) return t("library.grid.errors.load");
  if (error.sessionExpired) return t("library.grid.errors.sessionExpired");
  if (error.unreachable) return t("library.grid.errors.unreachable");
  return error.message;
}
