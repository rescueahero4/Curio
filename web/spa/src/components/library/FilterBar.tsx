import { Show } from "solid-js";
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
import { familyOptions, termOptions } from "~/components/library/vocab";
import type { ItemStatus } from "~/lib/types";

const STATUSES: { id: ItemStatus; label: string }[] = [
  { id: "processing", label: "Waiting for assessment" },
  { id: "ready", label: "Described" },
  { id: "needs_review", label: "Needs review" },
  { id: "assessment_failed", label: "Assessment failed" },
];

/**
 * The filter row (FR-10).
 *
 * Facets AND across kinds and OR within one, which is why each kind gets its own pill and
 * each pill holds checkboxes: "tag: warm **or** muted" and "tag: warm **and** type: hero"
 * are different questions, and the shape of the control is what tells them apart.
 */
export function FilterBar(props: {
  facets: Facets;
  onFacets: (next: Facets) => void;
  dense: boolean;
  onDense: (dense: boolean) => void;
}) {
  const change = (kind: FacetKind, id: string) =>
    props.onFacets(toggleFacet(props.facets, kind, id));

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

      <button
        type="button"
        class="pill"
        classList={{ "tint-caution": props.facets.needs_review }}
        aria-pressed={props.facets.needs_review}
        onClick={() =>
          props.onFacets({ ...props.facets, needs_review: !props.facets.needs_review })
        }
      >
        Needs review
      </button>

      <Show when={facetCount(props.facets) > 0}>
        <button type="button" class="pill" onClick={() => props.onFacets(NO_FACETS)}>
          Clear filters
        </button>
      </Show>

      <div class="ml-auto">
        <button
          type="button"
          class="pill"
          classList={{ "pill-current": props.dense }}
          aria-pressed={props.dense}
          title={props.dense ? "Show larger cards" : "Fit more on screen"}
          onClick={() => props.onDense(!props.dense)}
        >
          {props.dense ? "Comfortable" : "Dense"}
        </button>
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
