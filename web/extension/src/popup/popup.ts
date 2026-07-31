/**
 * The popup.
 *
 * **P5.** Three rules that look like trivia and are not (R-EXT-17, R-EXT-18,
 * Inventory §10.23):
 *
 * 1. **Register listeners before the first `await`.** The popup script starts running
 *    because the user opened the popup; a listener attached after an await has already
 *    missed the event that woke it.
 * 2. **Buttons are disabled in the HTML**, not by JavaScript afterwards. Between paint and
 *    the first line of script there is a window in which an enabled button can be pressed.
 * 3. **Both capture buttons toggle together.** They share the tab's scroll position, so
 *    two captures at once fight each other.
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

/** Rule 3: the two capture buttons move together. */
function setCaptureEnabled(enabled: boolean): void {
  if (buttons.fold) buttons.fold.disabled = !enabled;
  if (buttons.full) buttons.full.disabled = !enabled;
}

function render(connection: Connection | null): void {
  const state = connection?.state ?? "stale";

  if (dot) dot.dataset.state = state === "stale" ? "" : state;

  if (!connection || state === "stale") {
    if (text) text.textContent = "Curio isn't running";
    // FR-22: say so; never start it.
    if (hint) hint.textContent = "Launch Curio, then try again.";
    setCaptureEnabled(false);
    return;
  }

  if (state === "paused") {
    if (text) text.textContent = "Curio is paused";
    // Stops capture at the source rather than posting into a 503 (R-EXT-11).
    if (hint) hint.textContent = "Resume Curio from its tray icon to capture.";
    setCaptureEnabled(false);
    return;
  }

  if (text) text.textContent = "Curio is running";
  if (hint) hint.textContent = "";
  setCaptureEnabled(true);
  if (buttons.projects) buttons.projects.disabled = false;
  if (buttons.newProject) buttons.newProject.disabled = false;
}

chrome.runtime.sendMessage({ type: "status" }, (reply) => {
  render((reply?.connection as Connection | undefined) ?? null);
});
