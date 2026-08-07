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

/**
 * What became of the last press. Kinds rather than sentences, so the note follows a language
 * change — it has no timer and stays until the button is pressed again.
 *
 * `declined` is the one that keeps the server's wording. When the OS refuses to open the
 * file the reason is the whole message, and it is not something a dictionary here could
 * anticipate. The success path is ours: it was the server's English sentence until now,
 * which meant a Japanese reader got English whenever the button worked.
 */
type Note =
  | { kind: "asked" }
  | { kind: "declined"; message: string }
  | { kind: "paused" | "failed" };

export function RubricSection(props: { settings: Settings }) {
  const [note, setNote] = createSignal<Note | null>(null);
  const [asking, setAsking] = createSignal(false);

  const noteText = (current: Note) => {
    switch (current.kind) {
      case "asked":
        return t("settings.rubric.asked", { path: props.settings.skill_file_path });
      case "declined":
        return current.message;
      case "paused":
        return t("settings.rubric.paused");
      default:
        return t("settings.rubric.failed");
    }
  };

  async function open() {
    setAsking(true);
    setNote(null);
    try {
      const outcome = (await openSkillFile()) as Outcome;
      // `asked` was declared and never read. It is the difference between "we handed this to
      // the OS" and "the OS would not take it", and only the second needs the server's own
      // words — which is what let the first one go back into the dictionary.
      setNote(outcome.asked ? { kind: "asked" } : { kind: "declined", message: outcome.message });
    } catch (error) {
      setNote({ kind: error instanceof ApiError && error.isPaused ? "paused" : "failed" });
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
          {(current) => (
            <span role="status" class="text-xs text-ink-muted">
              {noteText(current())}
            </span>
          )}
        </Show>
      </div>
    </Section>
  );
}
