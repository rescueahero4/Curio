/**
 * Copy Prompt — serialize → clipboard → mark sent (R-FE-15, Inventory §10.22).
 *
 * There used to be a second button here called Send to Claude, and Curio never sent
 * anything. It copied the same text to the same clipboard and then spawned a local `claude`
 * binary with no arguments and stdin nulled — the prompt was never transmitted, on argv or
 * otherwise. The handoff was the user pressing Ctrl+V either way, so the button named a
 * channel that did not exist. What its removal costs is one process spawn.
 *
 * The step that matters is still the one that does **not** happen: if the clipboard
 * refuses, the claim is never staked. A prompt marked sent against an empty clipboard would
 * arm the project watcher for work the user cannot start.
 *
 * Serialization is the server's throughout. This file sends ids and forwards sentences.
 */

import { markPromptSent, serializePrompt } from "~/lib/api";
import type { Prompt } from "~/lib/types";

export interface ActionOutcome {
  message: string;
  tone: "confirm" | "caution";
  /** The prompt as the server left it, when the claim was staked. */
  prompt: Prompt | null;
}

const REFUSED = "The clipboard refused. Nothing was copied and the prompt is unclaimed.";

export async function copyPrompt(id: string): Promise<ActionOutcome> {
  const { text } = await serializePrompt(id);
  if (!(await copyToClipboard(text))) return { message: REFUSED, tone: "caution", prompt: null };

  const prompt = await markPromptSent(id);
  return {
    message: "Copied. Curio is watching for the next project folder.",
    tone: "confirm",
    prompt,
  };
}

async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
