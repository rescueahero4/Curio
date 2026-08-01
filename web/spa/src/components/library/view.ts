import { createSignal } from "solid-js";

/**
 * How much of the screen one capture gets.
 *
 * `comfortable` and `dense` are the same grid at two scales; `list` is a different shape
 * entirely — one row per item, so a long library can be read by name and date rather than
 * by picture.
 */
export type ViewMode = "comfortable" | "dense" | "list";

const MODES: readonly ViewMode[] = ["comfortable", "dense", "list"] as const;

const KEY = "curio.view";

/**
 * R-FE-9 named `curio.dense` as part of the contract, back when the choice was a boolean.
 * It is read once, on a browser that has never written `curio.view`, so a user who already
 * chose the dense grid keeps it. Nothing writes it any more — the migration is one-way and
 * a downgrade is not a case this has to serve.
 */
const LEGACY_KEY = "curio.dense";

/**
 * The view mode, remembered across sessions.
 *
 * `localStorage` can throw — private-mode Safari, a disabled-storage policy — and a
 * preference about card size is never worth failing a page render for, so both sides are
 * guarded and the default is the comfortable grid.
 */
export function createViewMode() {
  const [mode, setMode] = createSignal<ViewMode>(read());

  return [
    mode,
    (next: ViewMode) => {
      setMode(next);
      try {
        localStorage.setItem(KEY, next);
      } catch {
        // Nothing to recover: the setting holds for this session and is re-chosen later.
      }
    },
  ] as const;
}

function read(): ViewMode {
  try {
    const stored = localStorage.getItem(KEY);
    if (stored && (MODES as readonly string[]).includes(stored)) return stored as ViewMode;
    return localStorage.getItem(LEGACY_KEY) === "1" ? "dense" : "comfortable";
  } catch {
    return "comfortable";
  }
}
