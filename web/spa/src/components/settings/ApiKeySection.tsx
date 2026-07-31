/**
 * The Anthropic key: set it, or clear it.
 *
 * The key is write-only. It goes up on a PUT and only `api_key_set` and `api_key_masked`
 * come back, so this section never puts a key into an input's value — not even one it just
 * sent. That also rules out an undo: there is nothing held anywhere to put back.
 */

import { createSignal, Show } from "solid-js";
import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { clearApiKey } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import type { Settings } from "~/lib/types";

export function ApiKeySection(props: {
  settings: Settings;
  commit: Commit;
  refresh: () => Promise<void>;
}) {
  const saver = createSaver(props.commit);
  const [clearing, setClearing] = createSignal(false);
  const [note, setNote] = createSignal<string | null>(null);

  function commitKey(event: FocusEvent & { currentTarget: HTMLInputElement }) {
    const input = event.currentTarget;
    const key = input.value.trim();
    if (!key) return;
    // Cleared before the request resolves: the box is the only place the secret exists in
    // this document, and it has no reason to still be there once it has been handed over.
    input.value = "";
    setNote(null);
    void saver.save({ api_key: key });
  }

  async function clear() {
    setClearing(true);
    setNote(null);
    try {
      await clearApiKey();
      await props.refresh();
      setNote("Key cleared. Assessments will queue until you set a new one.");
    } catch (error) {
      setNote(
        error instanceof ApiError && error.isPaused
          ? "Curio is paused, so the key was not cleared. Resume from the tray icon."
          : "Curio could not clear the key.",
      );
    } finally {
      setClearing(false);
    }
  }

  return (
    <Section
      title="Anthropic API key"
      saver={saver}
      blurb="Curio needs a key to assess captures. Without one, captures still arrive and queue — they are never lost, they just wait."
    >
      <p class="text-sm">
        <Show when={props.settings.api_key_set} fallback="No key is set.">
          A key is set:{" "}
          <span class="numeric font-mono">{props.settings.api_key_masked ?? "sk-ant-…"}</span>
        </Show>
      </p>

      <Field
        label={props.settings.api_key_set ? "Replace the key" : "Set a key"}
        hint="Saves when you click away from the box. Curio stores it in your OS keychain and never shows it again — replacing a key discards the old one for good, so there is nothing to undo."
      >
        {(id) => (
          <input
            id={id}
            type="password"
            class="field field-block"
            autocomplete="off"
            spellcheck={false}
            placeholder="sk-ant-…"
            disabled={paused()}
            title={paused() ? PAUSED_REASON : undefined}
            onBlur={commitKey}
          />
        )}
      </Field>

      <div class="flex flex-wrap items-center gap-2">
        <button
          type="button"
          class="pill pill-outline"
          onClick={() => void clear()}
          disabled={!props.settings.api_key_set || clearing() || paused()}
          title={
            paused()
              ? PAUSED_REASON
              : props.settings.api_key_set
                ? undefined
                : "There is no key to clear."
          }
        >
          {clearing() ? "Clearing…" : "Clear key"}
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
