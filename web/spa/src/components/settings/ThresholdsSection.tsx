/**
 * The two numbers that decide when Curio is sure, unsure, or wrong.
 *
 * The blurb deliberately no longer spells out the three bands or the lower ≤ upper rule. The
 * field hints say what each number does at the moment you are editing it, and the ordering
 * rule is enforced on save with a message of its own — so stating it up front only asked the
 * reader to carry a constraint they had not yet had the chance to break.
 */

import { type Commit, pausedReason } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Settings, Thresholds } from "~/lib/types";

const STEP = 0.05;

export function ThresholdsSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function commitBound(bound: keyof Thresholds) {
    return (input: HTMLInputElement) => {
      const next = Number.parseFloat(input.value);
      const previous = props.settings.thresholds;
      if (Number.isNaN(next) || next === previous[bound]) return;
      void saver.save({ thresholds: { ...previous, [bound]: next } }, { thresholds: previous });
    };
  }

  return (
    <Section
      id="thresholds"
      title={t("settings.thresholds.title")}
      saver={saver}
      blurb={t("settings.thresholds.blurb")}
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Field
          label={t("settings.thresholds.lower.label")}
          hint={t("settings.thresholds.lower.hint")}
        >
          {(id) => (
            <input
              id={id}
              type="number"
              class="field field-block numeric"
              min={0}
              max={1}
              step={STEP}
              value={props.settings.thresholds.lower}
              disabled={paused()}
              title={paused() ? pausedReason() : undefined}
              {...blurOrEnter(commitBound("lower"))}
            />
          )}
        </Field>

        <Field
          label={t("settings.thresholds.upper.label")}
          hint={t("settings.thresholds.upper.hint")}
        >
          {(id) => (
            <input
              id={id}
              type="number"
              class="field field-block numeric"
              min={0}
              max={1}
              step={STEP}
              value={props.settings.thresholds.upper}
              disabled={paused()}
              title={paused() ? pausedReason() : undefined}
              {...blurOrEnter(commitBound("upper"))}
            />
          )}
        </Field>
      </div>
    </Section>
  );
}
