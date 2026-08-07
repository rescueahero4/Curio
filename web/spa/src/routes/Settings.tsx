/**
 * Settings (E3 S3.4) — eight sections, one file each.
 *
 * The page it replaces was 889 lines of one component, which is the specific outcome NFR-8
 * exists to prevent. So this file is only four things: the fetch, the one write every
 * section shares, the order the sections appear in, and the table of contents that mirrors
 * that order. Each section owns its own copy, its own controls, and its own save badge.
 *
 * ## Why there is no "Browser extension" section
 *
 * There was one, linking to `/pair`. It was removed because it was a **developer control on
 * an end-user surface**: the state it resolves needs a port pinned through `CURIO_PORT` or a
 * hand-edited `config.json`, neither of which is reachable from this page, and a normally
 * installed extension never reaches that state at all (D11, R-EXT-8).
 *
 * `/pair` itself is untouched and still routed — it is the only channel that can hand the
 * extension a token when native messaging is absent. What went away is advertising it to
 * people who cannot use it. The conditions, and the one command that avoids all of them,
 * are documented in `web/extension/README.md` under "Development installs".
 */

import { createResource, For, Show } from "solid-js";
import { ApiKeySection } from "~/components/settings/ApiKeySection";
import { McpSection } from "~/components/settings/McpSection";
import { ModelsSection } from "~/components/settings/ModelsSection";
import type { Commit } from "~/components/settings/model";
import { PathsSection } from "~/components/settings/PathsSection";
import { RubricSection } from "~/components/settings/RubricSection";
import { StartupSection } from "~/components/settings/StartupSection";
import { ThresholdsSection } from "~/components/settings/ThresholdsSection";
import { getSettings, saveSettings } from "~/lib/api";
// Aliased because this route's own component is called `Settings` too. The record is what
// the server holds; the component is the page that edits it.
import type { Settings as SettingsRecord } from "~/lib/types";

/**
 * The left column's table of contents: plain in-page anchors, in the order the sections
 * render below. Every section is still on screen in one scroll, so unlike tabs or an
 * accordion this list cannot hide a setting — it only says where each one is.
 *
 * The ids are the contract between this list and the `Section`s; each section carries
 * `scroll-mt` so the sticky top bar does not cover the heading a jump lands on.
 */
const SECTION_NAV = [
  {
    group: "General",
    items: [
      { id: "paths", label: "Paths" },
      { id: "startup", label: "Startup" },
    ],
  },
  {
    group: "Assessment",
    items: [
      { id: "api-key", label: "Anthropic key" },
      { id: "models", label: "Models" },
      { id: "assessment-rubric", label: "Assessment rubric" },
      { id: "thresholds", label: "Confidence thresholds" },
    ],
  },
  {
    group: "Agents",
    items: [{ id: "mcp", label: "MCP server" }],
  },
];

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
    <section class="mx-auto flex max-w-5xl flex-col gap-4 pb-24">
      <header>
        <h1 class="text-2xl font-semibold">Settings</h1>
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
          <div class="flex items-start gap-10">
            <SectionNav settings={current()} />

            <div class="flex min-w-0 flex-1 flex-col gap-8">
              <PathsSection settings={current()} commit={commit} />
              <StartupSection settings={current()} commit={commit} />
              <ApiKeySection settings={current()} commit={commit} refresh={refresh} />
              <ModelsSection settings={current()} commit={commit} />
              <RubricSection settings={current()} />
              <ThresholdsSection settings={current()} commit={commit} />
              <McpSection settings={current()} commit={commit} />
            </div>
          </div>
        )}
      </Show>
    </section>
  );
}

/**
 * The table of contents. Sticky beneath the app header, and dropped entirely below `lg` —
 * at that width two columns would leave the settings themselves too narrow, and the page
 * already reads top to bottom without it.
 *
 * The build and the port sit at the foot of it. They are not a setting and not a heading —
 * they are what you read out when something is wrong — so they belong at the bottom of the
 * quietest column rather than opposite the page title, where they had the weight of a
 * subtitle and were the first thing the eye landed on after "Settings".
 */
function SectionNav(props: { settings: SettingsRecord }) {
  return (
    <nav
      aria-label="Settings sections"
      class="sticky top-20 hidden w-44 shrink-0 flex-col gap-6 pt-6 lg:flex"
    >
      <ul class="grid gap-6">
        <For each={SECTION_NAV}>
          {(group) => (
            <li>
              <p class="text-xs font-medium text-ink-faint">{group.group}</p>
              <ul class="mt-2 border-l border-line">
                <For each={group.items}>
                  {(item) => (
                    <li>
                      <a
                        href={`#${item.id}`}
                        class="-ml-px block border-l border-transparent py-1 pl-3 text-sm leading-snug text-ink-muted transition-colors hover:border-ink hover:text-ink"
                      >
                        {item.label}
                      </a>
                    </li>
                  )}
                </For>
              </ul>
            </li>
          )}
        </For>
      </ul>

      {/* Two lines, not one: a version and a port are two different facts, and running them
          together behind a separator made a reader parse the string before reading either.

          `bound_port` — the socket — never `port`, which is only the pin in config.json. A
          library with 4321 pinned reported 4321 here no matter where the process actually
          was, and a port that is wrong is worse than no port: it is the number the user
          types into a browser when they are already trying to work out why nothing responds.

          Deliberately not `window.location.port` either, close as that is to the truth. It
          is Vite's port under `npm run dev`, which would make this line lie in exactly the
          setting where a developer is most likely to trust it. */}
      <p class="numeric text-2xs text-ink-faint">
        Curio {props.settings.version}
        <br />
        Port {props.settings.bound_port}
        {/* An ephemeral port is a different number every run, and a user reading this to put
            it somewhere durable — a config file, a bookmark — needs to know that. */}
        <Show when={props.settings.port === null}>
          <br />
          <span>chosen at launch</span>
        </Show>
      </p>
    </nav>
  );
}
