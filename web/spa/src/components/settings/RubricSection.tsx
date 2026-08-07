/** The assessment rubric — a markdown file the user owns and Curio reads. */

import { createSignal, Show } from "solid-js";
import { pausedReason } from "~/components/settings/model";
import { Section } from "~/components/settings/section";
import { openSkillFile } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { t } from "~/lib/i18n";
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
      // Verbatim, and therefore in the server's language rather than the reader's. The
      // server phrases it as a request because that is all it can honestly claim — and it
      // names the tool it asked, which is the part that helps. Restating it from a
      // dictionary here would turn "asked" into "opened" and drop the detail.
      setNote(outcome.message);
    } catch (error) {
      setNote(
        error instanceof ApiError && error.isPaused
          ? t("settings.rubric.paused")
          : t("settings.rubric.failed"),
      );
    } finally {
      setAsking(false);
    }
  }

  return (
    <Section
      id="assessment-rubric"
      title={t("settings.rubric.title")}
      blurb={t("settings.rubric.blurb")}
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
          title={paused() ? pausedReason() : undefined}
        >
          {asking() ? t("settings.rubric.opening") : t("settings.rubric.open")}
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
