import { createMemo, createSignal, For, Show } from "solid-js";

/** One choosable vocabulary entry. `count` is shown when the source knows it. */
export interface Option {
  id: string;
  label: string;
  count?: number;
}

/** Above this many entries the list gets its own filter box; below it, one is clutter. */
const FILTER_FROM = 8;

/**
 * A multi-select over vocabulary, used by every filter pill and every bulk panel.
 *
 * Checkboxes rather than a listbox: within one facet the choices OR together, and a
 * checkbox is the control that says "several of these at once" without being taught to.
 */
export function OptionList(props: {
  options: Option[];
  selected: string[];
  empty: string;
  onToggle: (id: string) => void;
}) {
  const [needle, setNeedle] = createSignal("");

  const shown = createMemo(() => {
    const query = needle().trim().toLowerCase();
    if (!query) return props.options;
    return props.options.filter((option) => option.label.toLowerCase().includes(query));
  });

  const isSelected = (id: string) => props.selected.includes(id);

  return (
    <>
      <Show when={props.options.length >= FILTER_FROM}>
        <input
          type="search"
          class="field field-block"
          placeholder="Filter this list"
          value={needle()}
          onInput={(event) => setNeedle(event.currentTarget.value)}
        />
      </Show>

      <Show
        when={shown().length}
        fallback={
          <p class="px-1 py-2 text-xs text-ink-faint">
            {props.options.length ? "Nothing matches that." : props.empty}
          </p>
        }
      >
        <ul class="max-h-64 overflow-y-auto">
          <For each={shown()}>
            {(option) => (
              <li>
                <label class="flex cursor-pointer items-center gap-2 rounded px-1 py-1 text-sm hover:bg-desk">
                  <input
                    type="checkbox"
                    checked={isSelected(option.id)}
                    onChange={() => props.onToggle(option.id)}
                  />
                  <span class="min-w-0 flex-1 truncate">{option.label}</span>
                  <Show when={option.count !== undefined}>
                    <span class="numeric text-2xs text-ink-faint">{option.count}</span>
                  </Show>
                </label>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </>
  );
}
