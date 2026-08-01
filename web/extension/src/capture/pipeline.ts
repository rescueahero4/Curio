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

import { authedFetch, ensureConnection, PausedError } from "../worker/connection";
import {
  hideFixedElements,
  measurePage,
  type PageMetrics,
  pingWatchdog,
  primeLazyContent,
  readTitle,
  STATE_KEY,
  scrollToOffset,
  suppressPageChrome,
  teardownPage,
} from "./injected";

/** How far each priming step scrolls, and how long it waits for images to start. */
const PRIME_STEP_PX = 800;
const PRIME_SETTLE_MS = 120;

/** What a finished capture reports back to the popup. */
export interface CaptureResult {
  itemId: string;
  frames: number;
  truncated: boolean;
}

/** Progress the popup renders as its toast copy (R-EXT-18). */
export type Progress = (message: string) => void;

/** Run one function in the page and return its result. */
async function inPage<Args extends unknown[], R>(
  tabId: number,
  func: (...args: Args) => R,
  args: Args,
): Promise<R> {
  const [injected] = await chrome.scripting.executeScript({
    target: { tabId },
    func: func as (...a: unknown[]) => unknown,
    args,
  });
  return injected?.result as R;
}

/**
 * Capture the active tab.
 *
 * The ordering below is normative (R-EXT-13) and every step encodes a real failure from the
 * shipped implementation. The one worth restating: **suppress before measuring**. Hiding
 * the scrollbar reflows the page, so measuring first bakes a stale width into every frame
 * offset and the stitch tears.
 */
export async function capture(
  mode: CaptureMode,
  progress: Progress = () => {},
): Promise<CaptureResult> {
  const connection = await ensureConnection();
  if (!connection?.token) throw new Error("Can't reach Curio — open it and try again");
  // Stop at the source rather than posting into a 503 (R-EXT-11).
  if (connection.state === "paused") throw new PausedError();

  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  if (!tab?.id || !tab.url || !/^https?:/i.test(tab.url)) {
    throw new Error("This page can't be captured — open a website first.");
  }
  const tabId = tab.id;

  progress(
    mode === "full" ? "Stitching the full page… don't switch tabs." : "Capturing the visible area…",
  );

  // Armed before anything else and released in `finally`, so a throw anywhere below still
  // leaves the page as it was found (R-EXT-15).
  await inPage(tabId, suppressPageChrome, [STATE_KEY, WATCHDOG_MS]);

  try {
    const metrics = await inPage(tabId, measurePage, []);
    const title = await inPage(tabId, readTitle, []);

    if (mode === "full") {
      await inPage(tabId, primeLazyContent, [PRIME_STEP_PX, PRIME_SETTLE_MS]);
      await inPage(tabId, pingWatchdog, [STATE_KEY, WATCHDOG_MS]);
    }

    const { frames, height, truncated } = await captureFrames(tabId, mode, metrics);
    const png = await stitch(frames, metrics, height);

    progress("Adding…");
    const itemId = await upload(png, {
      url: tab.url,
      title,
      viewportWidth: metrics.viewportWidth,
      viewportHeight: metrics.viewportHeight,
    });

    return { itemId, frames: frames.length, truncated };
  } finally {
    // Unconditional. A capture error that skips this strands the user partway down a page
    // with the scrollbar gone and no explanation.
    await inPage(tabId, teardownPage, [STATE_KEY]).catch(() => {});
  }
}

/** One frame: where it sits in the page, and what it looks like. */
interface Frame {
  offset: number;
  dataUrl: string;
}

/** Scroll, wait out the rate limit, capture. Repeat within budget. */
async function captureFrames(
  tabId: number,
  mode: CaptureMode,
  metrics: PageMetrics,
): Promise<{ frames: Frame[]; height: number; truncated: boolean }> {
  const budget = frameBudget(mode);
  // Fold is one viewport, from the top, so the same URL always yields the same image
  // (FR-21). Full is the page, capped in device pixels to pair with the server's 64 MB
  // body limit (R-EXT-16, Inventory §10.31).
  const cap = MAX_PAGE_HEIGHT_PX / metrics.devicePixelRatio;
  const wanted = mode === "full" ? metrics.height : metrics.viewportHeight;
  const height = Math.min(wanted, cap);
  const truncated = wanted > cap;

  const frames: Frame[] = [];
  let offset = 0;

  while (frames.length < budget && offset < height) {
    const landed = await inPage(tabId, scrollToOffset, [offset]);

    // The 550 ms is `captureVisibleTab`'s rate limit, not politeness. Faster and frames
    // come back empty or duplicated (R-EXT-14).
    await new Promise((resolve) => setTimeout(resolve, FRAME_INTERVAL_MS));

    const dataUrl = await chrome.tabs.captureVisibleTab({ format: "png" });
    frames.push({ offset: landed, dataUrl });
    await inPage(tabId, pingWatchdog, [STATE_KEY, WATCHDOG_MS]);

    // Only after frame 1, only in full mode: hiding on the first frame would delete the
    // page header from the fold shot (R-EXT-14).
    if (mode === "full" && frames.length === 1) {
      await inPage(tabId, hideFixedElements, [STATE_KEY, metrics.viewportHeight]);
    }

    const next = landed + metrics.viewportHeight;
    // The page stopped moving: we are at the bottom, and another frame would duplicate
    // this one.
    if (next <= offset) break;
    offset = next;
  }

  return { frames, height, truncated };
}

/** Draw every frame into one PNG at device resolution (R-EXT-16). */
async function stitch(frames: Frame[], metrics: PageMetrics, height: number): Promise<Blob> {
  const ratio = metrics.devicePixelRatio;
  const canvas = new OffscreenCanvas(
    Math.round(metrics.viewportWidth * ratio),
    Math.round(height * ratio),
  );
  const context = canvas.getContext("2d");
  if (!context) throw new Error("This page couldn't be turned into an image.");

  for (const frame of frames) {
    const response = await fetch(frame.dataUrl);
    const bitmap = await createImageBitmap(await response.blob());
    context.drawImage(bitmap, 0, Math.round(frame.offset * ratio));
    bitmap.close();
  }

  return canvas.convertToBlob({ type: "image/png" });
}

/** POST the capture. A 401 here re-handshakes once, inside `authedFetch` (R-EXT-18a). */
async function upload(
  png: Blob,
  meta: { url: string; title: string; viewportWidth: number; viewportHeight: number },
): Promise<string> {
  const form = new FormData();
  form.append("screenshot", png, "screenshot.png");
  form.append("source_url", meta.url);
  if (meta.title) form.append("title", meta.title);
  form.append("captured_at", new Date().toISOString());
  // What makes "the first fold" a measurement rather than a guess, server-side (R-BE-26).
  form.append("viewport_width", String(meta.viewportWidth));
  form.append("viewport_height", String(meta.viewportHeight));

  // `/api/items`, not the `/api/ingest` R-EXT-16 names: the two routes were consolidated
  // when the pairing token was replaced by the runtime token, and this is the one the
  // server serves (Inventory §1).
  const response = await authedFetch("/api/items", { method: "POST", body: form });

  if (response.status === 503) throw new PausedError();
  if (!response.ok) {
    throw new Error(`Curio couldn't save that (${response.status}).`);
  }

  const created = (await response.json()) as { item_id?: string };
  return created.item_id ?? "";
}
