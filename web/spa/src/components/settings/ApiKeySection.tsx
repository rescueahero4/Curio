/**
 * The Anthropic key: set it, or clear it.
 *
 * The key is write-only. It goes up on a PUT and only `api_key_set` and `api_key_masked`
 * come back, so this section never puts a key into an input's value — not even one it just
 * sent. That also rules out an undo: there is nothing held anywhere to put back.
 */

import { createSignal, Show } from "solid-js";
import { type Commit, pausedReason } from "~/components/settings/model";
import { blurOrEnter, createSaver } from "~/components/settings/save";
import { Field, Section } from "~/components/settings/section";
import { clearApiKey } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Settings } from "~/lib/types";

export function ApiKeySection(props: {
  settings: Settings;
  commit: Commit;
  refresh: () => Promise<void>;
}) {
  const saver = createSaver(props.commit);
  const [clearing, setClearing] = createSignal(false);
  const [note, setNote] = createSignal<string | null>(null);

  function commitKey(input: HTMLInputElement) {
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
      setNote(t("settings.apiKey.cleared"));
    } catch (error) {
      setNote(
        error instanceof ApiError && error.isPaused
          ? t("settings.apiKey.clearPaused")
          : t("settings.apiKey.clearFailed"),
      );
    } finally {
      setClearing(false);
    }
  }

  return (
    <Section
      id="api-key"
      title={t("settings.apiKey.title")}
      saver={saver}
      blurb={t("settings.apiKey.blurb")}
    >
      {/* The masked key is interpolated rather than wrapped in a `font-mono` span of its own.
          It used to sit mid-sentence in a span, which only works while the sentence is
          English — Japanese puts the value in a different place, and a fragment either side
          of it cannot be translated on its own. The whole clause is one key now, and the
          eight characters of `sk-ant-…` read perfectly well in the body face. */}
      <p class="text-sm">
        <Show when={props.settings.api_key_set} fallback={t("settings.apiKey.none")}>
          {t("settings.apiKey.set", { key: props.settings.api_key_masked ?? "sk-ant-…" })}
        </Show>
      </p>

      <Field
        label={props.settings.api_key_set ? t("settings.apiKey.replace") : t("settings.apiKey.add")}
        hint={t("settings.apiKey.hint")}
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
            title={paused() ? pausedReason() : undefined}
            {...blurOrEnter(commitKey)}
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
              ? pausedReason()
              : props.settings.api_key_set
                ? undefined
                : t("settings.apiKey.nothingToClear")
          }
        >
          {clearing() ? t("settings.apiKey.clearing") : t("settings.apiKey.clear")}
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
