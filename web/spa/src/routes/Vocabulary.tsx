import { createSignal, For, Show } from "solid-js";
import { ensureVocabulary } from "~/components/library/vocab";
import { AddTerm } from "~/components/vocabulary/AddTerm";
import type { VocabEntry } from "~/components/vocabulary/VocabRow";
import { VocabTable } from "~/components/vocabulary/VocabTable";
import { refreshVocabulary, vocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

const TABS: { kind: VocabularyKind; label: string }[] = [
  { kind: "families", label: "Families" },
  { kind: "types", label: "Design types" },
  { kind: "tags", label: "Tags" },
];

const PLURAL: Record<VocabularyKind, string> = {
  families: "families",
  types: "design types",
  tags: "tags",
};

const SINGULAR: Record<VocabularyKind, string> = {
  families: "family",
  types: "design type",
  tags: "tag",
};

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
        <h1 class="text-xl font-semibold">Vocabulary</h1>
        <p class="text-sm text-ink-muted">
          Every name Curio uses to describe your library. Rename one and every item follows; merge
          two when they turned out to mean the same thing.
        </p>
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
            Reading the vocabulary…
            <button type="button" class="pill" onClick={() => void refreshVocabulary()}>
              Try again
            </button>
          </p>
        }
      >
        <VocabTable
          kind={tab()}
          entries={entries()}
          noun={PLURAL[tab()]}
          one={SINGULAR[tab()]}
          tabs={
            <nav aria-label="Vocabulary collections" class="flex items-center gap-1">
              <For each={TABS}>
                {(entry) => (
                  <button
                    type="button"
                    class="pill"
                    classList={{ "pill-current": tab() === entry.kind }}
                    aria-current={tab() === entry.kind ? "page" : undefined}
                    onClick={() => setTab(entry.kind)}
                  >
                    {entry.label}
                    <span class="numeric text-2xs">{count(entry.kind)}</span>
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
