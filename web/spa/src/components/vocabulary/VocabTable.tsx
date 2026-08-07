import { createEffect, createMemo, createSignal, For, type JSX, on, Show } from "solid-js";
import { ChevronDown, Search } from "~/components/icons";
import { createSelection } from "~/components/library/selection";
import { compare, createSort, type SortKey } from "~/components/vocabulary/sort";
import { VocabBulkBar } from "~/components/vocabulary/VocabBulkBar";
import { type VocabEntry, VocabRow } from "~/components/vocabulary/VocabRow";
import { t } from "~/lib/i18n";
import type { VocabularyKind } from "~/lib/types";

type Origin = "all" | "ai" | "user";

/**
 * The sortable columns, in the order they are drawn.
 *
 * A function rather than the array it used to be: a module-level array is built at import,
 * so its labels would be whatever language was loaded then and would stay that way through
 * every switch afterwards. Called from inside the `<For>`, the headings track the language.
 */
const columns = (): { key: SortKey; label: string; class?: string }[] => [
  { key: "name", label: t("vocabulary.fields.name") },
  { key: "items", label: t("vocabulary.table.columns.items"), class: "th-end" },
  { key: "origin", label: t("vocabulary.origin.label"), class: "td-origin" },
];

/**
 * One collection, as a table.
 *
 * Sorting by item count is the maintenance view: the words with one item are the ones worth
 * merging away, and the words with two hundred are the ones worth describing well. Filtering
 * by who coined a name is the other half of that — a list of everything Curio invented is
 * exactly the list a user wants to audit.
 *
 * Columns are what make that auditing possible. The same three facts were on the card that
 * came before this, but a fact in a column can be compared with the one above it and a fact
 * on a card cannot, and every question this page answers is a comparison down a list.
 *
 * The header is where the ordering lives now, replacing a `<select>` that offered two of
 * these three orderings and neither of their reverses.
 */
