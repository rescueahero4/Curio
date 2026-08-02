import { createMemo, createSignal, For, Show } from "solid-js";
import type { Selection } from "~/components/library/selection";
import type { VocabEntry } from "~/components/vocabulary/VocabRow";
import { deleteTerm, mergeTerm } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { refreshVocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

interface Notice {
  tone: "confirm" | "caution";
  text: string;
}

/** What one pass over the selection left behind. */
interface Outcome {
  done: number;
  refused: { name: string; why: string }[];
}

/**
 * The bar that appears when names are selected.
 *
 * Deleting thirty tags Curio coined and never used again is the maintenance job this page
 * exists for, and one at a time it is ninety clicks. So the two destructive actions the row
 * panel already offers — delete, and merge into another name — are offered here over a
 * whole selection.
 *
 * **These are not atomic, and the bar says so rather than pretending otherwise.** There is
 * no bulk vocabulary endpoint: this is a loop over the same per-term routes a single row
 * calls, run in series so the server sees the same sequence it would from a user working
 * down the list. A failure partway through leaves the earlier terms changed, which is why
 * the result is reported as a count plus every refusal by name — "deleted 28, 2 refused"
 * is recoverable, "something went wrong" is not.
 *
 * Series rather than parallel is deliberate on top of that: each of these rebuilds FTS and
 * the sidecars in-transaction, and thirty of those at once is a thundering herd against a
 * local SQLite file for no gain a user can perceive.
 */
export function VocabBulkBar(props: {
  kind: VocabularyKind;
  entries: VocabEntry[];
  selection: Selection;
  noun: string;
  /** The same noun for one of them. A bar that says "1 families" reads as a bug. */
  one: string;
}) {
  const [notice, setNotice] = createSignal<Notice | null>(null);
  const [asking, setAsking] = createSignal(false);
  const [into, setInto] = createSignal("");
  const [progress, setProgress] = createSignal<number | null>(null);

  const chosen = () => props.selection.picked();

  const counted = (many: number) => (many === 1 ? props.one : props.noun);

  const blocked = () => {
    if (paused()) return "Curio is paused. Resume from the tray icon to edit the vocabulary.";
    if (progress() !== null) return "Working…";
    return undefined;
  };

  const named = (id: string) => props.entries.find((entry) => entry.id === id)?.name ?? id;

  /**
   * Everything that is not itself being merged away.
   *
   * A term cannot be merged into one that is disappearing in the same pass — the second
   * half of the loop would be folding names into an id the first half already deleted — so
   * the selection is removed from its own target list.
   */
  const targets = createMemo(() => {
    const selected = new Set(chosen());
    return props.entries
      .filter((entry) => !selected.has(entry.id))
      .map((entry) => ({ id: entry.id, name: entry.name }))
      .sort((left, right) => left.name.localeCompare(right.name));
  });

  const target = () => targets().find((entry) => entry.id === into());

  async function apply(work: (id: string) => Promise<unknown>, verb: string) {
    const ids = chosen();
    const outcome: Outcome = { done: 0, refused: [] };

    setNotice(null);
    setProgress(0);

    for (const id of ids) {
      try {
        await work(id);
        outcome.done += 1;
      } catch (error) {
        outcome.refused.push({ name: named(id), why: refusal(error) });
      }
      setProgress(outcome.done + outcome.refused.length);
    }

    setProgress(null);
    await refreshVocabulary();
    props.selection.clear();
    setInto("");
    setNotice(explain(outcome, verb, counted(outcome.done)));
  }

  return (
    <Show when={props.selection.any() || notice()}>
      {/* The library's bulk bar, in the same material: frosted and lifted, because it is
          pinned over a list the user is still scrolling. The two bars are the same thing in
          two places and a user who moves between them should not have to learn it twice. */}
      <div class="card glass-card float sticky bottom-4 z-20 mx-auto flex w-full max-w-4xl flex-wrap items-center gap-2 px-3 py-2">
        <Show when={props.selection.any()}>
          <span class="text-sm text-ink-muted">
            <span class="numeric font-medium text-ink">{chosen().length}</span> selected
          </span>

          <Show when={progress() !== null}>
            <span class="text-sm text-ink-faint">
              <span class="numeric">{progress()}</span> of{" "}
              <span class="numeric">{chosen().length}</span>…
            </span>
          </Show>

          <Show when={!asking()}>
            <label class="flex items-center gap-2 text-sm">
              <span class="text-ink-muted">Merge into</span>
              <select
                class="field"
                value={into()}
                disabled={!!blocked() || targets().length === 0}
                title={
                  targets().length === 0 ? "There is nothing left to merge these into." : blocked()
                }
                onChange={(event) => setInto(event.currentTarget.value)}
              >
                <option value="">Choose…</option>
                <For each={targets()}>
                  {(entry) => <option value={entry.id}>{entry.name}</option>}
                </For>
              </select>
            </label>

            <Show when={target()}>
              {(pick) => (
                <button
                  type="button"
                  class="pill pill-ink"
                  disabled={!!blocked()}
                  title={blocked()}
                  onClick={() =>
                    void apply(
                      (id) => mergeTerm(props.kind, id, pick().id),
                      `merged into ${pick().name}`,
                    )
                  }
                >
                  Merge {chosen().length} into {pick().name}
                </button>
              )}
            </Show>
          </Show>

          <Show
            when={asking()}
            fallback={
              <button
                type="button"
                class="pill ml-auto"
                disabled={!!blocked()}
                title={blocked()}
                onClick={() => setAsking(true)}
              >
                Delete
              </button>
            }
          >
            {/* The count is in the question, because it is the part a user can get wrong:
                a selection made three scroll-lengths ago is not on screen when this is
                clicked. The reassurance is the row panel's, word for word — the items are
                not going anywhere, only the word is. */}
            <span class="ml-auto text-ink-muted text-sm">
              Delete {chosen().length} {counted(chosen().length)}? The items stay — they just lose
              {chosen().length === 1 ? " this word" : " these words"}.
            </span>
            <button
              type="button"
              class="pill tint-caution"
              disabled={!!blocked()}
              title={blocked()}
              onClick={() => {
                setAsking(false);
                void apply((id) => deleteTerm(props.kind, id), "deleted");
              }}
            >
              Yes, delete
            </button>
            <button type="button" class="pill" onClick={() => setAsking(false)}>
              Keep them
            </button>
          </Show>

          <button
            type="button"
            class="pill"
            disabled={!!blocked()}
            onClick={() => {
              setAsking(false);
              props.selection.clear();
            }}
          >
            Clear
          </button>
        </Show>

        <Show when={notice()}>
          {(said) => (
            <output
              class="banner w-full"
              classList={{
                "tint-confirm": said().tone === "confirm",
                "tint-caution": said().tone === "caution",
              }}
            >
              {said().text}
            </output>
          )}
        </Show>
      </div>
    </Show>
  );
}

/**
 * What happened, in the order a user needs it.
 *
 * The refusals come first when there are any, and they are named. A partial result is the
 * whole reason this bar reports at all — the count alone would let a user believe a pass
 * finished when a third of it was turned away.
 */
function explain(outcome: Outcome, verb: string, noun: string): Notice {
  const done = `${outcome.done} ${noun} ${verb}`;

  if (!outcome.refused.length) return { tone: "confirm", text: `${done}.` };

  const names = outcome.refused.map((one) => one.name).join(", ");
  const why = new Set(outcome.refused.map((one) => one.why));

  return {
    tone: "caution",
    text: outcome.done
      ? `${done}. ${outcome.refused.length} refused — ${names}. ${[...why].join(" ")}`
      : `Nothing changed. ${outcome.refused.length} refused — ${names}. ${[...why].join(" ")}`,
  };
}

function refusal(error: unknown): string {
  if (!(error instanceof ApiError)) return "That change did not go through.";
  if (error.isPaused) return "Curio is paused. Resume from the tray icon.";
  return error.message;
}
