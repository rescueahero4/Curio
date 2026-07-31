/**
 * Settings (E3 S3.4) — nine sections, one file each.
 *
 * The page it replaces was 889 lines of one component, which is the specific outcome NFR-8
 * exists to prevent. So this file is only three things: the fetch, the one write every
 * section shares, and the order the sections appear in. Each section owns its own copy, its
 * own controls, and its own save badge.
 */

import { createResource, Show } from "solid-js";
import { ApiKeySection } from "~/components/settings/ApiKeySection";
import { McpSection } from "~/components/settings/McpSection";
import { ModelsSection } from "~/components/settings/ModelsSection";
import type { Commit } from "~/components/settings/model";
import { PairingSection } from "~/components/settings/PairingSection";
import { PathsSection } from "~/components/settings/PathsSection";
import { RubricSection } from "~/components/settings/RubricSection";
import { SendToClaudeSection } from "~/components/settings/SendToClaudeSection";
import { StartupSection } from "~/components/settings/StartupSection";
import { ThresholdsSection } from "~/components/settings/ThresholdsSection";
import { getSettings, saveSettings } from "~/lib/api";

export function Settings() {
  const [settings, { mutate, refetch }] = createResource(getSettings);

  /**
   * Every section writes through here, and the reply — the server's full projection —
   * becomes the page's state. Nothing is applied optimistically: a value on this page is
   * always one the server has confirmed it holds, which is what lets a save badge say
   * "Saved" and mean it.
   */
  const commit: Commit = async (patch) => {
    mutate(await saveSettings(patch));
  };

  const refresh = async () => {
    await refetch();
  };

  return (
    <section class="flex flex-col gap-4">
      <header class="flex flex-wrap items-baseline justify-between gap-2">
        <h1 class="text-2xl font-semibold">Settings</h1>
        <Show when={settings()}>
          {(current) => (
            <p class="numeric text-sm text-ink-faint">
              Curio {current().version}
              {current().port === null ? " · port chosen at launch" : ` · port ${current().port}`}
            </p>
          )}
        </Show>
      </header>

      <Show when={settings.error}>
        <div class="banner tint-caution">
          <span>Curio could not read your settings.</span>
          <button type="button" class="pill pill-outline" onClick={() => void refresh()}>
            Try again
          </button>
        </div>
      </Show>

      <Show when={settings.loading}>
        <p class="text-sm text-ink-muted">Reading your settings…</p>
      </Show>

      <Show when={settings()}>
        {(current) => (
          <div class="flex flex-col gap-4">
            <PathsSection settings={current()} commit={commit} />
            <PairingSection />
            <ApiKeySection settings={current()} commit={commit} refresh={refresh} />
            <ModelsSection settings={current()} commit={commit} />
            <RubricSection settings={current()} />
            <ThresholdsSection settings={current()} commit={commit} />
            <McpSection settings={current()} commit={commit} />
            <SendToClaudeSection settings={current()} commit={commit} />
            <StartupSection settings={current()} commit={commit} />
          </div>
        )}
      </Show>
    </section>
  );
}
