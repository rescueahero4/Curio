/**
 * The prompt a project came from.
 *
 * This link is the only thing about a project that does not exist on disk, which is why
 * FR-19 keeps a `missing` record rather than deleting it — so the control stays usable on a
 * missing project too.
 */

import { A } from "@solidjs/router";
import { createSignal, For, Show } from "solid-js";
import { PAUSED_REASON } from "~/components/projects/copy";
import { updateProject } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import type { Project, Prompt } from "~/lib/types";

const UNLINKED = "";

export function PromptLink(props: {
  project: Project;
  prompts: Prompt[];
  onChanged: (project: Project) => void;
}) {
  const [saving, setSaving] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  const linked = () => props.prompts.find((prompt) => prompt.id === props.project.prompt_id);

  async function relink(id: string) {
    setSaving(true);
    setError(null);
    try {
      props.onChanged(await updateProject(props.project.id, { prompt_id: id || null }));
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
    <div class="flex flex-wrap items-center gap-2 text-xs">
      <span class="text-ink-faint">Prompt</span>

      {/* Selection is expressed on the options rather than on the select: a `value` set
          before its options exist silently resolves to nothing. */}
      <select
        class="field w-auto text-xs"
        disabled={saving() || paused()}
        title={paused() ? PAUSED_REASON : undefined}
        onChange={(event) => void relink(event.currentTarget.value)}
      >
        <option value={UNLINKED} selected={!props.project.prompt_id}>
          Not linked
        </option>
        <For each={props.prompts}>
          {(prompt) => (
            <option value={prompt.id} selected={prompt.id === props.project.prompt_id}>
              {prompt.title || "Untitled prompt"}
            </option>
          )}
        </For>
      </select>

      <Show when={linked()}>
        {(prompt) => (
          <A href={`/prompts/${prompt().id}`} class="pill pill-outline">
            Open prompt
          </A>
        )}
      </Show>

      <Show when={props.project.prompt_id && !linked()}>
        <span class="text-ink-faint">
          Linked to a prompt that is no longer in the list — choose another, or Not linked.
        </span>
      </Show>

      <Show when={error()}>
        {(message) => (
          <span role="alert" class="text-caution">
            {message()}
          </span>
        )}
      </Show>
    </div>
  );
}
