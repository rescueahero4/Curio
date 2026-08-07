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
      blurb="Which AI models Curio uses. Names aren't checked as you type, so a typo shows up later as a review that failed."
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label="Vision" hint="Looks at your screenshots and writes the review.">
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

        <Field label="Utility" hint="Handles the smaller background jobs.">
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
