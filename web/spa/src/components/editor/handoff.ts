/**
 * Copy Prompt and Send to Claude — an ordering, not two buttons (R-FE-15, Inventory §10.22).
 *
 * Copy Prompt: serialize → clipboard → mark sent.
 * Send to Claude: serialize → copy → claim → launch.
 *
 * The step that matters is the one that does **not** happen. If the clipboard refuses, the
 * launch is abandoned: an agent that opens with nothing to paste has consumed the user's
 * attention and produced a window they now have to close, and it did so while reporting
 * success. So a clipboard failure ends the sequence, and the claim is never staked.
 *
 * Serialization is the server's throughout. This file sends ids and forwards sentences.
 */

import { markPromptSent, sendToClaude, serializePrompt } from "~/lib/api";
import type { Prompt } from "~/lib/types";

export interface ActionOutcome {
  message: string;
  tone: "confirm" | "caution";
  /** The prompt as the server left it, when the claim was staked. */
  prompt: Prompt | null;
}

const REFUSED = "The clipboard refused. Nothing was copied and nothing was sent.";

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

export async function sendPromptToClaude(id: string): Promise<ActionOutcome> {
  const { text } = await serializePrompt(id);
  if (!(await copyToClipboard(text))) {
    return {
      message: `${REFUSED} Nothing was launched — an agent with an empty clipboard is worse than no agent.`,
      tone: "caution",
      prompt: null,
    };
  }

  const prompt = await markPromptSent(id);

  // The server's sentence, verbatim. It is the one that says "Asked Claude Code to open —
  // paste there" rather than "opened", and paraphrasing it here would put the single claim
  // this system cannot honestly make back into the UI (Inventory §10.22).
  const outcome = await sendToClaude(text);
  return { message: outcome.message, tone: outcome.asked ? "confirm" : "caution", prompt };
}

async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
