/**
 * Copy Prompt — serialize → clipboard (R-FE-15, Inventory §10.22).
 *
 * There used to be a second button here called Send to Claude, and Curio never sent
 * anything. It copied the same text to the same clipboard and then spawned a local `claude`
 * binary with no arguments and stdin nulled — the prompt was never transmitted, on argv or
 * otherwise. The handoff was the user pressing Ctrl+V either way, so the button named a
 * channel that did not exist. What its removal costs is one process spawn.
 *
 * ## Why this no longer stakes a claim
 *
 * Copying also used to `markPromptSent`, arming a six-hour claim on the next project folder
 * the watcher adopted. It was removed because the prompt text already tells the model where
 * to put its work: a project built inside the watched projects root is picked up on its own,
 * and one landing anywhere else is registered through the `project_register` MCP tool. The
 * claim was a third answer to a question already answered twice, and it made a copy into a
 * background commitment — one the user had to notice a banner to discover, and press a
 * button to undo.
 *
 * Detection is untouched: the watcher adopts any new top-level folder on its own sweep, and
 * a claim only ever supplied an *optional* prompt id to attribute it to. What is given up is
 * that attribution for watcher-adopted folders.
 *
 * Serialization is the server's throughout. This file sends ids and forwards sentences.
 */

import { serializePrompt } from "~/lib/api";

export interface ActionOutcome {
  message: string;
  tone: "confirm" | "caution";
}

const REFUSED = "The clipboard refused. Nothing was copied.";

export async function copyPrompt(id: string): Promise<ActionOutcome> {
  const { text } = await serializePrompt(id);
  if (!(await copyToClipboard(text))) return { message: REFUSED, tone: "caution" };

  return { message: "Copied to clipboard", tone: "confirm" };
}

async function copyToClipboard(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text);
    return true;
  } catch {
    return false;
  }
}
