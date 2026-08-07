import { useNavigate } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { CopyButton } from "~/components/library/CopyButton";
import { briefText } from "~/components/library/handoff";
import { deleteItem, reassessItem } from "~/lib/api";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Item } from "~/lib/types";

/**
 * The four things a user comes to this page to *do*, at the top of it.
 *
 * These were stacked in the right-hand column under three headings, which put the page's
 * primary actions below the fold of a tall screenshot and gave them the same weight as the
 * prose explaining them. A toolbar in the header is where a document's verbs belong: one
 * row, one treatment, no hierarchy invented between copying a path and copying a brief.
 *
 * Outlined rather than filled, all four. Nothing here is *the* action — an item is read as
 * often as it is re-assessed — and a filled button among outlined ones would claim it is.
 */
export function ItemToolbar(props: {
  item: Item;
  /** The item's absolute directory, once Settings has answered. */
  directory: string | null;
}) {
  const navigate = useNavigate();
  const [asking, setAsking] = createSignal(false);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [problem, setProblem] = createSignal<string | null>(null);

  const pausedReason = () => (paused() ? t("library.item.toolbar.paused") : undefined);

  async function reassess() {
    setBusy("reassess");
    setProblem(null);
    try {
      await reassessItem(props.item.id);
    } catch {
      setProblem(t("library.item.toolbar.errors.reassess"));
    }
    setBusy(null);
  }

  async function remove() {
    setBusy("delete");
    try {
      await deleteItem(props.item.id);
      navigate("/");
    } catch {
      setProblem(t("library.item.toolbar.errors.delete"));
      setBusy(null);
    }
  }

  return (
    <div class="flex flex-col items-end gap-2">
      <div class="flex flex-wrap items-center justify-end gap-2">
        <CopyButton
          label={t("library.item.toolbar.copyFolder")}
          text={() => props.directory ?? ""}
          blocked={props.directory ? undefined : t("library.item.handoff.locating")}
        />
        <CopyButton
          label={t("library.item.toolbar.copyBrief")}
          text={() => briefText(props.item, props.directory)}
        />

        <button
          type="button"
          class="pill pill-outline"
          disabled={!!pausedReason() || busy() === "reassess"}
          title={pausedReason() ?? t("library.item.toolbar.reassessHint")}
          onClick={() => void reassess()}
        >
          {busy() === "reassess"
            ? t("library.item.toolbar.queueing")
            : t("library.item.toolbar.reassess")}
        </button>

        {/* Delete asks in place rather than in a dialog. The question is one line and the
            answer is one click, and a modal for that costs a user their place on the page. */}
        <Show
          when={asking()}
          fallback={
            <button
              type="button"
              class="pill pill-outline"
              disabled={!!pausedReason()}
              title={pausedReason()}
              onClick={() => setAsking(true)}
            >
              {t("common.delete")}
            </button>
          }
        >
          <span class="text-xs text-ink-muted">{t("library.item.toolbar.askDelete")}</span>
          <button
            type="button"
            class="pill tint-caution"
            disabled={busy() === "delete"}
            onClick={() => void remove()}
          >
            {busy() === "delete"
              ? t("library.item.toolbar.deleting")
              : t("library.item.toolbar.yes")}
          </button>
          <button type="button" class="pill pill-outline" onClick={() => setAsking(false)}>
            {t("library.item.toolbar.no")}
          </button>
        </Show>
      </div>

      <Show when={problem()}>
        <output class="banner tint-caution">{problem()}</output>
      </Show>
    </div>
  );
}

/**
 * What is left in the column once the verbs have moved up: the agent handoff.
 *
 * The path block is the point of the panel — Curio's output is a directory an agent can
 * read, and this is where a user sees where that is. Copying it is a toolbar action now,
 * but *showing* it is not an action at all: it is the answer to "where did this go", and it
 * belongs next to the item it describes.
 */
export function ItemActions(props: {
  item: Item;
  directory: string | null;
  apiKeySet: boolean | null;
}) {
  return (
    <aside class="flex flex-col gap-3">
      <Show when={props.apiKeySet === false}>
        <section class="banner tint-caution flex-col items-start gap-1">
          <strong class="font-semibold">{t("library.item.handoff.noKey.title")}</strong>
          <span>{t("library.item.handoff.noKey.blurb")}</span>
        </section>
      </Show>

      <section class="card flex flex-col gap-2 p-3">
        <h2 class="text-sm font-medium">{t("library.item.handoff.title")}</h2>
        <Show
          when={props.directory}
          fallback={<p class="text-xs text-ink-faint">{t("library.item.handoff.locating")}</p>}
        >
          {(directory) => (
            <>
              <code class="block overflow-x-auto rounded bg-desk px-2 py-1 font-mono text-xs">
                {directory()}
              </code>
              {/* One sentence rather than a clause per file name. The two names used to be
                  `<code>` spans sitting mid-clause, and a clause cannot be reordered around
                  markup — Japanese puts the verb after both of them. The names are still
                  literal, because a file name is not a word. */}
              <p class="text-xs text-ink-muted">{t("library.item.handoff.contents")}</p>
            </>
          )}
        </Show>
      </section>

      <section class="card flex flex-col gap-2 p-3">
        <h2 class="text-sm font-medium">{t("library.item.handoff.imagePrompt.title")}</h2>
        <CopyButton
          label={t("library.item.handoff.imagePrompt.copy")}
          text={() => props.item.image_recipe ?? ""}
          blocked={props.item.image_recipe ? undefined : t("library.item.handoff.imagePrompt.none")}
        />
      </section>
    </aside>
  );
}
