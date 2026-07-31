/**
 * The app's global keys, and the ordering rule behind Escape.
 *
 * R-FE-20 is one sentence — "one Escape does one thing" — and it is a rule because the
 * alternative is not obvious: with independent listeners, Escape inside an open popover
 * over a grid with a live selection closes the popover *and* clears the selection, and the
 * user loses work they never asked to discard. So Escape is arbitrated in one place, in
 * strict tiers:
 *
 *   1. close the topmost open popover / menu / dialog;
 *   2. clear the grid selection;
 *   3. no-op.
 *
 * Tier 3 is a real outcome, not a gap. Escape with nothing open and nothing selected must
 * do nothing at all — not navigate, not blur, not close the app.
 */

import { onCleanup } from "solid-js";

/** Escape's precedence tiers, highest first. */
export type EscapeTier = "overlay" | "selection";

const TIERS: readonly EscapeTier[] = ["overlay", "selection"];

/** A handler returns whether it consumed the key. Only the first consumer runs. */
type EscapeHandler = () => boolean;

interface Layer {
  tier: EscapeTier;
  handle: EscapeHandler;
}

const layers: Layer[] = [];
let attached = false;

/**
 * Register an Escape handler. Returns the disposer.
 *
 * Within a tier the newest layer wins, which is what makes nested popovers close one at a
 * time in the order the user opened them.
 */
export function pushEscapeLayer(tier: EscapeTier, handle: EscapeHandler): () => void {
  const layer: Layer = { tier, handle };
  layers.push(layer);
  attach();
  return () => {
    const index = layers.indexOf(layer);
    if (index >= 0) layers.splice(index, 1);
  };
}

/** `pushEscapeLayer`, scoped to the calling component's lifetime. */
export function createEscapeLayer(tier: EscapeTier, handle: EscapeHandler): void {
  onCleanup(pushEscapeLayer(tier, handle));
}

/** How a chord is described. `mod` is Cmd on macOS and Ctrl everywhere else. */
export interface Chord {
  key: string;
  mod?: boolean;
  shift?: boolean;
}

/**
 * Bind a global chord for the calling component's lifetime.
 *
 * Bound on `keydown` at the window, and it fires while a text field has focus — Cmd/Ctrl-K
 * has to reach the search box from inside the editor, which is where a user most often
 * wants it.
 */
export function createShortcut(chord: Chord, run: () => void): void {
  const onKeyDown = (event: KeyboardEvent) => {
    if (event.key.toLowerCase() !== chord.key.toLowerCase()) return;
    if (!!chord.mod !== (event.metaKey || event.ctrlKey)) return;
    if (!!chord.shift !== event.shiftKey) return;
    event.preventDefault();
    run();
  };

  window.addEventListener("keydown", onKeyDown);
  onCleanup(() => window.removeEventListener("keydown", onKeyDown));
}

/** Whether to render the modifier as ⌘ or as Ctrl. Presentation only. */
export function isApplePlatform(): boolean {
  return /mac|iphone|ipad/i.test(navigator.userAgent);
}

function attach(): void {
  if (attached) return;
  attached = true;
  window.addEventListener("keydown", arbitrate);
}

function arbitrate(event: KeyboardEvent): void {
  if (event.key !== "Escape") return;

  for (const tier of TIERS) {
    for (let index = layers.length - 1; index >= 0; index -= 1) {
      const layer = layers[index];
      if (!layer || layer.tier !== tier) continue;
      if (layer.handle()) {
        event.preventDefault();
        event.stopPropagation();
        return;
      }
    }
  }
}
