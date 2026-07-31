import { A, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, on, onCleanup, Show } from "solid-js";
import { createStore } from "solid-js/store";
import { createAutosave } from "~/components/library/autosave";
import { applyPatch, mergeServer } from "~/components/library/draft";
import { GrayZoneCard } from "~/components/library/GrayZoneCard";
import { itemDirectory } from "~/components/library/handoff";
import { ItemActions } from "~/components/library/ItemActions";
import { ItemFields } from "~/components/library/ItemFields";
import { ItemLinks } from "~/components/library/ItemLinks";
import { ensureVocabulary } from "~/components/library/vocab";
import { getItem, getSettings, itemImageUrl, updateItem } from "~/lib/api";
import { events } from "~/lib/events";
import { ApiError } from "~/lib/http";
import type { Item, ItemPatch } from "~/lib/types";

/**
 * One item, editable (FR-8, R-FE-13, R-FE-14, R-FE-15b).
 *
 * This route holds the item and arranges the panels; the timing rules live in `autosave`
 * and the merge rules in `draft`, because both are contracts that want to be readable
 * without a page of JSX around them.
 *
 * The item is kept here rather than read from the shared store: a detail page is often
 * opened directly, when no grid page has been fetched and the store is empty.
 */
export function ItemDetail() {
  const params = useParams<{ id: string }>();
  const [state, setState] = createStore<{ item: Item | null }>({ item: null });
  const [problem, setProblem] = createSignal<string | null>(null);
  const [gone, setGone] = createSignal(false);
  const [settings] = createResource(getSettings);

  const autosave = createAutosave((patch) => updateItem(params.id, patch));

  ensureVocabulary();

  createEffect(
    on(
      () => params.id,
      async (id) => {
        setProblem(null);
        setGone(false);
        try {
          setState("item", await getItem(id));
        } catch (error) {
          setState("item", null);
          setProblem(explain(error));
        }
      },
    ),
  );

  const offUpdated = events.on("item.updated", (payload) => {
    const next = payload as Item;
    const current = state.item;
    if (!current || next?.id !== current.id) return;
    setState("item", mergeServer(current, next, autosave));
  });

  const offDeleted = events.on("item.deleted", (payload) => {
    const { id } = (payload ?? {}) as { id?: string };
    if (id && id === state.item?.id) setGone(true);
  });

  onCleanup(() => {
    offUpdated();
    offDeleted();
  });

  /** Show the edit at once, save it 600 ms later (R-FE-13). */
  function edit(patch: ItemPatch): void {
    const current = state.item;
    if (!current) return;
    setState("item", applyPatch(current, patch));
    autosave.edit(patch);
  }

  const directory = () => {
    const root = settings()?.data_root;
    const item = state.item;
    return root && item ? itemDirectory(root, item.id) : null;
  };

  return (
    <section class="flex flex-col gap-4">
      <A href="/" class="pill self-start">
        ← Library
      </A>

      <Show when={gone()}>
        <output class="banner tint-caution">
          This item was deleted.{" "}
          <A href="/" class="underline underline-offset-2">
            Back to the library
          </A>
          .
        </output>
      </Show>

      <Show when={problem()}>
        {(message) => <output class="banner tint-caution">{message()}</output>}
      </Show>

      <Show
        when={state.item}
        fallback={
          <Show when={!problem()}>
            <p class="py-12 text-center text-sm text-ink-faint">Opening…</p>
          </Show>
        }
      >
        {(item) => (
          <>
            <GrayZoneCard item={item()} onResolved={(next) => setState("item", next)} />

            <div class="grid gap-6 lg:grid-cols-[minmax(0,2fr)_minmax(0,1fr)]">
              <div class="flex flex-col gap-4">
                {/* The 16:10 frame is the design language's, so it stays — but the whole
                    screenshot is the point of this page, so it is contained, not cropped. */}
                <img src={itemImageUrl(item())} alt={item().name} class="shot object-contain" />
                <ItemFields
                  item={item()}
                  state={autosave.state()}
                  problem={autosave.problem()}
                  onEdit={edit}
                />
                <ItemLinks item={item()} onEdit={edit} />
              </div>

              <ItemActions
                item={item()}
                directory={directory()}
                apiKeySet={settings()?.api_key_set ?? null}
              />
            </div>
          </>
        )}
      </Show>
    </section>
  );
}

function explain(error: unknown): string {
  if (!(error instanceof ApiError)) return "Could not open that item.";
  if (error.status === 404) return "That item is not in the library. It may have been deleted.";
  if (error.sessionExpired) return "Curio restarted. Open the dashboard from the tray again.";
  if (error.unreachable) return "Curio is not answering. Is it still running?";
  return error.message;
}
