/** A config snippet with a copy button that reports what actually happened. */

import { createSignal, Show } from "solid-js";

export function CopyBlock(props: { label: string; hint?: string; snippet: string }) {
  const [note, setNote] = createSignal<string | null>(null);

  async function copy() {
    try {
      await navigator.clipboard.writeText(props.snippet);
      setNote("Copied.");
    } catch {
      // The clipboard API is permission-gated and can simply refuse. Claiming a copy that
      // did not happen sends the user to paste nothing into a config file.
      setNote("Your browser would not let Curio use the clipboard. Select the text and copy it.");
    }
  }

  return (
    <div class="flex flex-col gap-2">
      <div class="flex flex-wrap items-baseline justify-between gap-2">
        <span class="text-sm font-medium">{props.label}</span>
        <button type="button" class="pill pill-outline" onClick={() => void copy()}>
          Copy
        </button>
      </div>
      <Show when={props.hint}>
        <p class="max-w-prose text-xs text-ink-faint">{props.hint}</p>
      </Show>
      <pre class="card overflow-x-auto bg-desk p-3 font-mono text-xs select-all">
        {props.snippet}
      </pre>
      <Show when={note()}>
        {(message) => (
          <span role="status" class="text-xs text-ink-muted">
            {message()}
          </span>
        )}
      </Show>
    </div>
  );
}
