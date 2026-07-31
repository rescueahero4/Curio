import { A } from "@solidjs/router";
import { For } from "solid-js";

/**
 * The three top-level destinations, and only three.
 *
 * Vocabulary is deliberately absent (PRD §5, Inventory §6): it is reached from the Library
 * and from Settings, where the user is already thinking about the words. Promoting it here
 * would put a maintenance screen next to the three places work happens.
 */
const DESTINATIONS = [
  { href: "/", label: "Library", end: true },
  { href: "/projects", label: "Projects", end: false },
  { href: "/prompts", label: "Prompts", end: false },
] as const;

export function NavPills() {
  return (
    <nav aria-label="Sections" class="flex items-center gap-1">
      <For each={DESTINATIONS}>
        {(destination) => (
          <A
            href={destination.href}
            end={destination.end}
            class="pill"
            activeClass="pill-current"
            inactiveClass=""
          >
            {destination.label}
          </A>
        )}
      </For>
    </nav>
  );
}
