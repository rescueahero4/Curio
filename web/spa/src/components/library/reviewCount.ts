import { createSignal, onCleanup, onMount } from "solid-js";
import { countItems } from "~/lib/api";
import { events } from "~/lib/events";

/**
 * How many items are waiting on a decision, for the filter row's badge.
 *
 * Deliberately its own request rather than a tally of the loaded grid. The grid holds one
 * page of whatever the current filter matches; counting review items in it would produce a
 * number that changed as the user scrolled and went to zero the moment they filtered by
 * tag. The badge has to mean "in the library", so it asks the library.
 *
 * The queue changes for reasons this page never sees — an assessment finishing on the
 * server, a threshold moving in Settings — so it re-asks on the item events rather than
 * only on mount. Those arrive in bursts (a bulk edit is one event per item), and the count
 * is a badge, so the refresh is trailing-edge debounced: many events, one request.
 */
const SETTLE_MS = 400;

export function createReviewCount() {
  const [count, setCount] = createSignal<number | null>(null);

  let timer: ReturnType<typeof setTimeout> | undefined;
  let live = true;

  async function refresh(): Promise<void> {
    try {
      const { count: next } = await countItems({ needs_review: true });
      if (live) setCount(next);
    } catch {
      // A badge is not worth a banner. The last good number stays until the next event,
      // and a filter row that can still filter is more use than one reporting its own
      // plumbing.
    }
  }

  const schedule = () => {
    clearTimeout(timer);
    timer = setTimeout(() => void refresh(), SETTLE_MS);
  };

  onMount(() => {
    void refresh();
  });

  const offs = [
    events.on("item.created", schedule),
    events.on("item.updated", schedule),
    events.on("item.deleted", schedule),
  ];

  onCleanup(() => {
    live = false;
    clearTimeout(timer);
    for (const off of offs) off();
  });

  return count;
}
