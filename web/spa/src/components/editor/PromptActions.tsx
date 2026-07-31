/**
 * The two ways a prompt leaves Curio.
 *
 * Both are disabled together while either is running: they share a serialize step and a
 * clipboard, and two of them racing would leave the user unable to say which text they
 * are holding. The disabled state carries its reason, per PRD §5 — no dead clicks.
 *
 * Whatever the launch answers is shown **verbatim**. The server is the only party that
 * knows what it managed to ask for, and its phrasing is deliberate: "Asked Claude Code to
 * open — paste there", never "opened" (R-FE-15).
 */

import { createSignal, Show } from "solid-js";
import type { ActionOutcome } from "~/components/editor/handoff";
import { copyPrompt, sendPromptToClaude } from "~/components/editor/handoff";
import type { Prompt } from "~/lib/types";

interface Props {
  id: string;
  /** Flushes any unsaved edit, so the server serializes what is on screen. */
  beforeSend: () => Promise<void>;
  onSent: (prompt: Prompt) => void;
}

export function PromptActions(props: Props) {
  const [busy, setBusy] = createSignal(false);
  const [outcome, setOutcome] = createSignal<ActionOutcome | null>(null);

  const run = async (action: (id: string) => Promise<ActionOutcome>) => {
    setBusy(true);
    setOutcome(null);
    try {
      await props.beforeSend();
      const result = await action(props.id);
      setOutcome(result);
      if (result.prompt) props.onSent(result.prompt);
    } catch (error) {
      setOutcome({ message: message(error), tone: "caution", prompt: null });
    } finally {
      setBusy(false);
    }
  };

  const reason = () => (busy() ? "Curio is working on the last request" : undefined);

  return (
    <div class="flex flex-col items-end gap-2">
      <div class="flex items-center gap-2">
        <button
          type="button"
          class="pill pill-outline"
          disabled={busy()}
          title={reason() ?? "Serialize, copy, and start watching for a project folder"}
          onClick={() => void run(copyPrompt)}
        >
          Copy Prompt
        </button>
        <button
          type="button"
          class="pill pill-ink"
          disabled={busy()}
          title={reason() ?? "Copy first, then ask the configured tool to open"}
          onClick={() => void run(sendPromptToClaude)}
        >
          Send to Claude
        </button>
      </div>

      <Show when={outcome()}>
        {(result) => (
          <output
            class="banner"
            classList={{
              "tint-confirm": result().tone === "confirm",
              "tint-caution": result().tone === "caution",
            }}
          >
            {result().message}
          </output>
        )}
      </Show>
    </div>
  );
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : "Something went wrong.";
}
