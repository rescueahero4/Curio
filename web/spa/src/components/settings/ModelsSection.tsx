/** The two model slots: one that looks at screenshots, one that does the cheap text work. */

import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import type { Models, Settings } from "~/lib/types";

export function ModelsSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function commitSlot(slot: keyof Models) {
    return (input: HTMLInputElement) => {
      const next = input.value.trim();
      const previous = props.settings.models;
      if (!next || next === previous[slot]) return;
      void saver.save({ models: { ...previous, [slot]: next } }, { models: previous });
    };
  }

  return (
    <Section
      id="models"
      title="Models"
      saver={saver}
      blurb="Model ids are sent to Anthropic as you type them here. Curio does not check that a name exists — a typo shows up as a failed assessment, not as a rejected save."
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label="Vision" hint="Reads screenshots and writes the assessment.">
          {(id) => (
            <input
              id={id}
              type="text"
              class="field field-block"
              spellcheck={false}
              value={props.settings.models.vision}
              disabled={paused()}
              title={paused() ? PAUSED_REASON : undefined}
              {...blurOrEnter(commitSlot("vision"))}
            />
          )}
        </Field>

        <Field label="Utility" hint="Smaller jobs: vocabulary dedupe and key checks.">
          {(id) => (
            <input
              id={id}
              type="text"
              class="field field-block"
              spellcheck={false}
              value={props.settings.models.utility}
              disabled={paused()}
              title={paused() ? PAUSED_REASON : undefined}
              {...blurOrEnter(commitSlot("utility"))}
            />
          )}
        </Field>
      </div>
    </Section>
  );
}
