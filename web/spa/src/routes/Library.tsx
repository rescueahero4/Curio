import { useSearchParams } from "@solidjs/router";
import { createEffect, createMemo, createSignal, For, on, onCleanup, Show } from "solid-js";
import { BulkBar } from "~/components/library/BulkBar";
import { FilterBar } from "~/components/library/FilterBar";
import { createItemFeed, createSentinel } from "~/components/library/feed";
import {
  type Facets,
  filterKey,
  isNarrowed,
  NO_FACETS,
  toFilter,
} from "~/components/library/filters";
import { ItemCard } from "~/components/library/ItemCard";
import { ItemRow } from "~/components/library/ItemRow";
import { createSelection } from "~/components/library/selection";
import { createViewMode } from "~/components/library/view";
import { ensureVocabulary } from "~/components/library/vocab";
import { t } from "~/lib/i18n";
import { items } from "~/lib/stores";

/** R-FE-9. A contract, not a tuning knob: the old app settled on it and users learned it. */
const SEARCH_DEBOUNCE_MS = 200;

/**
 * The library grid — the dashboard's home.
 *
 * The route owns the query and hands it to three collaborators: the feed fetches pages, the
 * selection interprets clicks, and the bulk bar acts on what is selected. Each of those
 * lives in its own file (NFR-8); what is left here is the arrangement.
 *
 * Search comes from the URL rather than from a signal, so a result is reloadable and
 * shareable — `SearchBox` writes `?q=`, and the 200 ms debounce lives here because the
 * grid is what pays for a request (R-FE-9).
 */
export function Library() {
  const [searchParams] = useSearchParams();
  const [facets, setFacets] = createSignal<Facets>(NO_FACETS);
  const [view, setView] = createViewMode();

  const typed = () => (typeof searchParams.q === "string" ? searchParams.q : "");
  const [search, setSearch] = createSignal(typed());

  createEffect(
    on(typed, (value) => {
      const timer = setTimeout(() => setSearch(value), SEARCH_DEBOUNCE_MS);
      onCleanup(() => clearTimeout(timer));
    }),
  );

  const filter = createMemo(() => toFilter(facets(), search().trim()));
  const narrowed = () => isNarrowed(filter());

  const feed = createItemFeed(filter);
  const selection = createSelection(
    () => items.list.map((item) => item.id),
    () => filterKey(filter()),
  );

  ensureVocabulary();
  const sentinel = createSentinel(feed.loadMore);

  return (
    <section class="flex flex-col gap-4">
      <FilterBar facets={facets()} onFacets={setFacets} view={view()} onView={setView} />

      <Show when={feed.problem()}>
        {(message) => (
          <output class="banner tint-caution">
            {message()}
            <button type="button" class="pill pill-outline" onClick={feed.retry}>
              {t("common.retry")}
            </button>
          </output>
        )}
      </Show>

      <Show
        when={items.list.length}
        fallback={<Empty ready={feed.ready()} narrowed={narrowed()} />}
      >
        {/* One list, laid out two ways. The `<ul>` is the same element in both modes so a
            screen reader hears "list of N items" either way — the view control changes how
            much room each one gets, not what the page is. */}
        <ul
          classList={{
            "flex flex-col gap-2": view() === "list",
            "grid grid-cols-1 gap-4 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4":
              view() === "comfortable",
            "grid grid-cols-1 gap-3 sm:grid-cols-3 lg:grid-cols-4 xl:grid-cols-5":
              view() === "dense",
          }}
        >
          <For each={items.list}>
            {(item) => (
              <li>
                <Show
                  when={view() !== "list"}
                  fallback={
                    <ItemRow
                      item={item}
                      selected={selection.isSelected(item.id)}
                      onToggle={selection.toggle}
                    />
                  }
                >
                  <ItemCard
                    item={item}
                    dense={view() === "dense"}
                    selected={selection.isSelected(item.id)}
                    onToggle={selection.toggle}
                  />
                </Show>
              </li>
            )}
          </For>
        </ul>
      </Show>

      {/* The sentinel is rendered only while the server says there is another page, so a
          fully-loaded library cannot sit in a loop asking for one that is not there. */}
      <Show when={feed.more()}>
        <div ref={sentinel} class="h-8" aria-hidden="true" />
      </Show>

      <Show when={feed.loading() && items.list.length > 0}>
        <p class="text-center text-xs text-ink-faint">{t("library.grid.loadingMore")}</p>
      </Show>

      <BulkBar selection={selection} filter={filter} narrowed={narrowed} />
    </section>
  );
}

/**
 * The empty states, which are three different situations and must not read as one.
 *
 * PRD §5: an empty state teaches the next action. "No results" tells the user something
 * they can already see.
 */
function Empty(props: { ready: boolean; narrowed: boolean }) {
  return (
    <Show
      when={props.ready}
      fallback={
        <p class="py-12 text-center text-sm text-ink-faint">{t("library.grid.empty.loading")}</p>
      }
    >
      <div class="card flex flex-col items-center gap-2 px-6 py-12 text-center">
        {/* A title and a body, which is a split both languages keep: two sentences, not one
            sentence broken over two elements. */}
        <Show
          when={props.narrowed}
          fallback={
            <>
              <p class="font-medium">{t("library.grid.empty.fresh.title")}</p>
              <p class="text-sm text-ink-muted">{t("library.grid.empty.fresh.blurb")}</p>
            </>
          }
        >
          <p class="font-medium">{t("library.grid.empty.filtered.title")}</p>
          <p class="text-sm text-ink-muted">{t("library.grid.empty.filtered.blurb")}</p>
        </Show>
      </div>
    </Show>
  );
}
