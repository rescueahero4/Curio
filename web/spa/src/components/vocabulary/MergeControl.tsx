import { createSignal, For, Show } from "solid-js";
import { t } from "~/lib/i18n";

/** One thing this entry could be merged into. */
export interface MergeTarget {
  id: string;
  name: string;
}

/**
 * Fold one name into another.
 *
 * Merge is the answer to the collision a rename cannot resolve: two words for one idea.
 * Every item linked to this entry ends up linked to the target — families keep the higher
 * score and the settled gray zone (Inventory §10.14) — and this entry goes.
 *
 * Confirmation names both sides, because the direction is the part a user can get wrong and
 * the part they cannot undo.
 */
export function MergeControl(props: {
  name: string;
  targets: MergeTarget[];
  blocked?: string;
  onMerge: (into: string) => void;
}) {
  const [into, setInto] = createSignal("");

  const chosen = () => props.targets.find((target) => target.id === into());

  return (
    <div class="flex flex-wrap items-center gap-2">
      <label class="flex items-center gap-2 text-sm">
        {/* English hangs a preposition over the dropdown and lets it point at what follows.
            Japanese marks the role on the noun instead — 統合先, "what this is merged into" —
            which is why this label is a phrase in one language and a noun in the other. */}
        <span class="text-ink-muted">{t("vocabulary.merge.into")}</span>
        <select
          class="field"
          value={into()}
          disabled={!!props.blocked || props.targets.length === 0}
          title={props.targets.length === 0 ? t("vocabulary.merge.empty") : props.blocked}
          onChange={(event) => setInto(event.currentTarget.value)}
        >
          <option value="">{t("vocabulary.merge.choose")}</option>
          <For each={props.targets}>
            {(target) => <option value={target.id}>{target.name}</option>}
          </For>
        </select>
      </label>

      <Show when={chosen()}>
        {(target) => (
          <>
            <button
              type="button"
              class="pill pill-ink"
              onClick={() => {
                props.onMerge(target().id);
                setInto("");
              }}
            >
              {t("vocabulary.merge.action", { name: props.name, target: target().name })}
            </button>
            <span class="text-xs text-ink-faint">
              {t("vocabulary.merge.hint", { name: props.name })}
            </span>
          </>
        )}
      </Show>
    </div>
  );
}
