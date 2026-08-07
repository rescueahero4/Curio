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
 * ## Reconnect is a button, not a secret
 *
 * `status` deliberately does not probe (R-EXT-8b), so a popup that once decided Curio was
 * gone keeps saying so however many times it is reopened. Only `refresh` clears the stored
 * connection and re-runs the discovery ladder. That recovery used to be reachable solely by
 * clicking the unlabelled status row, which nothing on screen suggested was clickable and
 * no keyboard could reach — so the one action that fixes a dropped connection was invisible
 * while the hint told the user to "try again", meaning the thing that does not work.
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
  reconnect: document.getElementById("reconnect") as HTMLButtonElement | null,
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

/**
 * Show the recovery, or take it away entirely.
 *
 * Hidden and disabled move together on purpose. Hiding alone would leave a button that
 * still answers to a keyboard; disabling alone would leave a dead control on screen in the
 * running state, where there is nothing to recover from.
 */
function setReconnectShown(shown: boolean): void {
  if (!buttons.reconnect) return;
  buttons.reconnect.hidden = !shown;
  buttons.reconnect.disabled = !shown;
}

function say(message: string): void {
  if (hint) hint.textContent = message;
}

function render(connection: Connection | null): void {
  const state = connection?.state ?? "stale";

  if (dot) dot.dataset.state = state === "stale" ? "" : state;

  if (!connection || state === "stale") {
    if (text) text.textContent = "Curio isn't running";
    // Two different situations wear this one label, and the old hint only named the first.
    // FR-22 holds either way: the extension says to launch Curio and never launches it.
    say("Launch Curio if it's closed, or press Reconnect if it's already open.");
    setCaptureEnabled(false);
    setOpenersEnabled(false);
    setReconnectShown(true);
    return;
  }

  if (state === "paused") {
    if (text) text.textContent = "Curio is paused";
    // Stops capture at the source rather than posting into a 503 (R-EXT-11).
    say("Resume Curio from its tray icon to capture.");
    setCaptureEnabled(false);
    // Reads still work while paused (D25), so the openers stay live.
    setOpenersEnabled(Boolean(connection.token));
    // Paused is not lost: the connection is good and re-running the ladder would not
    // unpause anything.
    setReconnectShown(false);
    return;
  }

  if (!connection.token) {
    // Found by the probe, but with no credential: the address is known and the token is
    // not (R-EXT-8b).
    if (text) text.textContent = "Curio needs pairing";
    // Names the page rather than a route into it. Settings used to carry a link and no
    // longer does — the state is only reachable behind a pinned port, which makes it a
    // development configuration, so the entry point belongs in the extension's README
    // rather than on an end-user settings page.
    //
    // Reconnect stays offered because pairing happens in Curio and nothing here notices it
    // happened; it is how the popup asks again. It cannot resolve this state on its own —
    // the ladder has no rung that mints a token without native messaging.
    say("Open Curio's /pair page to authorize this browser, then press Reconnect.");
    setCaptureEnabled(false);
    setOpenersEnabled(false);
    setReconnectShown(true);
    return;
  }

  if (text) text.textContent = "Curio is running";
  say("");
  setCaptureEnabled(true);
  setOpenersEnabled(true);
  setReconnectShown(false);
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

/** Clear the stored connection and re-run the discovery ladder. */
async function reconnect(): Promise<void> {
  // Down for the duration. `refresh` clears the connection before it rebuilds one, so a
  // second press would restart the ladder on top of the first and race it.
  if (buttons.reconnect) buttons.reconnect.disabled = true;
  say("Looking for Curio…");

  try {
    const reply = (await chrome.runtime.sendMessage({ type: "refresh" })) as
      | { connection?: Connection | null }
      | undefined;

    // `render` decides the button's state along with everything else, which is what leaves
    // it pressable again when Curio is still down.
    render(reply?.connection ?? null);
  } catch {
    // The worker never answered. Fall back to the offline state rather than leaving a
    // disabled button under a "Looking for Curio…" that never resolves — a dead control
    // below a message that never settles is the exact failure this button was added to
    // remove, and it would be unrecoverable without closing the popup.
    render(null);
  }
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
buttons.reconnect?.addEventListener("click", () => void reconnect());

// The listener that used to live on `.row` is gone rather than kept as a shortcut. It fired
// in every state, so clicking the status line of a perfectly healthy popup threw away a
// working connection to rebuild an identical one. Once the recovery is a labelled button,
// an invisible second copy of it is a hazard and not a convenience.

void chrome.runtime
  .sendMessage({ type: "status" })
  .then((reply: { connection?: Connection | null } | undefined) =>
    render(reply?.connection ?? null),
  );
