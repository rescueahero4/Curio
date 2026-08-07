import { createMemo, createSignal, For, Show } from "solid-js";
import { t } from "~/lib/i18n";

/** One choosable vocabulary entry. `count` is shown when the source knows it. */
export interface Option {
  id: string;
  label: string;
  count?: number;
}

/** Above this many entries the list gets its own filter box; below it, one is clutter. */
const FILTER_FROM = 8;

/**
 * Turning the filter box into a create box as well.
 *
 * `noun` is the singular the offer is phrased with — "Aesthetic Family", "Design Type",
 * "Tag" — so one component can serve all three without the caller restating the sentence.
 */
export interface CreateOffer {
  noun: string;
  onCreate: (name: string) => void;
}

/**
 * A multi-select over vocabulary, used by every filter pill, bulk panel and item picker.
 *
 * Checkboxes rather than a listbox: within one facet the choices OR together, and a
 * checkbox is the control that says "several of these at once" without being taught to.
 *
 * ## One box, two jobs
 *
 * When a `create` offer is present this list has exactly one text input, and typing in it
 * does both things at once: it narrows the list, and — if what was typed is not already a
 * name here — it offers to create it, as the first row of the list.
 *
 * The shape it replaces had two inputs: a filter at the top and a separate "New name" +
 * "Add" pair at the bottom. That asked the user to know, before typing, whether the word
 * they had in mind already existed. Getting it wrong meant typing it twice, and the second
 * box could silently create a duplicate of something already in the list above it. One box
 * cannot be used wrongly: search and create are the same gesture, and the offer only
 * appears when there is genuinely nothing to match.
 */
export function OptionList(props: {
  options: Option[];
  selected: string[];
  empty: string;
  onToggle: (id: string) => void;
  /** Present means this list can mint new entries; absent means it is a filter only. */
  create?: CreateOffer;
}) {
  const [needle, setNeedle] = createSignal("");

  const query = () => needle().trim();

  const shown = createMemo(() => {
    const q = query().toLowerCase();
    if (!q) return props.options;
    return props.options.filter((option) => option.label.toLowerCase().includes(q));
  });

  /**
   * Offer to create only what does not already exist.
   *
   * The match is case-insensitive and against the **whole** option set rather than the
   * filtered view: "tag" typed while the list already holds "Tag" is the same word, and
   * offering to create it there would produce a duplicate that only differs in shift key.
   */
  const canCreate = () =>
    !!props.create &&
    query().length > 0 &&
    !props.options.some((option) => option.label.toLowerCase() === query().toLowerCase());

  const create = () => {
    const name = query();
    if (!name || !props.create) return;
    props.create.onCreate(name);
    setNeedle("");
  };

  const isSelected = (id: string) => props.selected.includes(id);

  return (
    <>
      <Show when={props.create || props.options.length >= FILTER_FROM}>
        <input
          type="search"
          class="field field-block"
          placeholder={
            props.create ? t("library.options.filterOrCreate") : t("library.options.filter")
          }
          aria-label={
            props.create
              ? t("library.options.filterOrCreateLabel", { noun: props.create.noun })
              : t("library.options.filter")
          }
          value={needle()}
          onInput={(event) => setNeedle(event.currentTarget.value)}
          onKeyDown={(event) => {
            // Enter is what the removed "Add" button used to be. Only bound when there is
            // something to create, so it never swallows a keystroke that meant nothing.
            if (event.key !== "Enter" || !canCreate()) return;
            event.preventDefault();
            create();
          }}
        />
      </Show>

      <Show
        when={shown().length || canCreate()}
        fallback={
          <p class="px-1 py-2 text-xs text-ink-faint">
            {props.options.length ? t("library.options.noMatch") : props.empty}
          </p>
        }
      >
        {/* Height is handed down by the popover, which is the only thing that knows how much
            room is left between the trigger and the edge of the screen. */}
        <ul class="option-scroll">
          <Show when={canCreate()}>
            <li>
              <button
                type="button"
                class="flex w-full cursor-pointer items-center gap-2 rounded px-1 py-1 text-left text-sm hover:bg-desk"
                onClick={create}
              >
                <span class="text-ink-faint" aria-hidden="true">
                  +
                </span>
                {/* One string, not an offer with the typed name tacked on the end: the name
                    sits mid-sentence in Japanese —「「warm」を新しいタグとして作成」— and a
                    trailing span cannot move there. The faint treatment on the name went with
                    it; a row that is already the only one of its kind does not need it. */}
                <span class="min-w-0 flex-1 truncate">
                  {t("library.options.create", {
                    noun: props.create?.noun ?? "",
                    name: query(),
                  })}
                </span>
              </button>
            </li>
          </Show>

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