export function VocabTable(props: {
  kind: VocabularyKind;
  entries: VocabEntry[];
  /**
   * What this collection is called, already translated, in both the forms English needs.
   *
   * This used to be a bare `noun` the component pasted in front of and behind — `Search
   * ${noun}`, `No ${noun} yet`. That only works in a language whose word order happens to
   * put the noun where the English does: Japanese wants 「タグ を検索」, with the particle
   * after the noun and the verb at the end, and there is no prefix or suffix that produces
   * it. So the sentences moved into the dictionary as whole templates with a slot, and what
   * crosses this boundary is the noun alone.
   *
   * Both forms because the bulk bar counts with it and a bar reading "1 families" reads as a
   * bug. The Japanese is the same word twice — 件 does the counting.
   */
  nouns: { one: string; other: string };
  /**
   * The collection tabs, and the Add control.
   *
   * Passed in rather than built here because both outlive this component — the tabs decide
   * which collection it is showing, and Add can mint a word in a collection it is not. They
   * are rendered here anyway because they belong on this row: which collection, then how it
   * is being read, then what can be added to it, left to right in one line above the table.
   */
  tabs: JSX.Element;
  add: JSX.Element;
}) {
  const [needle, setNeedle] = createSignal("");
  const [origin, setOrigin] = createSignal<Origin>("all");
  const sort = createSort();

  const shown = createMemo(() => {
    const query = needle().trim().toLowerCase();
    const filtered = props.entries.filter((entry) => {
      if (origin() !== "all" && entry.created_by !== origin()) return false;
      return !query || entry.name.toLowerCase().includes(query);
    });

    return filtered.sort((left, right) => compare(left, right, sort.key(), sort.dir()));
  });

  const order = () => shown().map((entry) => entry.id);
  const selection = createSelection(order, () => props.kind);

  /**
   * A tab is a different collection, not a different view of one.
   *
   * Ids picked among the families mean nothing among the tags, and a bulk delete carrying
   * them across would remove words the user cannot see on the tab they are looking at. The
   * search text and the sort survive the switch, because those are how the user is reading
   * and they were asked for once.
   */
  createEffect(on(() => props.kind, selection.clear, { defer: true }));

  const described = () => props.kind === "families";

  /** Checkbox, name, count, origin, actions — plus description on families. */
  const span = () => (described() ? 6 : 5);

  const picked = createMemo(() => new Set(selection.picked()));

  const allShown = () => shown().length > 0 && shown().every((entry) => picked().has(entry.id));

  const someShown = () => !allShown() && shown().some((entry) => picked().has(entry.id));

  const targets = (id: string) =>
    props.entries
      .filter((entry) => entry.id !== id)
      .map((entry) => ({ id: entry.id, name: entry.name }));

  return (
    <div class="flex flex-col gap-3">
      {/* Which collection on the left, how it is being read on the right. The row wraps
          rather than compressing, so on a narrow window the tabs keep their line and the
          controls drop to their own instead of the search box shrinking to nothing. */}
      <div class="flex flex-wrap items-center gap-2">
        {props.tabs}

        <div class="ml-auto flex flex-wrap items-center gap-2">
          {/* One string, one span. The two numbers used to sit in `numeric` spans of their
              own with the word between them, which is a sentence assembled out of markup —
              and the word does not stay between them in Japanese, where the total comes
              first. Tabular figures on the whole readout do the same job. */}
          <span class="numeric text-sm text-ink-faint">
            {t("vocabulary.table.shown", { shown: shown().length, total: props.entries.length })}
          </span>

          <div class="field-wrap">
            <Search class="field-mark" />
            {/* The glyph carries the meaning on screen and the label carries it everywhere
                else. The placeholder is the short word because the box is now narrow; the
                collection it searches is named by the tab sitting at the other end of this
                same row. */}
            <input
              type="search"
              class="field field-inset w-44"
              placeholder={t("vocabulary.table.search.placeholder")}
              aria-label={t("vocabulary.table.search.label", { noun: props.nouns.other })}
              value={needle()}
              onInput={(event) => setNeedle(event.currentTarget.value)}
            />
          </div>

          <div class="field-wrap">
            <select
              class="field field-select w-auto"
              aria-label={t("vocabulary.origin.label")}
              value={origin()}
              onChange={(event) => setOrigin(asOrigin(event.currentTarget.value))}
            >
              {/* Curio is the product's name and stays Latin in both languages. */}
              <option value="all">{t("vocabulary.origin.anyone")}</option>
              <option value="ai">{t("vocabulary.origin.ai")}</option>
              <option value="user">{t("vocabulary.origin.user")}</option>
            </select>
            {/* Drawn over the select rather than by it, so this is the same glyph at the
                same distance from the same edge as the Add button's. */}
            <ChevronDown class="field-mark field-mark-end" />
          </div>

          {props.add}
        </div>
      </div>

      <Show
        when={shown().length}
        fallback={
          <p class="card px-6 py-10 text-center text-sm text-ink-muted">
            {props.entries.length
              ? t("vocabulary.table.empty.filtered")
              : t("vocabulary.table.empty.none", { noun: props.nouns.other })}
          </p>
        }
      >
        {/* No `overflow: hidden` on the frame, deliberately: a clipped ancestor turns the
            sticky header into one that scrolls away with the rows. The corners are rounded
            on the cells instead — see `styles.css`. */}
        <div class="table-frame">
          <table class="table">
            <thead>
              <tr>
                <th class="th th-check">
                  <label class="flex items-center">
                    <span class="sr-only">
                      {allShown()
                        ? t("vocabulary.clearSelection")
                        : t("vocabulary.table.selectAll", { count: shown().length })}
                    </span>
                    {/* `indeterminate` is a property and not an attribute — there is no
                        markup for it — so it is written onto the node. It is the only thing
                        that distinguishes "some of these" from "none of these" once a user
                        has unticked one row out of forty. */}
                    <input
                      type="checkbox"
                      checked={allShown()}
                      ref={(node) => createEffect(() => (node.indeterminate = someShown()))}
                      onChange={() => (allShown() ? selection.clear() : selection.pick(order()))}
                    />
                  </label>
                </th>

                <For each={columns()}>
                  {(column) => (
                    <th
                      class={`th ${column.class ?? ""}`}
                      aria-sort={sort.ariaSort(column.key)}
                      scope="col"
                    >
                      <button type="button" class="th-sort" onClick={() => sort.toggle(column.key)}>
                        {column.label}
                        <ChevronDown class="sort-mark" />
                      </button>
                    </th>
                  )}
                </For>

                <Show when={described()}>
                  <th class="th td-description" scope="col">
                    {t("vocabulary.fields.description")}
                  </th>
                </Show>

                <th class="th td-actions" scope="col">
                  <span class="sr-only">{t("vocabulary.table.columns.actions")}</span>
                </th>
              </tr>
            </thead>

            <tbody>
              <For each={shown()}>
                {(entry) => (
                  <VocabRow
                    kind={props.kind}
                    entry={entry}
                    targets={targets(entry.id)}
                    span={span()}
                    described={described()}
                    selected={selection.isSelected(entry.id)}
                    onToggle={selection.toggle}
                  />
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>

      <VocabBulkBar
        kind={props.kind}
        entries={props.entries}
        selection={selection}
        nouns={props.nouns}
      />
    </div>
  );
}

function asOrigin(value: string): Origin {
  if (value === "ai" || value === "user") return value;
  return "all";
}
