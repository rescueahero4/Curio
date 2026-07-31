import { A } from "@solidjs/router";
import { createSignal, For, Show } from "solid-js";
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
        <div class="flex flex-col gap-1 p-3" classList={{ "p-2": props.dense }}>
          <h2 class="truncate font-medium text-ink">{props.item.name || "Untitled"}</h2>
          <Show when={!props.dense}>
            <p class="line-clamp-2 text-sm text-ink-muted">{props.item.short_description}</p>
          </Show>
        </div>
      </A>

      <div
        class="flex flex-wrap items-center gap-1 px-3 pb-3"
        classList={{ "px-2 pb-2": props.dense }}
      >
        <Show when={props.item.status === "processing"}>
          <span class="pill pill-outline text-2xs">Waiting for assessment</span>
        </Show>
        <Show when={props.item.status === "needs_review" || grayZone()}>
          <span class="pill tint-caution text-2xs">Needs review</span>
        </Show>
        <Show when={proposal()}>
          {(family) => <span class="pill tint-proposal text-2xs">Proposed: {family().name}</span>}
        </Show>
        <For each={props.item.families.filter((family) => !family.ai_proposed)}>
          {(family) => <span class="pill pill-outline text-2xs">{family.name}</span>}
        </For>
      </div>

      <Show when={props.item.status === "assessment_failed"}>
        <FailedFooter id={props.item.id} error={props.item.error} />
      </Show>

      <SelectToggle
        name={props.item.name}
        selected={props.selected}
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

/** A real checkbox, positioned over the card: selection must be reachable by keyboard. */
function SelectToggle(props: {
  name: string;
  selected: boolean;
  onToggle: (extend: boolean) => void;
}) {
  return (
    <label
      class="absolute top-2 left-2 flex items-center rounded bg-card/90 p-1 opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100"
      classList={{ "opacity-100": props.selected }}
      style={{ "transition-duration": "var(--duration-hover)" }}
    >
      <span class="sr-only">Select {props.name || "this item"}</span>
      <input
        type="checkbox"
        checked={props.selected}
        onClick={(event) => props.onToggle(event.shiftKey)}
      />
    </label>
  );
}
