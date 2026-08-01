/**
 * Where "Send to Claude" sends a prompt.
 *
 * Every option ends the same way: the prompt is serialized, put on the clipboard, and the
 * claim staked — that part is Curio's and it either happens or it does not. What differs is
 * the last step, which is a *request* to the OS to launch something. Nothing on this machine
 * can find out whether the app came up, so the copy here says "asks" throughout (§10.22).
 *
 * Clipboard is not the degraded choice. It is the right one for anyone pasting into an
 * editor Curio has never heard of, and it is the only option with nothing to go wrong.
 */

import { For, Show } from "solid-js";
import { type Commit, PAUSED_REASON } from "~/components/settings/model";
import { createSaver } from "~/components/settings/save";
import { Section } from "~/components/settings/section";
import { paused } from "~/lib/http";
import type { SendToClaudeTarget, Settings } from "~/lib/types";

const TARGETS: readonly { value: SendToClaudeTarget; label: string; blurb: string }[] = [
  {
    value: "claude-code",
    label: "Claude Code",
    blurb: "Copies the prompt, then asks your terminal to open Claude Code.",
  },
  {
    value: "claude-desktop",
    label: "Claude Desktop",
    blurb: "Copies the prompt, then asks your OS to open the Claude desktop app.",
  },
  {
    value: "clipboard",
    label: "Clipboard only",
    blurb: "Copies the prompt and stops there. Paste it wherever you actually work.",
  },
];

export function SendToClaudeSection(props: { settings: Settings; commit: Commit }) {
  const saver = createSaver(props.commit);

  function choose(target: SendToClaudeTarget) {
    const previous = props.settings.send_to_claude_target;
    if (target === previous) return;
    void saver.save({ send_to_claude_target: target }, { send_to_claude_target: previous });
  }

  return (
    <Section
      id="send-to-claude"
      title="Send to Claude"
      saver={saver}
      blurb="What the Prompt Helper's Send button does. Whichever you pick, the prompt lands on your clipboard first — so a launch that never happens still leaves you holding the text."
    >
      <fieldset class="flex flex-col gap-2" disabled={paused()}>
        <legend class="sr-only">Send to Claude target</legend>
        <For each={TARGETS}>
          {(target) => (
            <label class="flex items-start gap-2 text-sm">
              <input
                type="radio"
                name="send-to-claude-target"
                class="mt-1"
                value={target.value}
                checked={props.settings.send_to_claude_target === target.value}
                onChange={() => choose(target.value)}
              />
              <span class="flex flex-col">
                <span class="font-medium">{target.label}</span>
                <span class="text-xs text-ink-faint">{target.blurb}</span>
              </span>
            </label>
          )}
        </For>
      </fieldset>
      <Show when={paused()}>
        <p class="text-xs text-ink-faint">{PAUSED_REASON}</p>
      </Show>
    </Section>
  );
}
