/**
 * The two numbers that decide when Curio is sure, unsure, or wrong.
 *
 * The blurb deliberately no longer spells out the three bands or the lower ≤ upper rule. The
 * field hints say what each number does at the moment you are editing it, and the ordering
 * rule is enforced on save with a message of its own — so stating it up front only asked the
 * reader to carry a constraint they had not yet had the chance to break.
 */

import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
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
      title="Confidence thresholds"
      saver={saver}
      blurb="How sure Curio has to be before filing something on its own. When it isn't sure enough, it asks you instead."
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label="Lower" hint="Below this, Curio suggests a new family rather than guessing.">
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
              title={paused() ? PAUSED_REASON : undefined}
              {...blurOrEnter(commitBound("lower"))}
            />
          )}
        </Field>

        <Field label="Upper" hint="At or above this, Curio files the item without asking.">
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
              title={paused() ? PAUSED_REASON : undefined}
              {...blurOrEnter(commitBound("upper"))}
            />
          )}
        </Field>
      </div>
    </Section>
  );
}
