import { createSignal, Show } from "solid-js";
import { BulkDelete } from "~/components/library/BulkDelete";
import { type BulkMode, BulkVocabPanel } from "~/components/library/BulkVocabPanel";
import { type BulkOutcome, bulkTarget, runBulk } from "~/components/library/bulk";
import type { Selection } from "~/components/library/selection";
import { familyOptions, listOf, nameOf, termOptions } from "~/components/library/vocab";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { BulkEdit, ItemFilter } from "~/lib/types";

interface Notice {
  tone: "confirm" | "caution";
  text: string;
}

/**
 * The bar that appears when something is selected.
 *
 * Composed rather than written straight through: the panels, the delete confirmation and
 * the request builder each live in their own file (NFR-8). The bar's own job is the
 * lifecycle — what the selection means, what a run left behind, and when the bar goes away.
 *
 * Two rules that are easy to lose. A vocabulary edit **keeps** the selection, because the
 * user is usually about to make a second edit to the same items. A delete clears it, but
 * the `notice` keeps this bar mounted (Inventory §10.26) — a bar that vanishes at the same
 * moment the cards do leaves nowhere for "deleted 40 items" to be said.
 */
export function BulkBar(props: {
  selection: Selection;
  filter: () => ItemFilter;
  narrowed: () => boolean;
}) {
  const [notice, setNotice] = createSignal<Notice | null>(null);
  const [busy, setBusy] = createSignal(false);

  const blocked = () => {
    if (paused()) return t("library.bulk.paused");
    if (busy()) return t("common.applying");
    return undefined;
  };

  async function run(edit: BulkEdit, describe: (changed: number) => string, keep: boolean) {
    setBusy(true);
    setNotice(null);
    const outcome = await runBulk({ ...bulkTarget(props.selection, props.filter()), ...edit });
    setBusy(false);
    setNotice(explain(outcome, describe));
    if (outcome.kind === "done" && !keep) props.selection.clear();
  }

  const vocabEdit =
    (kind: "tags" | "types" | "families") => (mode: BulkMode, chosen: string[], fresh: string) => {
      const values = toValues(kind, chosen, fresh);
      if (!values.length) return;
      void run(edit(kind, mode, values), (changed) => summarize(mode, values, changed), true);
    };

  return (
    <Show when={props.selection.any() || notice()}>
      {/* Frosted and lifted, because this bar is the one surface in the library that is
          genuinely floating: it is pinned over a grid the user is still scrolling and still
          reading. The border that separates a resting card from the ground cannot say that
          on its own — see the shadow token's note in `styles.css`. */}
      <div class="card glass-card float sticky bottom-4 z-20 mx-auto flex w-full max-w-4xl flex-wrap items-center gap-2 px-3 py-2">
        <Show when={props.selection.any()}>
          {/* The count is inside the sentence rather than in a span of its own. English can
              put a styled number in front of "selected"; Japanese counts with 件 and puts
              the whole clause in a different order, and no amount of markup makes one shape
              serve both. The `numeric` face still reaches the digits from out here. */}
          <span class="numeric text-sm text-ink-muted">
            <Show
              when={props.selection.matching()}
              fallback={t("library.bulk.selected", { count: props.selection.count() })}
            >
              {t("library.bulk.allMatching")}
            </Show>
          </span>

          <Show when={!props.selection.matching() && props.narrowed()}>
            <button type="button" class="pill" onClick={props.selection.selectMatching}>
              {t("library.bulk.selectMatching")}
            </button>
          </Show>

          <BulkVocabPanel
            title={t("library.bulk.tags.title")}
            options={termOptions("tags")}
            empty={t("library.bulk.tags.empty")}
            newLabel={t("library.bulk.tags.fresh")}
            blocked={blocked()}
            onApply={vocabEdit("tags")}
          />
          <BulkVocabPanel
            title={t("library.bulk.types.title")}
            options={termOptions("types")}
            empty={t("library.bulk.types.empty")}
            newLabel={t("library.bulk.types.fresh")}
            blocked={blocked()}
            onApply={vocabEdit("types")}
          />
          <BulkVocabPanel
            title={t("library.bulk.families.title")}
            options={familyOptions()}
            empty={t("library.bulk.families.empty")}
            blocked={blocked()}
            onApply={vocabEdit("families")}
          />

          {/* The AI Re-tag popover (augment | replace + an optional instruction) belongs
              here, cheapest action last before Delete. It waits on `POST /api/bulk/retag`,
              which is E7's — no route exists yet, so no control is shown. When it lands it
              is a job, not a synchronous edit: R-FE-12's progress + Cancel + summary
              lifecycle attaches to this bar, merging partial `job.updated` payloads by id
              (Inventory §10.30). */}
          <BulkDelete
            count={props.selection.matching() ? null : props.selection.count()}
            blocked={blocked()}
            onConfirm={() => void run({ delete: true }, deleted, false)}
          />

          <button type="button" class="pill" onClick={props.selection.clear}>
            {t("library.bulk.clearSelection")}
          </button>
        </Show>

        <Show when={notice()}>
          {(current) => (
            <output
              class="banner ml-auto"
              classList={{
                "tint-confirm": current().tone === "confirm",
                "tint-caution": current().tone === "caution",
              }}
            >
              {current().text}
              <button type="button" class="pill" onClick={() => setNotice(null)}>
                {t("common.dismiss")}
              </button>
            </output>
          )}
        </Show>
      </div>
    </Show>
  );
}

