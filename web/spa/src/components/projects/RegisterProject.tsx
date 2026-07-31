/**
 * Register a folder by hand.
 *
 * The watcher covers the projects folder; this covers everywhere else. A registered project
 * gets no fingerprint by design, so it is deliberately not rename-followable — said here
 * because the user is the only one who can decide whether that matters.
 */

import { createSignal, Show } from "solid-js";
import { PAUSED_REASON } from "~/components/projects/copy";
import { registerProject } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import type { Project } from "~/lib/types";

export function RegisterProject(props: {
  onRegistered: (project: Project) => void;
  onClose: () => void;
}) {
  const [path, setPath] = createSignal("");
  const [name, setName] = createSignal("");
  const [error, setError] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const folder = path().trim();
    if (!folder) return;

    setSaving(true);
    setError(null);
    try {
      props.onRegistered(
        await registerProject({ path: folder, ...(name().trim() ? { name: name().trim() } : {}) }),
      );
      setPath("");
      setName("");
      props.onClose();
    } catch (failure) {
      setError(
        failure instanceof ApiError
          ? failure.isPaused
            ? PAUSED_REASON
            : failure.message
          : "Curio could not register that folder.",
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <form class="card flex flex-col gap-3 p-4" onSubmit={(event) => void submit(event)}>
      <h2 class="text-lg font-semibold">Register a folder</h2>
      <p class="max-w-prose text-sm text-ink-muted">
        Point Curio at a folder anywhere on this machine. It has to exist already — Curio checks the
        path rather than creating it. Folders added this way are not followed if you later rename
        them; only folders Curio finds itself carry a marker file that survives a rename.
      </p>

      <label class="flex flex-col gap-1">
        <span class="text-sm font-medium">Folder path</span>
        <input
          type="text"
          class="field field-block"
          required
          spellcheck={false}
          placeholder="/Users/you/Projects/landing-page"
          value={path()}
          onInput={(event) => setPath(event.currentTarget.value)}
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm font-medium">Name</span>
        <input
          type="text"
          class="field field-block"
          placeholder="Optional — defaults to the folder's own name"
          value={name()}
          onInput={(event) => setName(event.currentTarget.value)}
        />
      </label>

      <Show when={error()}>
        {(message) => (
          <p role="alert" class="banner tint-caution">
            {message()}
          </p>
        )}
      </Show>

      <div class="flex flex-wrap gap-2">
        <button
          type="submit"
          class="pill pill-ink"
          disabled={saving() || paused() || !path().trim()}
          title={
            paused() ? PAUSED_REASON : path().trim() ? undefined : "Enter a folder path first."
          }
        >
          {saving() ? "Registering…" : "Register"}
        </button>
        <button type="button" class="pill pill-outline" onClick={props.onClose}>
          Cancel
        </button>
      </div>
    </form>
  );
}
