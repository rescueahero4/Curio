/** The assessment rubric — a markdown file the user owns and Curio reads. */

import { createSignal, Show } from "solid-js";
import { PAUSED_REASON } from "~/components/settings/model";
import { Section } from "~/components/settings/section";
import { openSkillFile } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import type { Settings } from "~/lib/types";

/** The system route answers 200 with its outcome in the body, never a 4xx (Inventory §10.22). */
interface Outcome {
  asked: boolean;
  message: string;
}

export function RubricSection(props: { settings: Settings }) {
  const [note, setNote] = createSignal<string | null>(null);
  const [asking, setAsking] = createSignal(false);

  async function open() {
    setAsking(true);
    setNote(null);
    try {
      const outcome = (await openSkillFile()) as Outcome;
      // Verbatim. The server phrases it as a request because that is all it can honestly
      // claim — rewording it here would turn "asked" into "opened".
      setNote(outcome.message);
    } catch (error) {
      setNote(
        error instanceof ApiError && error.isPaused
          ? "Curio is paused, so it did not ask your editor to open. Resume from the tray icon."
          : "Curio could not reach your editor.",
      );
    } finally {
      setAsking(false);
    }
  }

  return (
    <Section
      title="Assessment rubric"
      blurb="The instructions Curio sends with every screenshot. It is written once, on first run, and never overwritten — so anything you change here stays changed, through updates and restarts."
    >
      <p class="field field-block bg-desk font-mono text-xs text-ink-muted select-all">
        {props.settings.skill_file_path}
      </p>

      <div class="flex flex-wrap items-center gap-2">
        <button
          type="button"
          class="pill pill-outline"
          onClick={() => void open()}
          disabled={asking() || paused()}
          title={paused() ? PAUSED_REASON : undefined}
        >
          {asking() ? "Asking…" : "Open the rubric"}
        </button>
        <Show when={note()}>
          {(message) => (
            <span role="status" class="text-xs text-ink-muted">
              {message()}
            </span>
          )}
        </Show>
      </div>
    </Section>
  );
}