/**
 * Tags and types travel as names, families as ids.
 *
 * That asymmetry is the server's, and it is deliberate there: a tag is created on first
 * use, while a family has a description and a rubric behind it and so must already exist.
 */
function toValues(kind: "tags" | "types" | "families", chosen: string[], fresh: string): string[] {
  if (kind === "families") return chosen;
  const single = kind === "tags" ? "tags" : "types";
  const names = chosen.map((id) => nameOf(single, id)).filter((name): name is string => !!name);
  return fresh ? [...names, fresh] : names;
}

function edit(kind: "tags" | "types" | "families", mode: BulkMode, values: string[]): BulkEdit {
  const add = mode === "add";
  switch (kind) {
    case "tags":
      return add ? { add_tags: values } : { remove_tags: values };
    case "types":
      return add ? { add_types: values } : { remove_types: values };
    case "families":
      return add ? { add_families: values } : { remove_families: values };
  }
}

/**
 * What a run left behind, as one sentence.
 *
 * This used to be `${verb} ${values} ${preposition} ${changed} items.` — four fragments in
 * English word order, which is the one shape that cannot be translated: Japanese puts the
 * verb last and marks the direction with a particle on the count, so 「40 件に warm を追加
 * しました」 reorders every piece. Each combination is therefore a whole key, and the code's
 * job here is only to pick which one.
 *
 * The `…One` / `…Other` split is English's plural and nothing more; both Japanese strings are
 * the same sentence, counting in 件. The names are joined by `listOf`, which knows that the
 * separator is a translation too.
 */
function summarize(mode: BulkMode, values: string[], changed: number): string {
  const named = { values: listOf(values), count: changed };
  if (mode === "add") {
    return changed === 1
      ? t("library.bulk.done.addedOne", named)
      : t("library.bulk.done.addedOther", named);
  }
  return changed === 1
    ? t("library.bulk.done.removedOne", named)
    : t("library.bulk.done.removedOther", named);
}

function deleted(changed: number): string {
  return changed === 1
    ? t("library.bulk.done.deletedOne", { count: changed })
    : t("library.bulk.done.deletedOther", { count: changed });
}

function explain(outcome: BulkOutcome, describe: (changed: number) => string): Notice {
  switch (outcome.kind) {
    case "done":
      return { tone: "confirm", text: describe(outcome.changed) };
    case "over-cap":
      return {
        tone: "caution",
        text: t("library.bulk.errors.overCap", {
          matched: outcome.matched,
          limit: outcome.limit,
        }),
      };
    case "error":
      return { tone: "caution", text: outcome.message };
  }
}
