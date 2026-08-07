/** Where the library lives, and where Curio looks for the folders your tools write. */

import { type Commit, pausedReason } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Settings } from "~/lib/types";

export function PathsSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function commitRoot(input: HTMLInputElement) {
    const next = input.value.trim();
    const previous = props.settings.projects_root;
    if (!next || next === previous) return;
    void saver.save({ projects_root: next }, { projects_root: previous });
  }

  return (
    <Section
      id="paths"
      title={t("settings.paths.title")}
      saver={saver}
      blurb={t("settings.paths.blurb")}
    >
      {/* The hint no longer names CURIO_DATA_ROOT. Moving a library is a file operation
          someone has to perform deliberately, and an env var printed on a settings page reads
          as a step a designer is expected to take. "Not from this page" is the honest and
          useful half; the variable is in the docs for whoever actually needs it. */}
      <Field label={t("settings.paths.dataRoot.label")} hint={t("settings.paths.dataRoot.hint")}>
        {(id) => (
          <p id={id} class="field field-block bg-desk text-ink-muted select-all">
            {props.settings.data_root}
          </p>
        )}
      </Field>

      {/* "(watched)" is not decoration. Without it this reads as "where Curio puts your
          projects", and it is not that: Curio never writes a project folder. An agent does,
          and this is the one directory Curio checks for new ones. The Bun original carried
          the same parenthetical for the same reason. */}
      <Field
        label={t("settings.paths.projectsRoot.label")}
        hint={t("settings.paths.projectsRoot.hint")}
      >
        {(id) => (
          <input
            id={id}
            type="text"
            class="field field-block"
            spellcheck={false}
            value={props.settings.projects_root}
            disabled={paused()}
            title={paused() ? pausedReason() : undefined}
            {...blurOrEnter(commitRoot)}
          />
        )}
      </Field>
    </Section>
  );
}
