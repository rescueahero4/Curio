/**
 * The popup.
 *
 * Three rules that look like trivia and are not (R-EXT-17, R-EXT-18, Inventory §10.23):
 *
 * 1. **Register listeners before the first `await`.** The popup script starts running
 *    because the user opened the popup; a listener attached after an await has already
 *    missed the event that woke it.
 * 2. **Buttons are disabled in the HTML**, not by JavaScript afterwards. Between paint and
 *    the first line of script there is a window in which an enabled button can be pressed.
 * 3. **Both capture buttons toggle together.** They share the tab's scroll position, so two
 *    captures at once fight each other.
 *
 * The status dot has three states, not two: green (running), amber (**paused** — a new
 * concept, R-EXT-11), and gray (not running). Gray says "Curio isn't running — launch it"
 * and the extension **never launches it** (FR-22): a capture tool starting an app behind
 * the user's back is a surprise, not a convenience.
 *
 * The Open Projects and New Project buttons request a **one-time nonce** using the held
 * token and open `…?t=<nonce>`, so the tab lands authenticated instead of on the
 * no-session screen (R-EXT-19, R-FE-6a).
 */

import type { Connection } from "../shared/storage";

// Rule 1: before any await.
const dot = document.getElementById("status-dot");
const text = document.getElementById("status-text");
const hint = document.getElementById("hint");

const buttons = {
  fold: document.getElementById("capture-fold") as HTMLButtonElement | null,
  full: document.getElementById("capture-full") as HTMLButtonElement | null,
  projects: document.getElementById("open-projects") as HTMLButtonElement | null,
  newProject: document.getElementById("new-project") as HTMLButtonElement | null,
};

/** How long "Added ✓" stays before the popup closes itself (R-EXT-18). */
const SUCCESS_CLOSE_MS = 900;

/** Rule 3: the two capture buttons move together. */
function setCaptureEnabled(enabled: boolean): void {
  if (buttons.fold) buttons.fold.disabled = !enabled;
  if (buttons.full) buttons.full.disabled = !enabled;
}

function setOpenersEnabled(enabled: boolean): void {
  if (buttons.projects) buttons.projects.disabled = !enabled;
  if (buttons.newProject) buttons.newProject.disabled = !enabled;
}

function say(message: string): void {
  if (hint) hint.textContent = message;
}

function render(connection: Connection | null): void {
  const state = connection?.state ?? "stale";

  if (dot) dot.dataset.state = state === "stale" ? "" : state;

  if (!connection || state === "stale") {
    if (text) text.textContent = "Curio isn't running";
    // FR-22: say so; never start it.
    say("Launch Curio, then try again.");
    setCaptureEnabled(false);
    setOpenersEnabled(false);
    return;
  }

  if (state === "paused") {
    if (text) text.textContent = "Curio is paused";
    // Stops capture at the source rather than posting into a 503 (R-EXT-11).
    say("Resume Curio from its tray icon to capture.");
    setCaptureEnabled(false);
    // Reads still work while paused (D25), so the openers stay live.
    setOpenersEnabled(Boolean(connection.token));
    return;
  }

  if (!connection.token) {
    // Found by the probe, but with no credential: the address is known and the token is
    // not (R-EXT-8b).
    if (text) text.textContent = "Curio needs pairing";
    say("Open Curio's Settings and use the pairing page.");
    setCaptureEnabled(false);
    setOpenersEnabled(false);
    return;
  }

  if (text) text.textContent = "Curio is running";
  say("");
  setCaptureEnabled(true);
  setOpenersEnabled(true);
}

/** What the worker replies to a capture request. */
interface CaptureReply {
  ok: boolean;
  itemId?: string;
  truncated?: boolean;
  error?: string;
}

async function runCapture(mode: "fold" | "full"): Promise<void> {
  // Rule 3 again, at the moment it matters: a second capture during the first would fight
  // it for the tab's scroll position.
  setCaptureEnabled(false);
  say(
    mode === "full" ? "Stitching the full page… don't switch tabs." : "Capturing the visible area…",
  );

  const reply = (await chrome.runtime.sendMessage({ type: "capture", mode })) as
    | CaptureReply
    | undefined;

  if (reply?.ok) {
    say(reply.truncated ? "Added ✓ (very long page — trimmed)" : "Added ✓");
    // The popup closes itself on success: the user's next action is on the page, not here.
    setTimeout(() => window.close(), SUCCESS_CLOSE_MS);
    return;
  }

  say(reply?.error ?? "That didn't work.");
  setCaptureEnabled(true);
}

async function open(target: "projects" | "new-project"): Promise<void> {
  setOpenersEnabled(false);
  const reply = (await chrome.runtime.sendMessage({ type: "open", target })) as
    | { ok: boolean; error?: string }
    | undefined;

  if (reply?.ok) {
    window.close();
    return;
  }
  say(reply?.error ?? "Couldn't open Curio.");
  setOpenersEnabled(true);
}

// Rule 1: every listener attached before the first await, including the worker's progress
// pushes — a capture that starts before this line would have nowhere to report to.
chrome.runtime.onMessage.addListener((message: { type?: string; message?: string }) => {
  if (message?.type === "progress" && typeof message.message === "string") {
    say(message.message);
  }
});

buttons.fold?.addEventListener("click", () => void runCapture("fold"));
buttons.full?.addEventListener("click", () => void runCapture("full"));
buttons.projects?.addEventListener("click", () => void open("projects"));
buttons.newProject?.addEventListener("click", () => void open("new-project"));

// Clicking the status line re-runs the discovery ladder — the recovery for "I just started
// Curio and the popup still says it isn't running".
document.querySelector(".row")?.addEventListener("click", () => {
  say("Looking for Curio…");
  void chrome.runtime
    .sendMessage({ type: "refresh" })
    .then((reply: { connection?: Connection | null } | undefined) =>
      render(reply?.connection ?? null),
    );
});

void chrome.runtime
  .sendMessage({ type: "status" })
  .then((reply: { connection?: Connection | null } | undefined) =>
    render(reply?.connection ?? null),
  );
