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
import { t } from "~/lib/i18n";
import type { ItemStatus } from "~/lib/types";

/**
 * Both lists are built per call rather than held as module constants, which is what a
 * translated label costs: `t` is a signal underneath, and a string read once at module scope
 * would be whatever language the tab happened to open in for the rest of the session.
 */
function statuses(): { id: ItemStatus; label: string }[] {
  return [
    { id: "processing", label: t("library.status.processing") },
    { id: "ready", label: t("library.status.ready") },
    { id: "needs_review", label: t("library.status.needsReview") },
    { id: "assessment_failed", label: t("library.status.failed") },
  ];
}

function views(): { mode: ViewMode; label: string; hint: string; icon: () => JSX.Element }[] {
  return [
    {
      mode: "comfortable",
      label: t("library.filters.view.comfortable.label"),
      hint: t("library.filters.view.comfortable.hint"),
      icon: () => <ViewComfortable />,
    },
    {
      mode: "dense",
      label: t("library.filters.view.dense.label"),
      hint: t("library.filters.view.dense.hint"),
      icon: () => <ViewDense />,
    },
    {
      mode: "list",
      label: t("library.filters.view.list.label"),
      hint: t("library.filters.view.list.hint"),
      icon: () => <ViewList />,
    },
  ];
}

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
        title={t("library.filters.type.title")}
        selected={props.facets.type}
        options={() => termOptions("types")}
        empty={t("library.filters.type.empty")}
        onToggle={change}
      />
      <FacetPill
        kind="family"
        title={t("library.filters.family.title")}
        selected={props.facets.family}
        options={familyOptions}
        empty={t("library.filters.family.empty")}
        onToggle={change}
      />
      <FacetPill
        kind="tag"
        title={t("library.filters.tag.title")}
        selected={props.facets.tag}
        options={() => termOptions("tags")}
        empty={t("library.filters.tag.empty")}
        onToggle={change}
      />

      <Popover
        title={t("library.filters.status.title")}
        label={
          <Label text={t("library.filters.status.title")} count={props.facets.status.length} />
        }
        active={props.facets.status.length > 0}
        outlined
      >
        {() => (
          <OptionList
            options={statuses()}
            selected={props.facets.status}
            empty={t("library.filters.status.empty")}
            onToggle={(id) => {
              const chosen = statuses().find((status) => status.id === id);
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
        {t("library.status.needsReview")}
        <Show when={reviewCount() !== null}>
          <span class="numeric text-2xs">{reviewCount()}</span>
        </Show>
      </button>

      <Show when={facetCount(props.facets) > 0}>
        <button type="button" class="pill" onClick={() => props.onFacets(NO_FACETS)}>
          {t("library.filters.clear")}
        </button>
      </Show>

      <div class="ml-auto flex items-center gap-3">
        <A href="/vocabulary" class="link-dotted">
          {t("library.filters.vocabulary")}
        </A>

        {/* A `<fieldset>` rather than a div with `role="group"`: three buttons that are one
            choice, and the legend is what a screen reader announces before the first of
            them. It is hidden visually because the icons and the row's position already
            say it to anyone who can see them. */}
        <fieldset class="segmented">
          <legend class="sr-only">{t("library.filters.view.legend")}</legend>
          <For each={views()}>
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
