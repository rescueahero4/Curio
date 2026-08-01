import { A } from "@solidjs/router";
import type { JSX } from "solid-js";
import { For, Show } from "solid-js";
import { ViewComfortable, ViewDense, ViewList } from "~/components/icons";
import {
  type FacetKind,
  type Facets,
  facetCount,
  NO_FACETS,
  toggleFacet,
  toggleStatus,
} from "~/components/library/filters";
import { OptionList } from "~/components/library/OptionList";
import { Popover } from "~/components/library/Popover";
import { createReviewCount } from "~/components/library/reviewCount";
import type { ViewMode } from "~/components/library/view";
import { familyOptions, termOptions } from "~/components/library/vocab";
import type { ItemStatus } from "~/lib/types";

const STATUSES: { id: ItemStatus; label: string }[] = [
  { id: "processing", label: "Waiting for assessment" },
  { id: "ready", label: "Described" },
  { id: "needs_review", label: "Needs review" },
  { id: "assessment_failed", label: "Assessment failed" },
];

const VIEWS: { mode: ViewMode; label: string; hint: string; icon: () => JSX.Element }[] = [
  {
    mode: "comfortable",
    label: "Comfortable grid",
    hint: "Larger cards, with descriptions",
    icon: () => <ViewComfortable />,
  },
  {
    mode: "dense",
    label: "Dense grid",
    hint: "Fit more on screen",
    icon: () => <ViewDense />,
  },
  {
    mode: "list",
    label: "List",
    hint: "One row per item, with capture dates",
    icon: () => <ViewList />,
  },
];

/**
 * The filter row (FR-10).
 *
 * Facets AND across kinds and OR within one, which is why each kind gets its own pill and
 * each pill holds checkboxes: "tag: warm **or** muted" and "tag: warm **and** type: hero"
 * are different questions, and the shape of the control is what tells them apart.
 *
 * The row is read left to right as narrowing then presenting: everything that changes
 * *which* items are shown is on the left, everything that changes *how* they are shown is
 * on the right. Vocabulary sits on the boundary — it is the only thing here that leaves the
 * page, which is why it is a link and not a pill (PRD §5: Vocabulary is reached from the
 * Library, not from the top nav).
 */
export function FilterBar(props: {
  facets: Facets;
  onFacets: (next: Facets) => void;
  view: ViewMode;
  onView: (next: ViewMode) => void;
}) {
  const change = (kind: FacetKind, id: string) =>
    props.onFacets(toggleFacet(props.facets, kind, id));

  const reviewCount = createReviewCount();

  return (
    <div class="flex flex-wrap items-center gap-2">
      <FacetPill
        kind="type"
        title="Design type"
        selected={props.facets.type}
        options={() => termOptions("types")}
        empty="No design types yet. Curio adds them as it describes captures."
        onToggle={change}
      />
      <FacetPill
        kind="family"
        title="Family"
        selected={props.facets.family}
        options={familyOptions}
        empty="No families yet. They appear once Curio has something to group."
        onToggle={change}
      />
      <FacetPill
        kind="tag"
        title="Tag"
        selected={props.facets.tag}
        options={() => termOptions("tags")}
        empty="No tags yet."
        onToggle={change}
      />

      <Popover
        title="Status"
        label={<Label text="Status" count={props.facets.status.length} />}
        active={props.facets.status.length > 0}
        outlined
      >
        {() => (
          <OptionList
            options={STATUSES}
            selected={props.facets.status}
            empty="No statuses."
            onToggle={(id) => {
              const chosen = STATUSES.find((status) => status.id === id);
              if (chosen) props.onFacets(toggleStatus(props.facets, chosen.id));
            }}
          />
        )}
      </Popover>

      {/*
        A toggle, not a menu — so no chevron, and the number beside it is a different number
        from the ones on its neighbours. Theirs count what the user has ticked; this one
        counts what is waiting, which is the only reason to press it. It is deliberately not
        derived from the loaded grid: see `reviewCount`.
      */}
      <button
        type="button"
        class="pill pill-outline"
        classList={{ "tint-caution": props.facets.needs_review }}
        aria-pressed={props.facets.needs_review}
        onClick={() =>
          props.onFacets({ ...props.facets, needs_review: !props.facets.needs_review })
        }
      >
        Needs review
        <Show when={reviewCount() !== null}>
          <span class="numeric text-2xs">{reviewCount()}</span>
        </Show>
      </button>

      <Show when={facetCount(props.facets) > 0}>
        <button type="button" class="pill" onClick={() => props.onFacets(NO_FACETS)}>
          Clear filters
        </button>
      </Show>

      <div class="ml-auto flex items-center gap-3">
        <A href="/vocabulary" class="link-dotted">
          Vocabulary
        </A>

        {/* A `<fieldset>` rather than a div with `role="group"`: three buttons that are one
            choice, and the legend is what a screen reader announces before the first of
            them. It is hidden visually because the icons and the row's position already
            say it to anyone who can see them. */}
        <fieldset class="segmented">
          <legend class="sr-only">How the library is shown</legend>
          <For each={VIEWS}>
            {(entry) => (
              <button
                type="button"
                class="pill pill-icon"
                classList={{ "pill-current": props.view === entry.mode }}
                aria-pressed={props.view === entry.mode}
                aria-label={entry.label}
                title={entry.hint}
                onClick={() => props.onView(entry.mode)}
              >
                {entry.icon()}
              </button>
            )}
          </For>
        </fieldset>
      </div>
    </div>
  );
}

function FacetPill(props: {
  kind: FacetKind;
  title: string;
  selected: string[];
  options: () => { id: string; label: string; count?: number }[];
  empty: string;
  onToggle: (kind: FacetKind, id: string) => void;
}) {
  return (
    <Popover
      title={props.title}
      label={<Label text={props.title} count={props.selected.length} />}
      active={props.selected.length > 0}
      outlined
    >
      {() => (
        <OptionList
          options={props.options()}
          selected={props.selected}
          empty={props.empty}
          onToggle={(id) => props.onToggle(props.kind, id)}
        />
      )}
    </Popover>
  );
}

function Label(props: { text: string; count: number }) {
  return (
    <>
      {props.text}
      <Show when={props.count > 0}>
        <span class="numeric text-2xs">{props.count}</span>
      </Show>
    </>
  );
}
