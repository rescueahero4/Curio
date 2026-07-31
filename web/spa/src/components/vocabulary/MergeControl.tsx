import { createSignal, For, Show } from "solid-js";

/** One thing this entry could be merged into. */
export interface MergeTarget {
  id: string;
  name: string;
}

/**
 * Fold one name into another.
 *
 * Merge is the answer to the collision a rename cannot resolve: two words for one idea.
 * Every item linked to this entry ends up linked to the target — families keep the higher
 * score and the settled gray zone (Inventory §10.14) — and this entry goes.
 *
 * Confirmation names both sides, because the direction is the part a user can get wrong and
 * the part they cannot undo.
 */
export function MergeControl(props: {
  name: string;
  targets: MergeTarget[];
  blocked?: string;
  onMerge: (into: string) => void;
}) {
  const [into, setInto] = createSignal("");

  const chosen = () => props.targets.find((target) => target.id === into());

  return (
    <div class="flex flex-wrap items-center gap-2">
      <label class="flex items-center gap-2 text-sm">
        <span class="text-ink-muted">Merge into</span>
        <select
          class="field"
          value={into()}
          disabled={!!props.blocked || props.targets.length === 0}
          title={
            props.targets.length === 0 ? "There is nothing else to merge this into." : props.blocked
          }
          onChange={(event) => setInto(event.currentTarget.value)}
        >
          <option value="">Choose…</option>
          <For each={props.targets}>
            {(target) => <option value={target.id}>{target.name}</option>}
          </For>
        </select>
      </label>

      <Show when={chosen()}>
        {(target) => (
          <>
            <button
              type="button"
              class="pill pill-ink"
              onClick={() => {
                props.onMerge(target().id);
                setInto("");
              }}
            >
              Merge {props.name} into {target().name}
            </button>
            <span class="text-xs text-ink-faint">
              Everything tagged {props.name} keeps its items and takes the other name.
            </span>
          </>
        )}
      </Show>
    </div>
  );
}
