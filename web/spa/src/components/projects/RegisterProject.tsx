/**
 * Register a folder by hand.
 *
 * The watcher covers the projects folder; this covers everywhere else. A registered project
 * gets no fingerprint by design, so it is deliberately not rename-followable — said here
 * because the user is the only one who can decide whether that matters.
 */

import { createSignal, Show } from "solid-js";
import { registerProject } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { t } from "~/lib/i18n";
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
            ? t("projects.paused")
            : failure.message
          : t("projects.register.failed"),
      );
    } finally {
      setSaving(false);
    }
  }

  return (
    <form class="card flex flex-col gap-3 p-4" onSubmit={(event) => void submit(event)}>
      <h2 class="text-lg font-semibold">{t("projects.register.title")}</h2>
      <p class="max-w-prose text-sm text-ink-muted">{t("projects.register.blurb")}</p>

      <label class="flex flex-col gap-1">
        <span class="text-sm font-medium">{t("projects.register.path")}</span>
        <input
          type="text"
          class="field field-block"
          required
          spellcheck={false}
          // A path, not a sentence — it stays as it is in both languages, like the paths on
          // the cards above.
          placeholder="/Users/you/Projects/landing-page"
          value={path()}
          onInput={(event) => setPath(event.currentTarget.value)}
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm font-medium">{t("projects.register.name")}</span>
        <input
          type="text"
          class="field field-block"
          placeholder={t("projects.register.namePlaceholder")}
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
            paused()
              ? t("projects.paused")
              : path().trim()
                ? undefined
                : t("projects.register.needPath")
          }
        >
          {saving() ? t("projects.register.saving") : t("projects.register.submit")}
        </button>
        <button type="button" class="pill pill-outline" onClick={props.onClose}>
          {t("common.cancel")}
        </button>
      </div>
    </form>
  );
}
