/** Where the library lives, and where Curio looks for the folders your tools write. */

import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import type { Settings } from "~/lib/types";

export function PathsSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function commitRoot(event: FocusEvent & { currentTarget: HTMLInputElement }) {
    const next = event.currentTarget.value.trim();
    const previous = props.settings.projects_root;
    if (!next || next === previous) return;
    void saver.save({ projects_root: next }, { projects_root: previous });
  }

  return (
    <Section
      title="Paths"
      saver={saver}
      blurb="Curio watches the projects folder and adds any new top-level folder to Projects within about five seconds."
    >
      <Field
        label="Data root"
        hint="Your library, sidecars and prompts. Not editable here on purpose — moving a library is a file operation, not a form field. Set CURIO_DATA_ROOT and restart to change it."
      >
        {(id) => (
          <p id={id} class="field field-block bg-desk text-ink-muted select-all">
            {props.settings.data_root}
          </p>
        )}
      </Field>

      <Field
        label="Projects folder"
        hint="Saves when you click away. It has to be a folder that already exists — Curio checks, and says so if it is not there."
      >
        {(id) => (
          <input
            id={id}
            type="text"
            class="field field-block"
            spellcheck={false}
            value={props.settings.projects_root}
            disabled={paused()}
            title={paused() ? PAUSED_REASON : undefined}
            onBlur={commitRoot}
          />
        )}
      </Field>
    </Section>
  );
}
