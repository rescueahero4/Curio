import { A } from "@solidjs/router";
import { For } from "solid-js";
import { t } from "~/lib/i18n";

/**
 * The three top-level destinations, and only three.
 *
 * A tab strip rather than a row of pills, because these are not three actions — they are
 * the parts of the app, one of which you are currently in. The underline is the whole point
 * of the pattern: it answers "where am I" before the user has read a word.
 *
 * Vocabulary is deliberately absent (PRD §5, Inventory §6): it is reached from the Library
 * filter row and from Settings, where the user is already thinking about the words.
 * Promoting it here would put a maintenance screen next to the three places work happens.
 *
 * `label` holds the dictionary *key*, not the word. This array is module-level and runs
 * once, at import — a `t(...)` evaluated here would freeze whichever language was loaded
 * when the module was first touched, and the tabs would keep their English while the rest
 * of the bar switched. Resolving inside the render puts the call in a reactive scope, where
 * a language change is just another signal read.
 */
const DESTINATIONS = [
  { href: "/", label: "shell.nav.library", end: true },
  { href: "/projects", label: "shell.nav.projects", end: false },
  { href: "/prompts", label: "shell.nav.prompts", end: false },
] as const;

export function NavTabs() {
  return (
    <nav aria-label={t("shell.nav.label")} class="flex items-center gap-4 ml-3">
      <For each={DESTINATIONS}>
        {(destination) => (
          <A
            href={destination.href}
            end={destination.end}
            class="tab py-3 px-1"
            activeClass="tab-current"
            inactiveClass=""
          >
            {t(destination.label)}
          </A>
        )}
      </For>
    </nav>
  );
}
