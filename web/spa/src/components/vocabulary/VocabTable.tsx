import { createMemo, createSignal, For, Show } from "solid-js";
import { type VocabEntry, VocabRow } from "~/components/vocabulary/VocabRow";
import type { VocabularyKind } from "~/lib/types";

type Sort = "name" | "items";
type Origin = "all" | "ai" | "user";

/**
 * One collection, sorted and filtered.
 *
 * Sorting by item count is the maintenance view: the words with one item are the ones worth
 * merging away, and the words with two hundred are the ones worth describing well. Filtering
 * by who coined a name is the other half of that — a list of everything Curio invented is
 * exactly the list a user wants to audit.
 */
export function VocabTable(props: { kind: VocabularyKind; entries: VocabEntry[]; noun: string }) {
  const [needle, setNeedle] = createSignal("");
  const [sort, setSort] = createSignal<Sort>("items");
  const [origin, setOrigin] = createSignal<Origin>("all");

  const shown = createMemo(() => {
    const query = needle().trim().toLowerCase();
    const filtered = props.entries.filter((entry) => {
      if (origin() !== "all" && entry.created_by !== origin()) return false;
      return !query || entry.name.toLowerCase().includes(query);
    });

    return filtered.sort((left, right) =>
      sort() === "name"
        ? left.name.localeCompare(right.name)
        : right.item_count - left.item_count || left.name.localeCompare(right.name),
    );
  });

  const targets = (id: string) =>
    props.entries
      .filter((entry) => entry.id !== id)
      .map((entry) => ({ id: entry.id, name: entry.name }));

  return (
    <div class="flex flex-col gap-3">
      <div class="flex flex-wrap items-center gap-2">
        <input
          type="search"
          class="field max-w-xs"
          placeholder={`Search ${props.noun}`}
          value={needle()}
          onInput={(event) => setNeedle(event.currentTarget.value)}
        />

        <label class="flex items-center gap-2 text-sm text-ink-muted">
          Sort
          <select
            class="field"
            value={sort()}
            onChange={(event) => setSort(event.currentTarget.value === "name" ? "name" : "items")}
          >
            <option value="items">Most used</option>
            <option value="name">Name</option>
          </select>
        </label>

        <label class="flex items-center gap-2 text-sm text-ink-muted">
          Named by
          <select
            class="field"
            value={origin()}
            onChange={(event) => setOrigin(asOrigin(event.currentTarget.value))}
          >
            <option value="all">Anyone</option>
            <option value="ai">Curio</option>
            <option value="user">You</option>
          </select>
        </label>

        <span class="ml-auto text-sm text-ink-faint">
          <span class="numeric">{shown().length}</span> of{" "}
          <span class="numeric">{props.entries.length}</span>
        </span>
      </div>

      <Show
        when={shown().length}
        fallback={
          <p class="card px-6 py-10 text-center text-sm text-ink-muted">
            {props.entries.length
              ? "Nothing here matches those filters."
              : `No ${props.noun} yet. Curio adds them as it describes captures, and you can add your own above.`}
          </p>
        }
      >
        <ul class="flex flex-col gap-2">
          <For each={shown()}>
            {(entry) => <VocabRow kind={props.kind} entry={entry} targets={targets(entry.id)} />}
          </For>
        </ul>
      </Show>
    </div>
  );
}

function asOrigin(value: string): Origin {
  if (value === "ai" || value === "user") return value;
  return "all";
}
