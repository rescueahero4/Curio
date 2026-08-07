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
      blurb="Where your library lives, and where Curio looks for new projects."
    >
      {/* The hint no longer names CURIO_DATA_ROOT. Moving a library is a file operation
          someone has to perform deliberately, and an env var printed on a settings page reads
          as a step a designer is expected to take. "Not from this page" is the honest and
          useful half; the variable is in the docs for whoever actually needs it. */}
      <Field
        label="Data root"
        hint="Your library, notes and prompts all live here. This can't be moved from inside the app."
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
        hint="Curio watches this folder. Add a folder inside it and it becomes a project within a few seconds. Press Enter to save. The folder has to exist already, and Curio needs a restart to start watching a new one."
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
