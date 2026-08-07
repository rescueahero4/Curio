import { createMemo, createSignal, For, Show } from "solid-js";
import type { Selection } from "~/components/library/selection";
import { quotedList } from "~/components/library/vocab";
import { refusal } from "~/components/vocabulary/errors";
import type { VocabEntry } from "~/components/vocabulary/VocabRow";
import { deleteTerm, mergeTerm } from "~/lib/api";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import { refreshVocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

/**
 * What the bar has to say, as finished sentences.
 *
 * A list rather than one string, because the last part of it is the server's: the reasons a
 * refusal carries are prose this code did not write and cannot promise ends in a full stop.
 * Joining them with a separator meant choosing a separator, and the Japanese one — nothing,
 * because 。 already closes a sentence — welds two reasons into a single word on the day a
 * message arrives without its terminator. The `.banner` they render into is a flex row with
 * a gap, so the space between them is layout and no dictionary key has to hold it.
 */
interface Notice {
  tone: "confirm" | "caution";
  lines: string[];
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
  /** What this collection is called, translated. See `VocabTable` for why it arrives here. */
  nouns: { one: string; other: string };
}) {
  const [notice, setNotice] = createSignal<Notice | null>(null);
  const [asking, setAsking] = createSignal(false);
  const [into, setInto] = createSignal("");
  const [progress, setProgress] = createSignal<number | null>(null);

  const chosen = () => props.selection.picked();

  const counted = (many: number) => (many === 1 ? props.nouns.one : props.nouns.other);

  const blocked = () => {
    if (paused()) return t("vocabulary.blocked.paused");
    if (progress() !== null) return t("vocabulary.blocked.busy");
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

  /**
   * Run the pass, then say what it left behind.
   *
   * `done` is a function that writes the whole success sentence for a count, rather than the
   * past participle this used to take and paste after a count and a noun. English is happy
   * to be assembled in that order; Japanese puts the verb last, so "28 tags" and "deleted"
   * cannot be two fragments joined at the call site in one language and at the end of the
   * sentence in the other.
   */
  async function apply(work: (id: string) => Promise<unknown>, done: (count: number) => string) {
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
    setNotice(explain(outcome, done));
  }

  return (
    <Show when={props.selection.any() || notice()}>
      {/* The library's bulk bar, in the same material: frosted and lifted, because it is
          pinned over a list the user is still scrolling. The two bars are the same thing in
          two places and a user who moves between them should not have to learn it twice. */}
      <div class="card glass-card float sticky bottom-4 z-20 mx-auto flex w-full max-w-4xl flex-wrap items-center gap-2 px-3 py-2">
        <Show when={props.selection.any()}>
          {/* The count used to be its own bolded span inside the sentence. It cannot be: the
              word that followed it in English follows nothing in Japanese, where the counter
              and the particle come after the number. The emphasis moves to the whole
              readout, which is the bar's headline anyway. */}
          <span class="numeric font-medium text-ink text-sm">
            {t("vocabulary.bulk.selected", { count: chosen().length })}
          </span>

          <Show when={progress() !== null}>
            <span class="numeric text-sm text-ink-faint">
              {t("vocabulary.bulk.progress", { done: progress() ?? 0, total: chosen().length })}
            </span>
          </Show>

          <Show when={!asking()}>
            <label class="flex items-center gap-2 text-sm">
              <span class="text-ink-muted">{t("vocabulary.merge.into")}</span>
              <select
                class="field"
                value={into()}
                disabled={!!blocked() || targets().length === 0}
                title={targets().length === 0 ? t("vocabulary.bulk.mergeEmpty") : blocked()}
                onChange={(event) => setInto(event.currentTarget.value)}
              >
                <option value="">{t("vocabulary.merge.choose")}</option>
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
                  onClick={() => {
                    // Read off the picked target before the pass starts. The `<Show>` above
                    // unmounts the moment the selection is cleared at the end of it, and the
                    // sentence reporting the result still has to name where the words went.
                    const { id, name } = pick();
                    void apply(
                      (term) => mergeTerm(props.kind, term, id),
                      (count) =>
                        t("vocabulary.bulk.result.merged", {
                          count,
                          noun: counted(count),
                          target: name,
                        }),
                    );
                  }}
                >
                  {t("vocabulary.bulk.merge", { count: chosen().length, target: pick().name })}
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
                {t("common.delete")}
              </button>
            }
          >
            {/* The count is in the question, because it is the part a user can get wrong:
                a selection made three scroll-lengths ago is not on screen when this is
                clicked. The reassurance is the row panel's, word for word — the items are
                not going anywhere, only the word is.

                One key or the other rather than a fragment swapped mid-sentence: the only
                thing that changes in English is "this word" / "these words", and that clause
                sits at the end of the English sentence and in the middle of the Japanese
                one. Both keys take the same arguments, so the choice is one expression.

                `w-full` rather than `ml-auto`: the Japanese runs about six hundred pixels
                and wraps, and a wrapped flex item with `ml-auto` right-aligns itself on a
                line of its own — ragged, and only in one language. Taking the whole line
                deliberately is the same shape in both. */}
            <span class="w-full text-ink-muted text-sm">
              {chosen().length === 1
                ? t("vocabulary.bulk.confirmOne", {
                    count: chosen().length,
                    noun: counted(chosen().length),
                  })
                : t("vocabulary.bulk.confirmOther", {
                    count: chosen().length,
                    noun: counted(chosen().length),
                  })}
            </span>
            <button
              type="button"
              class="pill tint-caution"
              disabled={!!blocked()}
              title={blocked()}
              onClick={() => {
                setAsking(false);
                void apply(
                  (id) => deleteTerm(props.kind, id),
                  (count) => t("vocabulary.bulk.result.deleted", { count, noun: counted(count) }),
                );
              }}
            >
              {t("vocabulary.confirmDelete")}
            </button>
            <button type="button" class="pill" onClick={() => setAsking(false)}>
              {t("vocabulary.bulk.keep")}
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
            {t("vocabulary.bulk.clear")}
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
              {/* One node per sentence. `.banner` is a flex row with a gap, so what sits
                  between two of them is the layout's business rather than a string this had
                  to pick — which is the only version of this that is right in both
                  languages and safe against a server message with no full stop. */}
              <For each={said().lines}>{(line) => <span>{line}</span>}</For>
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
 * The outcome comes first, then the refusals, named, then the reasons. A partial result is
 * the whole reason this bar reports at all — the count alone would let a user believe a pass
 * finished when a third of it was turned away.
 */
function explain(outcome: Outcome, done: (count: number) => string): Notice {
  if (!outcome.refused.length) return { tone: "confirm", lines: [done(outcome.done)] };

  /* `quotedList` rather than `listOf`: these names land inside a sentence, and a term is free
     text that can contain the separator the list is joined with. It also settles how the run
     meets the Japanese around it — each name arrives already delimited, so the template needs
     no space on either side and works whichever script the names turn out to be in. */
  const names = quotedList(outcome.refused.map((one) => one.name));
  const why = [...new Set(outcome.refused.map((one) => one.why))];
  const count = outcome.refused.length;

  return {
    tone: "caution",
    lines: outcome.done
      ? [done(outcome.done), t("vocabulary.bulk.result.refused", { count, names }), ...why]
      : [t("vocabulary.bulk.result.nothing", { count, names }), ...why],
  };
}
