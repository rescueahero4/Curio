import { A } from "@solidjs/router";
import { Show } from "solid-js";
import { ChevronRight } from "~/components/icons";
import { hostOf, ItemFacets } from "~/components/library/ItemFacets";
import { SelectToggle } from "~/components/library/SelectToggle";
import { itemImageUrl } from "~/lib/api";
import { perLocale } from "~/lib/format";
import { t } from "~/lib/i18n";
import type { Item } from "~/lib/types";

/**
 * The shapes are this view's — a bare date in the cell, the whole stamp in the tooltip — but
 * the caching and the reason for it belong to `lib/format.ts`, which explains both.
 */
const WHEN = perLocale((tag) => new Intl.DateTimeFormat(tag, { dateStyle: "medium" }));
const WHEN_EXACT = perLocale(
  (tag) => new Intl.DateTimeFormat(tag, { dateStyle: "full", timeStyle: "short" }),
);

/**
 * One item as a row.
 *
 * The same item as `ItemCard`, ordered for a different question. A grid answers "which of
 * these do I want", so the picture is the row; a list answers "what is in here and when did
 * it arrive", so the picture shrinks to a cell that confirms an identity the name has
 * already given.
 *
 * The row is a link end to end — including the date and the chevron, which are inside it
 * rather than beside it, so the target is the whole row rather than a 16px arrow. Shift-
 * click selects instead of navigating, the same gesture the grid uses (R-FE-22); the
 * checkbox sits outside the link because a checkbox inside an anchor is a click the browser
 * has to arbitrate.
 */
export function ItemRow(props: {
  item: Item;
  selected: boolean;
  onToggle: (id: string, extend: boolean) => void;
}) {
  const proposal = () => props.item.families.find((family) => family.ai_proposed);
  const grayZone = () => props.item.families.some((family) => family.gray_zone);
  const captured = () => new Date(props.item.created_at);
  const host = () => hostOf(props.item.source_url);

  return (
    <div
      class="card group flex items-center gap-3 py-2 pr-3 pl-2"
      classList={{ "border-line-strong": props.selected }}
    >
      <SelectToggle
        name={props.item.name}
        selected={props.selected}
        onToggle={(extend) => props.onToggle(props.item.id, extend)}
      />

      <A
        href={`/items/${props.item.id}`}
        class="flex min-w-0 flex-1 items-center gap-3 outline-none"
        onClick={(event) => {
          if (!event.shiftKey) return;
          event.preventDefault();
          props.onToggle(props.item.id, true);
        }}
      >
        <img
          src={itemImageUrl(props.item)}
          alt=""
          class="shot-row"
          loading="lazy"
          decoding="async"
        />

        <div class="flex min-w-0 flex-1 flex-col">
          <h2 class="truncate font-medium text-ink">
            {props.item.name || t("library.card.untitled")}
          </h2>
          {/* Where it came from, in place of the description a row has no room to finish
              anyway — a truncated sentence is the worst of both, costing the width and
              still not saying the thing. */}
          <Show when={host()}>
            {(where) => <span class="truncate text-xs text-ink-faint">{where()}</span>}
          </Show>
        </div>

        {/* The states that change what a user would do next, and nothing else. A row has
            one line to spend and the grid card is where the full vocabulary lives; these
            are hidden on narrow screens so the name and the date, which are the reason to
            be in this view at all, keep their width. */}
        <div class="hidden shrink-0 items-center gap-1 lg:flex">
          <Show when={props.item.status === "processing"}>
            <span class="badge badge-strong">{t("library.status.processing")}</span>
          </Show>
          <Show when={props.item.status === "assessment_failed"}>
            <span class="badge tint-caution">{t("library.status.failed")}</span>
          </Show>
          <Show when={props.item.status === "needs_review" || grayZone()}>
            <span class="badge tint-caution">{t("library.status.needsReview")}</span>
          </Show>
          <Show when={proposal()}>
            {(family) => (
              <span class="badge tint-proposal">
                {t("library.card.proposed", { name: family().name })}
              </span>
            )}
          </Show>
          <ItemFacets item={props.item} limit={3} />
        </div>

        <time
          class="numeric hidden shrink-0 text-xs text-ink-faint sm:block"
          datetime={props.item.created_at}
          title={WHEN_EXACT().format(captured())}
        >
          {WHEN().format(captured())}
        </time>

        <ChevronRight class="chevron" />
      </A>
    </div>
  );
}
