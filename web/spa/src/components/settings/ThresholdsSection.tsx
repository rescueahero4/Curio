/** The two numbers that decide when Curio is sure, unsure, or wrong. */

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
      blurb="Above the upper bound Curio files an item itself; below the lower bound it proposes a new family; between the two is the gray zone, where it files the nearest match and asks you to confirm. The lower bound has to sit at or below the upper one — the server refuses the save otherwise, because inverted bounds would not fail loudly, they would quietly classify everything wrong."
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label="Lower" hint="Under this, Curio proposes a new family instead of guessing.">
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
