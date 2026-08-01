/** Where the library lives, and where Curio looks for the folders your tools write. */

import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
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
      title="Paths"
      saver={saver}
      blurb="Where Curio keeps data, and where it watches for projects."
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

      {/* "(watched)" is not decoration. Without it this reads as "where Curio puts your
          projects", and it is not that: Curio never writes a project folder. An agent does,
          and this is the one directory Curio checks for new ones. The Bun original carried
          the same parenthetical for the same reason. */}
      <Field
        label="Projects root (watched)"
        hint="The folder Curio watches. Any new top-level folder inside it becomes a project within about five seconds — Curio never creates one itself. Saves when you press Enter or click away, and it has to be a folder that already exists. Takes effect on the next restart."
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
            {...blurOrEnter(commitRoot)}
          />
        )}
      </Field>
    </Section>
  );
}
