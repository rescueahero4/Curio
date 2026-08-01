import { A } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { hostOf, ItemFacets } from "~/components/library/ItemFacets";
import { SelectToggle } from "~/components/library/SelectToggle";
import { itemImageUrl, reassessItem } from "~/lib/api";
import { paused } from "~/lib/http";
import type { Item } from "~/lib/types";

/**
 * One item in the grid.
 *
 * Every state this card can be in says something true (PRD §5): a processing item is
 * "waiting for assessment" rather than spinning, a failed one carries the reason and the
 * one action that resolves it, and a family Curio proposed is marked as proposed rather
 * than presented as decided.
 *
 * The whole card is a link, so Enter opens the detail (R-FE-22). Shift-click selects
 * instead of navigating — the selection is the other thing a user does with a grid, and it
 * needs a gesture that does not leave the page.
 */
export function ItemCard(props: {
  item: Item;
  selected: boolean;
  dense: boolean;
  onToggle: (id: string, extend: boolean) => void;
}) {
  const proposal = () => props.item.families.find((family) => family.ai_proposed);
  const grayZone = () => props.item.families.some((family) => family.gray_zone);
  const host = () => hostOf(props.item.source_url);

  return (
    <article
      class="card group relative flex flex-col overflow-hidden"
      classList={{ "border-line-strong": props.selected }}
    >
      <A
        href={`/items/${props.item.id}`}
        class="flex flex-col outline-none"
        onClick={(event) => {
          if (!event.shiftKey) return;
          event.preventDefault();
          props.onToggle(props.item.id, true);
        }}
      >
        <img
          src={itemImageUrl(props.item)}
          alt={props.item.name}
          class="shot"
          loading="lazy"
          decoding="async"
        />
        {/* Name and source, and no description.

            A short description is a sentence, and a sentence is the one thing a scanned
            grid cannot read — it costs two lines on every tile to say something the
            screenshot above it has already shown. The host earns that space instead: it is
            the fact the picture cannot carry, and it is one line at any width. */}
        <div class="flex flex-col gap-1 p-0" classList={{ "p-2": props.dense }}>
          <h2 class="truncate font-medium text-ink p-0 m-0 pt-1">
            {props.item.name || "Untitled"}
          </h2>
          <Show when={host()}>
            {(where) => <span class="truncate text-xs text-ink-faint">{where()}</span>}
          </Show>
        </div>
      </A>

      <div
        class="flex flex-wrap items-center gap-1 px-0 pb-3 pt-1"
        classList={{ "px-2 pb-2": props.dense }}
      >
        <Show when={props.item.status === "processing"}>
          <span class="badge badge-strong">Waiting for assessment</span>
        </Show>
        <Show when={props.item.status === "needs_review" || grayZone()}>
          <span class="badge tint-caution">Needs review</span>
        </Show>
        <Show when={proposal()}>
          {(family) => <span class="badge tint-proposal">Proposed: {family().name}</span>}
        </Show>
        {/* Dense tiles are half the width, so they get half the vocabulary before the
            overflow marker takes over. */}
        <ItemFacets item={props.item} limit={props.dense ? 2 : 4} />
      </div>

      <Show when={props.item.status === "assessment_failed"}>
        <FailedFooter id={props.item.id} error={props.item.error} />
      </Show>

      <SelectToggle
        name={props.item.name}
        selected={props.selected}
        class="absolute top-2 left-2 rounded bg-card/90 p-1"
        onToggle={(extend) => props.onToggle(props.item.id, extend)}
      />
    </article>
  );
}

/** The assessment failed. The reason, and the one action that changes it. */
function FailedFooter(props: { id: string; error: string | null }) {
  const [busy, setBusy] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);

  const blocked = () => {
    if (paused()) return "Curio is paused. Resume from the tray icon to re-assess.";
    if (busy()) return "Asking Curio to look again…";
    return undefined;
  };

  return (
    <div class="banner tint-caution m-3 mt-0 flex-col items-start gap-1">
      <span>{problem() ?? props.error ?? "Curio could not describe this one."}</span>
      <button
        type="button"
        class="pill pill-outline text-2xs"
        disabled={!!blocked()}
        title={blocked()}
        onClick={async () => {
          setBusy(true);
          setProblem(null);
          try {
            await reassessItem(props.id);
          } catch {
            setProblem("Could not queue a re-assessment. Try again in a moment.");
          }
          setBusy(false);
        }}
      >
        {busy() ? "Queueing…" : "Re-assess"}
      </button>
    </div>
  );
}
