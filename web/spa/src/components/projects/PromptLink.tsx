/**
 * The prompt a project came from.
 *
 * This link is the only thing about a project that does not exist on disk, which is why
 * FR-19 keeps a `missing` record rather than deleting it — so the control stays usable on a
 * missing project too.
 *
 * It shows only what the current state needs, and only when there is a link to show: the
 * prompt's name with a way to undo, or a note that the prompt it pointed at is gone. An
 * unlinked project renders nothing here — the watcher's attribution is right often enough
 * that offering to correct it on every card put a form control on a catalogue tile to
 * express a fact that is usually already settled.
 */

import { A } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { PAUSED_REASON } from "~/components/projects/copy";
import { updateProject } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import type { Project, Prompt } from "~/lib/types";

export function PromptLink(props: {
  project: Project;
  prompts: Prompt[];
  onChanged: (project: Project) => void;
}) {
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const linked = () => props.prompts.find((prompt) => prompt.id === props.project.prompt_id);

  async function relink(id: string | null) {
    setSaving(true);
    setError(null);
    try {
      props.onChanged(await updateProject(props.project.id, { prompt_id: id }));
    } catch (failure) {
      setError(
        failure instanceof ApiError && failure.isPaused
          ? PAUSED_REASON
          : "Curio could not change the prompt link.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <>
      <Show when={linked()}>
        {(prompt) => (
          <span class="flex min-w-0 items-center gap-1.5">
            <A
              href={`/prompts/${prompt().id}`}
              class="max-w-[12rem] truncate text-ink-muted underline decoration-line underline-offset-4 hover:text-ink"
              title={`From "${prompt().title || "Untitled prompt"}"`}
            >
              {prompt().title || "Untitled prompt"}
            </A>
            <button
              type="button"
              class="text-ink-faint hover:text-caution"
              title="Unlink this prompt"
              aria-label="Unlink this prompt"
              disabled={saving() || paused()}
              onClick={() => void relink(null)}
            >
              ×
            </button>
          </span>
        )}
      </Show>

      {/* A record pointing at a prompt that is no longer in the list: say so, and offer the
          way out, rather than rendering an empty space where a name should be. */}
      <Show when={props.project.prompt_id && !linked()}>
        <span class="flex items-center gap-1.5 text-ink-faint">
          <span>Prompt deleted</span>
          <button
            type="button"
            class="underline decoration-line underline-offset-4 hover:text-ink"
            disabled={saving() || paused()}
            onClick={() => void relink(null)}
          >
            Clear
          </button>
        </span>
      </Show>

      <Show when={error()}>
        {(message) => (
          <span role="alert" class="text-caution">
            {message()}
          </span>
        )}
      </Show>
    </>
  );
}
