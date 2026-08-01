import { createSignal, Show } from "solid-js";
import { ChevronDown } from "~/components/icons";
import { SelectToggle } from "~/components/library/SelectToggle";
import { MergeControl, type MergeTarget } from "~/components/vocabulary/MergeControl";
import { deleteTerm, mergeTerm, updateTerm } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { refreshVocabulary } from "~/lib/stores";
import type { CreatedBy, VocabularyKind } from "~/lib/types";

/** One row's data, whichever collection it came from. */
export interface VocabEntry {
  id: string;
  name: string;
  item_count: number;
  created_by: CreatedBy;
  /** Families only. The rubric and the prompt chips both read it. */
  description?: string;
}

/**
 * One vocabulary entry, and everything that can be done to it.
 *
 * Two `<tr>`s rather than one: the row a reader scans, and — only while it is open — the
 * panel that edits it. Rename, merge and delete stay folded behind one "Edit" toggle for
 * the reason they always were: this page is read far more often than it is edited, and
 * three controls per row on a hundred rows is a wall. What changed is that the row above
 * the panel is now columns, so the answer to "which of these has no description" is
 * available without opening any of them.
 *
 * The panel is a second row spanning every column rather than a drawer or a dialog, because
 * the thing being edited has to stay on screen and in place while it is edited — a rename
 * that collides needs the old name still visible above the message that says so.
 *
 * A rename that collides answers 409 with prose that points at merge, and that prose is
 * shown verbatim — the server knows which name is taken, and paraphrasing it here would
 * mean maintaining the same sentence twice.
 */
export function VocabRow(props: {
  kind: VocabularyKind;
  entry: VocabEntry;
  targets: MergeTarget[];
  /** How many columns the panel below has to span. */
  span: number;
  /** Families carry a description column; types and tags have nothing to put in one. */
  described: boolean;
  selected: boolean;
  onToggle: (id: string, extend: boolean) => void;
}) {
  const [open, setOpen] = createSignal(false);
  const [name, setName] = createSignal(props.entry.name);
  const [description, setDescription] = createSignal(props.entry.description ?? "");
  const [asking, setAsking] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const blocked = () => {
    if (paused()) return "Curio is paused. Resume from the tray icon to edit the vocabulary.";
    if (busy()) return "Working…";
    return undefined;
  };

  async function run(work: () => Promise<unknown>) {
    setBusy(true);
    setProblem(null);
    try {
      await work();
      await refreshVocabulary();
    } catch (error) {
      setProblem(refusal(error));
    }
    setBusy(false);
  }

  return (
    <>
      <tr class="group" classList={{ "tr-selected": props.selected }}>
        <td class="td td-check">
          <SelectToggle
            name={props.entry.name}
            selected={props.selected}
            onToggle={(extend) => props.onToggle(props.entry.id, extend)}
          />
        </td>

        <td class="td font-medium">{props.entry.name}</td>

        {/* Right-aligned and tabular, because the only reason to put a count in a column is
            to compare it with the one above it. */}
        <td class="td numeric text-end text-ink-muted">{props.entry.item_count}</td>

        <td class="td td-origin text-ink-muted">
          {props.entry.created_by === "ai" ? "Curio" : "You"}
        </td>

        <Show when={props.described}>
          {/* An empty description is a fact worth seeing from the list: it is the rubric
              Curio judges new captures against, so a family without one is silently
              matching on its name alone. */}
          <td class="td td-description">
            <Show
              when={props.entry.description?.trim()}
              fallback={<span class="text-ink-faint">No description</span>}
            >
              {(said) => <span class="line-clamp-1 text-ink-muted">{said()}</span>}
            </Show>
          </td>
        </Show>

        <td class="td td-actions">
          <button
            type="button"
            class="pill"
            classList={{ "pill-current": open() }}
            aria-expanded={open()}
            onClick={() => setOpen((was) => !was)}
          >
            Edit
            <ChevronDown class={open() ? "chevron chevron-open" : "chevron"} />
          </button>
        </td>
      </tr>

      <Show when={open()}>
        <tr>
          <td class="td td-panel" colSpan={props.span}>
            <div class="flex flex-col gap-3">
              <form
                class="flex flex-wrap items-end gap-2"
                onSubmit={(event) => {
                  event.preventDefault();
                  const next = name().trim();
                  if (!next || next === props.entry.name) return;
                  void run(() => updateTerm(props.kind, props.entry.id, { name: next }));
                }}
              >
                <label class="flex flex-1 flex-col gap-1 text-sm">
                  <span class="text-ink-muted">Name</span>
                  <input
                    type="text"
                    class="field field-block"
                    value={name()}
                    disabled={!!blocked()}
                    title={blocked()}
                    onInput={(event) => setName(event.currentTarget.value)}
                  />
                </label>
                <button
                  type="submit"
                  class="pill pill-ink"
                  disabled={!!blocked() || !name().trim() || name().trim() === props.entry.name}
                  title={
                    name().trim() === props.entry.name ? "The name has not changed." : blocked()
                  }
                >
                  Rename
                </button>
              </form>

              <Show when={props.entry.description !== undefined}>
                <form
                  class="flex flex-col gap-1 text-sm"
                  onSubmit={(event) => {
                    event.preventDefault();
                    void run(() =>
                      updateTerm(props.kind, props.entry.id, { description: description() }),
                    );
                  }}
                >
                  <span class="text-ink-muted">Description</span>
                  <textarea
                    class="field field-block"
                    rows="3"
                    value={description()}
                    disabled={!!blocked()}
                    title={blocked()}
                    onInput={(event) => setDescription(event.currentTarget.value)}
                  />
                  <span class="text-xs text-ink-faint">
                    Curio reads this when it decides what belongs here, and it is what a family chip
                    expands to in a prompt. Describing the feel is worth more than listing examples.
                  </span>
                  <button type="submit" class="pill pill-outline self-start" disabled={!!blocked()}>
                    Save description
                  </button>
                </form>
              </Show>

              <MergeControl
                name={props.entry.name}
                targets={props.targets}
                blocked={blocked()}
                onMerge={(into) => void run(() => mergeTerm(props.kind, props.entry.id, into))}
              />

              <Show
                when={asking()}
                fallback={
                  <button
                    type="button"
                    class="pill self-start"
                    disabled={!!blocked()}
                    title={blocked()}
                    onClick={() => setAsking(true)}
                  >
                    Delete
                  </button>
                }
              >
                <div class="flex flex-wrap items-center gap-2 text-sm">
                  <span class="text-ink-muted">
                    Delete {props.entry.name}? The {props.entry.item_count} items stay — they just
                    lose this word.
                  </span>
                  <button
                    type="button"
                    class="pill tint-caution"
                    onClick={() => {
                      setAsking(false);
                      void run(() => deleteTerm(props.kind, props.entry.id));
                    }}
                  >
                    Yes, delete
                  </button>
                  <button type="button" class="pill" onClick={() => setAsking(false)}>
                    Keep it
                  </button>
                </div>
              </Show>

              <Show when={problem()}>
                <output class="banner tint-caution">{problem()}</output>
              </Show>
            </div>
          </td>
        </tr>
      </Show>
    </>
  );
}

/**
 * The server's own words.
 *
 * A 409 on a rename means the name is taken, and the server's message names it and points
 * at merge — which is a different action, not a typo to fix, so it is worth saying in full
 * rather than reducing to "that did not work".
 */
function refusal(error: unknown): string {
  if (!(error instanceof ApiError)) return "That change did not go through.";
  if (error.isPaused) return "Curio is paused. Resume from the tray icon.";
  return error.message;
}
