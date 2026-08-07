/** The two model slots: one that looks at screenshots, one that does the cheap text work. */

import { type Commit, pausedReason } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
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
      title={t("settings.models.title")}
      saver={saver}
      blurb={t("settings.models.blurb")}
    >
      <div class="grid gap-3 sm:grid-cols-2">
        <Field label={t("settings.models.vision.label")} hint={t("settings.models.vision.hint")}>
          {(id) => (
            <input
              id={id}
              type="text"
              class="field field-block"
              spellcheck={false}
              value={props.settings.models.vision}
              disabled={paused()}
              title={paused() ? pausedReason() : undefined}
              {...blurOrEnter(commitSlot("vision"))}
            />
          )}
        </Field>

        <Field label={t("settings.models.utility.label")} hint={t("settings.models.utility.hint")}>
          {(id) => (
            <input
              id={id}
              type="text"
              class="field field-block"
              spellcheck={false}
              value={props.settings.models.utility}
              disabled={paused()}
              title={paused() ? pausedReason() : undefined}
              {...blurOrEnter(commitSlot("utility"))}
            />
          )}
        </Field>
      </div>
    </Section>
  );
}
