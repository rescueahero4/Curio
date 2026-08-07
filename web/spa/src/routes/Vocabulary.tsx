import { createSignal, For, Show } from "solid-js";
import { ensureVocabulary } from "~/components/library/vocab";
import { AddTerm } from "~/components/vocabulary/AddTerm";
import type { VocabEntry } from "~/components/vocabulary/VocabRow";
import { VocabTable } from "~/components/vocabulary/VocabTable";
import { t } from "~/lib/i18n";
import { refreshVocabulary, vocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

/** The tabs in the order they are drawn. The labels are looked up, not stored. */
const KINDS: VocabularyKind[] = ["families", "types", "tags"];

/**
 * A tab's label, resolved where it is drawn.
 *
 * The array above used to carry the words. A module-level array is built once, at import,
 * which is before a reader has switched language and long before they switch it again — the
 * labels would have been English for the rest of the session.
 */
function tabLabel(kind: VocabularyKind): string {
  if (kind === "families") return t("vocabulary.tabs.families");
  if (kind === "types") return t("vocabulary.tabs.types");
  return t("vocabulary.tabs.tags");
}

/**
 * The collection as a noun a sentence can be built around, in both the forms English needs.
 *
 * Handed down rather than looked up in the table, because the table is one component over
 * three collections and the sentences it writes — "Search tags", "Delete 3 families?" — are
 * whole templates with a slot for this.
 */
function nouns(kind: VocabularyKind): { one: string; other: string } {
  if (kind === "families") {
    return { one: t("vocabulary.kinds.families.one"), other: t("vocabulary.kinds.families.other") };
  }
  if (kind === "types") {
    return { one: t("vocabulary.kinds.types.one"), other: t("vocabulary.kinds.types.other") };
  }
  return { one: t("vocabulary.kinds.tags.one"), other: t("vocabulary.kinds.tags.other") };
}

/**
 * The words the library is described in (FR-11, R-FE-15a).
 *
 * A maintenance page, and it says so: renaming, merging and deleting are how a vocabulary
 * that grew one capture at a time becomes one a person would have chosen. It is reached
 * from here and from Settings rather than the top nav (PRD §5) — it is not a place work
 * happens.
 *
 * Families carry a description because that description is doing two jobs: it is the rubric
 * Curio judges a new capture against, and it is what a family chip expands to in a prompt.
 * The UI says both, on the field itself, because a description written for one of those
 * jobs and not the other is how the two drift apart.
 */
export function Vocabulary() {
  const [tab, setTab] = createSignal<VocabularyKind>("families");

  ensureVocabulary();

  const entries = (): VocabEntry[] => {
    switch (tab()) {
      case "families":
        return vocabulary.families.map((family) => ({
          id: family.id,
          name: family.name,
          item_count: family.item_count,
          created_by: family.created_by,
          description: family.description,
        }));
      case "types":
        return vocabulary.design_types;
      case "tags":
        return vocabulary.tags;
    }
  };

  return (
    <section class="flex flex-col gap-4">
      <header class="flex flex-col gap-1">
        <h1 class="text-xl font-semibold">{t("vocabulary.title")}</h1>
        <p class="text-sm text-ink-muted">{t("vocabulary.blurb")}</p>
      </header>

      {/* ConsistencyPass — "these three words look like one thing" — belongs here, above
          the list it suggests changes to. It waits on `POST /api/bulk/dedupe` and
          `GET /api/bulk/dedupe/latest`, which are E7's; neither route exists yet, so no
          control is shown. Its rules are already settled (R-FE-15a): the latest result is
          re-fetched from the server so it survives a reload, merges are applied
          client-side through the same merge endpoint this page already calls, per group
          Merge / Keep both, and nothing auto-applies. */}

      <Show
        when={vocabulary.loaded}
        fallback={
          <p class="flex items-center justify-center gap-2 py-12 text-sm text-ink-faint">
            {t("vocabulary.loading")}
            <button type="button" class="pill" onClick={() => void refreshVocabulary()}>
              {t("common.retry")}
            </button>
          </p>
        }
      >
        <VocabTable
          kind={tab()}
          entries={entries()}
          nouns={nouns(tab())}
          tabs={
            <nav aria-label={t("vocabulary.tabs.label")} class="flex items-center gap-1">
              <For each={KINDS}>
                {(kind) => (
                  <button
                    type="button"
                    class="pill"
                    classList={{ "pill-current": tab() === kind }}
                    aria-current={tab() === kind ? "page" : undefined}
                    onClick={() => setTab(kind)}
                  >
                    {tabLabel(kind)}
                    <span class="numeric text-2xs">{count(kind)}</span>
                  </button>
                )}
              </For>
            </nav>
          }
          /* Adding is the one control here that is not about the tab you are on, so it
             takes you to the one you added to. A design type minted from the families tab
             would otherwise land somewhere the user cannot see, and the only feedback would
             be a count going up on a pill they were not looking at. */
          add={<AddTerm onAdded={setTab} />}
        />
      </Show>
    </section>
  );
}

function count(kind: VocabularyKind): number {
  if (kind === "families") return vocabulary.families.length;
  if (kind === "types") return vocabulary.design_types.length;
  return vocabulary.tags.length;
}
