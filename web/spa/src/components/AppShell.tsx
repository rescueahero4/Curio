import { A } from "@solidjs/router";
import type { ParentProps } from "solid-js";

/**
 * The frame every route renders inside: navigation, and (from P3) the centred search and
 * the Add Item dialog.
 *
 * The tray deliberately does not duplicate this. Navigation lives here, where it can show
 * the user what they are navigating to; the tray is a switch (D14).
 */
export function AppShell(props: ParentProps) {
  return (
    <div class="min-h-screen">
      <header class="border-b" style={{ "border-color": "var(--color-line)" }}>
        <nav class="mx-auto flex max-w-6xl items-center gap-6 px-6 py-4 text-sm">
          <A href="/" class="font-semibold">
            Curio
          </A>
          <A href="/projects">Projects</A>
          <A href="/prompts">Prompts</A>
          <A href="/vocabulary">Vocabulary</A>
          <A href="/settings" class="ml-auto">
            Settings
          </A>
        </nav>
      </header>
      <main class="mx-auto max-w-6xl px-6 py-10">{props.children}</main>
    </div>
  );
}
