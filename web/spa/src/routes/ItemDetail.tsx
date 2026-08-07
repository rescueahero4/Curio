import { A, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, on, onCleanup, Show } from "solid-js";
import { createStore } from "solid-js/store";
import { createAutosave } from "~/components/library/autosave";
import { applyPatch, mergeServer } from "~/components/library/draft";
import { GrayZoneCard } from "~/components/library/GrayZoneCard";
import { itemDirectory } from "~/components/library/handoff";
import { ItemActions, ItemToolbar } from "~/components/library/ItemActions";
import { ItemFields } from "~/components/library/ItemFields";
import { ItemLinks } from "~/components/library/ItemLinks";
import { ensureVocabulary } from "~/components/library/vocab";
import { getItem, getSettings, itemDetailImageUrl, updateItem } from "~/lib/api";
import { events } from "~/lib/events";
import { ApiError } from "~/lib/http";
import { t } from "~/lib/i18n";
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
  // The failure is kept, not the sentence describing it, so a banner already on screen
  // re-reads itself when the language changes under it. Storing `explain(error)` would
  // freeze whichever language was current at the moment the fetch failed.
  const [failure, setFailure] = createSignal<unknown>(null);
  const problem = () => {
    const error = failure();
    return error ? explain(error) : null;
  };
  const [gone, setGone] = createSignal(false);
  const [settings] = createResource(getSettings);

  const autosave = createAutosave((patch) => updateItem(params.id, patch));

  ensureVocabulary();

  createEffect(
    on(
      () => params.id,
      async (id) => {
        setFailure(null);
        setGone(false);
        try {
          setState("item", await getItem(id));
        } catch (error) {
          setState("item", null);
          // Functional form: a bare `setFailure(error)` would let Solid mistake a thrown
          // function for an updater and call it.
          setFailure(() => error);
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
      {/* Where you came from on the left, what you can do on the right. */}
      <div class="flex flex-wrap items-start justify-between gap-3">
        <div class="flex flex-wrap items-center gap-2">
          {/* The arrow is decoration on a link whose word is the destination, so it stays
              here rather than being baked into the string — 「← ライブラリ」 reads the same
              way round, and a translator should never have to carry a glyph. */}
          <A href="/" class="pill">
            <span aria-hidden="true">←</span> {t("items.detail.back")}
          </A>

          {/* The grid already says this on the card it was opened from; saying it here too is
              the point. An item opened while it is still queued has empty fields and no
              families, and without this badge that is indistinguishable from an item Curio
              looked at and had nothing to say about. */}
          <Show when={state.item?.status === "processing"}>
            <span class="badge badge-strong" aria-live="polite">
              <span class="dot-live" aria-hidden="true" />
              {t("items.detail.processing")}
            </span>
          </Show>
        </div>

        <Show when={state.item}>
          {(item) => <ItemToolbar item={item()} directory={directory()} />}
        </Show>
      </div>

      <Show when={gone()}>
        {/* Two whole clauses, not one sentence with a link inside it: the statement and the
            way out survive translation independently, and the full stop that used to trail
            the link is gone — it was underlined-adjacent in English and would have been a
            stray `.` after 「ライブラリに戻る」. */}
        <output class="banner tint-caution">
          {t("items.detail.gone.message")}
          {/* The gap is a margin rather than a text node: English wants a word space after
              its full stop, 。 carries one of its own, and a literal space on top of that
              opens a visible hole before the link. */}
          <A href="/" class="ml-1 underline underline-offset-2">
            {t("items.detail.gone.back")}
          </A>
        </output>
      </Show>

      <Show when={problem()}>
        {(message) => <output class="banner tint-caution">{message()}</output>}
      </Show>

      <Show
        when={state.item}
        fallback={
          <Show when={!problem()}>
            <p class="py-12 text-center text-sm text-ink-faint">{t("common.loading")}</p>
          </Show>
        }
      >
        {(item) => (
          <>
            <GrayZoneCard item={item()} onResolved={(next) => setState("item", next)} />

            {/* Two columns, split by kind rather than by importance: everything that is an
                image on the left, everything that is words on the right. Interleaving the
                two — which is what putting the fields under the screenshot did — pushed the
                edit form below the fold for exactly the tall full-page captures that most
                need describing. `items-start` keeps the text column at the top of a column
                that a long screenshot has made very tall.

                The text column is capped rather than proportional. Everything in it — a
                name, three short fields, rows of chips — is done at 26rem, and a share of
                the width would have gone on growing the whitespace beside them; capped, the
                whole of a wider window reaches the capture, which is the one thing on this
                page that can always use more room. */}
            <div class="grid items-start gap-6 lg:grid-cols-[minmax(0,1fr)_minmax(0,26rem)]">
              <ItemShot item={item()} />

              <div class="flex flex-col gap-4">
                <ItemFields
                  item={item()}
                  state={autosave.state()}
                  problem={autosave.problem()}
                  onEdit={edit}
                />
                <ItemLinks item={item()} onEdit={edit} />
                <ItemActions
                  item={item()}
                  directory={directory()}
                  apiKeySet={settings()?.api_key_set ?? null}
                />
              </div>
            </div>
          </>
        )}
      </Show>
    </section>
  );
}

/**
 * The capture, alone in its column, whole.
 *
 * Nothing is cropped here and nothing is letterboxed: the image gets the column's full
 * width and whatever height its own proportions ask for. 16:10 is the *grid's* contract —
 * it keeps a page of thumbnails from reflowing under a reader mid-scroll — and this page
 * has one image, which is the subject rather than a thumbnail of it. A full-page screenshot
 * held to that frame is either cut off below its masthead or squeezed into a stripe between
 * two grey bars, and both answer a question nobody asked.
 *
 * The frame is still *reserved* at 16:10 until `load`, so the fields beside it do not jump
 * when the bytes arrive; `loaded` then releases it to the real shape. `complete` is checked
 * too, because a cached image can finish decoding before this handler is attached — without
 * it, a revisit would sit boxed at 16:10 forever waiting for a `load` that already happened.
 */
function ItemShot(props: { item: Item }) {
  const [loaded, setLoaded] = createSignal(false);

  return (
    <img
      src={itemDetailImageUrl(props.item)}
      alt={props.item.name}
      class="shot"
      classList={{ "shot-full": loaded() }}
      ref={(image) => queueMicrotask(() => image.complete && setLoaded(true))}
      onLoad={() => setLoaded(true)}
    />
  );
}

/** See `AddItemDialog`'s `explain`: the same shape, over the failures this page can hit. */
function explain(error: unknown): string {
  if (!(error instanceof ApiError)) return t("items.detail.errors.open");
  if (error.status === 404) return t("items.detail.errors.notFound");
  if (error.sessionExpired) return t("items.errors.sessionExpired");
  if (error.unreachable) return t("items.errors.unreachable");
  return error.message;
}
