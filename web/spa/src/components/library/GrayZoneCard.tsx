import { createSignal, For, Show } from "solid-js";
import { resolveGrayZone } from "~/lib/api";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import { vocabulary } from "~/lib/stores";
import type { GrayZoneAction, Item } from "~/lib/types";

/**
 * The one question the gray zone asks, and the three answers to it (FR-7, R-FE-14).
 *
 * Shown only when there is something to decide: a link inside the threshold band, or a
 * family Curio proposed. "Accept the proposal" appears only when an `ai_proposed` link
 * exists — offering it otherwise would be a button that cannot mean anything.
 *
 * Answering it is one request and one decision. The badge clears because the item comes
 * back `ready`, not because this card hides itself.
 */
export function GrayZoneCard(props: { item: Item; onResolved: (item: Item) => void }) {
  const [busy, setBusy] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);
  const [target, setTarget] = createSignal("");

  const nearest = () => [...props.item.families].sort((left, right) => right.score - left.score)[0];
  const proposal = () => props.item.families.find((family) => family.ai_proposed);
  const undecided = () =>
    props.item.families.some((family) => family.gray_zone) || props.item.status === "needs_review";

  const blocked = () => {
    if (paused()) return t("library.item.grayZone.paused");
    if (busy()) return t("library.item.grayZone.busy");
    return undefined;
  };

  async function decide(action: GrayZoneAction, familyId?: string) {
    setBusy(true);
    setProblem(null);
    try {
      props.onResolved(
        await resolveGrayZone(
          props.item.id,
          familyId ? { action, family_id: familyId } : { action },
        ),
      );
    } catch {
      setProblem(t("library.item.grayZone.problem"));
    }
    setBusy(false);
  }

  return (
    <Show when={undecided()}>
      <section class="banner tint-caution flex-col items-start gap-2">
        {/* A lead and a body, which is a split that survives translation — a heading and the
            sentence under it are two clauses, not a subject and a predicate. The score used
            to sit in a `numeric` span mid-sentence; it is formatted before it goes into the
            string now, because Japanese puts that clause somewhere English does not and no
            markup can follow it there. */}
        <div>
          <strong class="font-semibold">{t("library.item.grayZone.title")}</strong>{" "}
          <Show when={nearest()} fallback={t("library.item.grayZone.unplaced")}>
            {(family) => (
              <span class="numeric">
                {t("library.item.grayZone.near", {
                  name: family().name,
                  score: family().score.toFixed(2),
                })}
              </span>
            )}
          </Show>
        </div>

        <div class="flex flex-wrap items-center gap-2">
          <Show when={nearest()}>
            {(family) => (
              <button
                type="button"
                class="pill pill-ink"
                disabled={!!blocked()}
                title={blocked()}
                onClick={() => void decide("accept")}
              >
                {t("library.item.grayZone.keep", { name: family().name })}
              </button>
            )}
          </Show>

          <Show when={proposal()}>
            {(family) => (
              <button
                type="button"
                class="pill tint-proposal"
                disabled={!!blocked()}
                title={blocked()}
                onClick={() => void decide("accept_proposal")}
              >
                {t("library.item.grayZone.acceptProposal", { name: family().name })}
              </button>
            )}
          </Show>

          <label class="flex items-center gap-2 text-sm">
            <span class="text-ink-muted">{t("library.item.grayZone.moveTo")}</span>
            <select
              class="field"
              value={target()}
              disabled={!!blocked()}
              title={blocked()}
              onChange={(event) => {
                const chosen = event.currentTarget.value;
                setTarget("");
                if (chosen) void decide("reassign", chosen);
              }}
            >
              <option value="">{t("library.item.grayZone.choose")}</option>
              <For each={vocabulary.families}>
                {(family) => <option value={family.id}>{family.name}</option>}
              </For>
            </select>
          </label>
        </div>

        <Show when={problem()}>
          <span class="text-xs">{problem()}</span>
        </Show>
      </section>
    </Show>
  );
}
