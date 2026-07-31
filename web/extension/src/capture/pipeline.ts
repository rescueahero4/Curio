/**
 * The capture pipeline.
 *
 * **P5.** Every ordering rule below encodes a real failure from the shipped
 * implementation. They are inherited as tested contracts, not as folklore — which is why
 * they are written down before the code exists rather than rediscovered by breaking them.
 *
 * ## The order (R-EXT-13)
 *
 * ```text
 * find server → require token → active tab is http(s)
 *   → suppressPageChrome  ← FIRST
 *   → measure             ← only now
 *   → primeLazyContent (full only)
 *   → frame loop
 *   → stitch → POST
 * ```
 *
 * **Suppress before measuring.** Hiding the scrollbar reflows the layout. Measuring first
 * bakes the pre-reflow width into every frame offset, and the stitched image tears.
 *
 * ## The frame loop (R-EXT-14)
 *
 * Scroll, **sleep ≥ 550 ms**, capture. The sleep is `captureVisibleTab`'s rate limit, not a
 * politeness delay — go faster and frames come back empty or duplicated.
 *
 * `hideFixedElements` runs **only after frame 1**, **only in full mode**, and **only for
 * elements under 90% of viewport height**, via `visibility: hidden`. Each clause matters:
 * hiding on frame 1 deletes the page header from the fold shot, which is the one part of
 * the page the user most wanted; the height limit spares full-screen overlays that are the
 * content; and `visibility` rather than `display` avoids a reflow mid-capture.
 *
 * ## Teardown is unconditional (R-EXT-15)
 *
 * Restore fixed elements, scroll behavior, styles, and the original scroll position — on
 * the **failure path exactly as on success**. A capture error that skips teardown strands
 * the user at the bottom of a page with invisible navigation, and they have no idea why.
 *
 * ## The worker can die mid-capture (R-EXT-15a)
 *
 * MV3 may kill the worker at any moment, including between frames — and then nothing runs
 * the teardown. So the injected suppression carries a **content-side watchdog**: no
 * progress ping within 10 seconds and the content script restores everything itself. A
 * worker that later wakes to find a stale in-flight record clears it and re-arms nothing.
 *
 * ## Caps
 *
 * Fold has a frame budget of 1 and is the default — **any stored or received mode value
 * other than `"full"` resolves to fold**, which is what makes a stale popup safe. Full has
 * a budget of 60 frames and a hard cap of 20,000 device pixels of page height, which pairs
 * with the server's 64 MB body cap (R-EXT-12, R-EXT-16, Inventory §10.31).
 *
 * Fold scrolls to the top first, so the same URL always yields the same image (FR-21).
 */

/** Capture modes as a total function over the frame budget (R-EXT-12). */
export type CaptureMode = "fold" | "full";

/** Fold is one frame. */
export const FOLD_FRAME_BUDGET = 1;

/** Full is at most 60 frames… */
export const FULL_FRAME_BUDGET = 60;

/** …and at most 20,000 device pixels, which pairs with the server's 64 MB body cap. */
export const MAX_PAGE_HEIGHT_PX = 20_000;

/** `captureVisibleTab`'s rate limit. Below this, frames come back empty or duplicated. */
export const FRAME_INTERVAL_MS = 550;

/** How long the content-side watchdog waits before restoring the page itself. */
export const WATCHDOG_MS = 10_000;

/**
 * Resolve a mode value into a real mode.
 *
 * Anything that is not exactly `"full"` is fold. Not defensive programming for its own
 * sake: a popup restored from a previous session may hold a stale value, and the safe
 * failure is a smaller capture rather than a 60-frame scroll the user did not ask for
 * (Inventory §7).
 */
export function resolveMode(value: unknown): CaptureMode {
  return value === "full" ? "full" : "fold";
}

/** The frame budget for a mode. */
export function frameBudget(mode: CaptureMode): number {
  return mode === "full" ? FULL_FRAME_BUDGET : FOLD_FRAME_BUDGET;
}
