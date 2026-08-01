/**
 * The functions that run **inside the page**.
 *
 * ## They cannot close over anything
 *
 * `chrome.scripting.executeScript({func})` serializes the function to source and evaluates
 * it in the page. A reference to an imported constant, a module-level variable, or another
 * function in this file becomes `ReferenceError` at run time — and only at run time, on a
 * user's page, with no build error anywhere. So every one of these is standalone, and
 * everything it needs arrives through `args`.
 *
 * Shared state between calls lives on `window`, because each injection is a separate
 * evaluation with a fresh scope. The single namespace key keeps it findable by the
 * watchdog, which is the one piece that has to clean up after a caller that no longer
 * exists.
 *
 * ## The watchdog is the reason this file is careful
 *
 * MV3 can kill the service worker between two frames, and then nothing runs teardown — the
 * user is left partway down a page with the scrollbar gone and navigation invisible, with
 * no idea why. So suppression arms a timer *in the page*: no progress ping for ten seconds
 * and the page restores itself (R-EXT-15a).
 */

/** The single `window` key everything hangs off. */
export const STATE_KEY = "__curioCapture";

/** What the page remembers between injections. */
export interface PageState {
  scrollY: number;
  scrollBehavior: string;
  overflow: string;
  hidden: Element[];
  watchdog: number;
}

/** What a measurement produced. */
export interface PageMetrics {
  width: number;
  height: number;
  viewportWidth: number;
  viewportHeight: number;
  devicePixelRatio: number;
}

/**
 * Suppress page chrome and arm the watchdog. **Runs first, before any measurement.**
 *
 * Hiding the scrollbar reflows the layout. Measuring first bakes the pre-reflow width into
 * every frame offset and the stitched image tears (R-EXT-13).
 */
export function suppressPageChrome(key: string, watchdogMs: number): void {
  const root = document.documentElement;
  const scope = window as unknown as Record<string, unknown>;

  // Re-entry means a previous capture never tore down. Restore first so the saved
  // originals are the page's, not the last capture's.
  const previous = scope[key] as { restore?: () => void } | undefined;
  previous?.restore?.();

  const saved = {
    scrollY: window.scrollY,
    scrollBehavior: root.style.scrollBehavior,
    overflow: root.style.overflow,
    hidden: [] as HTMLElement[],
    watchdog: 0,
    restore(): void {
      window.clearTimeout(this.watchdog);
      for (const element of this.hidden) element.style.visibility = "";
      this.hidden = [];
      root.style.scrollBehavior = this.scrollBehavior;
      root.style.overflow = this.overflow;
      window.scrollTo(0, this.scrollY);
      delete (window as unknown as Record<string, unknown>)[key];
    },
    arm(ms: number): void {
      window.clearTimeout(this.watchdog);
      // The worker may be killed at any moment, including between frames. If it is, this
      // is the only thing that restores the page (R-EXT-15a).
      this.watchdog = window.setTimeout(() => this.restore(), ms);
    },
  };

  // Smooth scrolling turns every frame offset into an animation the capture races.
  root.style.scrollBehavior = "auto";
  root.style.overflow = "hidden";
  saved.arm(watchdogMs);

  scope[key] = saved;
}

/** Tell the watchdog the worker is still alive. */
export function pingWatchdog(key: string, watchdogMs: number): void {
  const state = (window as unknown as Record<string, unknown>)[key] as
    | { arm?: (ms: number) => void }
    | undefined;
  state?.arm?.(watchdogMs);
}

/** Measure the page. Only ever called **after** suppression. */
export function measurePage(): PageMetrics {
  const root = document.documentElement;
  const body = document.body;

  return {
    width: root.clientWidth,
    height: Math.max(
      root.scrollHeight,
      root.offsetHeight,
      body?.scrollHeight ?? 0,
      body?.offsetHeight ?? 0,
    ),
    viewportWidth: root.clientWidth,
    viewportHeight: window.innerHeight,
    devicePixelRatio: window.devicePixelRatio || 1,
  };
}

/**
 * Walk the page once so lazy images load, then return to the top. Full mode only.
 *
 * Without this the stitch is a column of placeholder boxes on any site that defers images
 * below the fold — which is most of them.
 */
export async function primeLazyContent(step: number, settleMs: number): Promise<void> {
  const root = document.documentElement;
  const height = Math.max(root.scrollHeight, root.offsetHeight);

  for (let offset = 0; offset < height; offset += step) {
    window.scrollTo(0, offset);
    await new Promise((resolve) => setTimeout(resolve, settleMs));
  }
  window.scrollTo(0, 0);
  await new Promise((resolve) => setTimeout(resolve, settleMs));
}

/** Scroll to an absolute offset and report where the page actually landed. */
export function scrollToOffset(offset: number): number {
  window.scrollTo(0, offset);
  return window.scrollY;
}

/**
 * Hide sticky chrome so it does not repeat down the stitch.
 *
 * Three clauses, each of which matters (R-EXT-14):
 *
 * * **only after frame 1** — hiding on the first frame deletes the page header from the
 *   fold shot, which is the part of the page the user most wanted;
 * * **only elements under 90% of viewport height** — a full-screen fixed overlay *is* the
 *   content, and hiding it captures a blank page;
 * * **`visibility`, not `display`** — `display: none` reflows the layout mid-capture and
 *   every subsequent frame offset is wrong.
 */
export function hideFixedElements(key: string, viewportHeight: number): number {
  const state = (window as unknown as Record<string, unknown>)[key] as
    | { hidden?: HTMLElement[] }
    | undefined;
  if (!state) return 0;
  state.hidden = state.hidden ?? [];

  const limit = viewportHeight * 0.9;
  let hidden = 0;

  for (const element of Array.from(document.body?.querySelectorAll("*") ?? [])) {
    const node = element as HTMLElement;
    const style = window.getComputedStyle(node);
    if (style.position !== "fixed" && style.position !== "sticky") continue;
    if (style.visibility === "hidden" || style.display === "none") continue;

    const box = node.getBoundingClientRect();
    if (box.height >= limit) continue;
    if (box.width === 0 || box.height === 0) continue;

    node.style.visibility = "hidden";
    state.hidden.push(node);
    hidden += 1;
  }

  return hidden;
}

/**
 * Restore everything. **Unconditional** — this runs on the failure path exactly as on
 * success (R-EXT-15).
 *
 * A capture error that skips teardown strands the user at the bottom of a page with
 * invisible navigation and no explanation.
 */
export function teardownPage(key: string): void {
  const state = (window as unknown as Record<string, unknown>)[key] as
    | { restore?: () => void }
    | undefined;
  state?.restore?.();
}

/** The page's own title, for the capture's default name. */
export function readTitle(): string {
  return document.title ?? "";
}
