import { A } from "@solidjs/router";
import { For } from "solid-js";

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
 */
const DESTINATIONS = [
  { href: "/", label: "Library", end: true },
  { href: "/projects", label: "Projects", end: false },
  { href: "/prompts", label: "Prompts", end: false },
] as const;

export function NavTabs() {
  return (
    <nav aria-label="Sections" class="flex items-center gap-4 ml-3">
      <For each={DESTINATIONS}>
        {(destination) => (
          <A
            href={destination.href}
            end={destination.end}
            class="tab py-3 px-1"
            activeClass="tab-current"
            inactiveClass=""
          >
            {destination.label}
          </A>
        )}
      </For>
    </nav>
  );
}
