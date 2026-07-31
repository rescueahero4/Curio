/**
 * The claim on the next project folder (FR-16).
 *
 * A sent prompt watches for six hours. The banner says so with a clock time rather than
 * "6 hours", because the useful question is not how long the window is — it is whether the
 * folder the user is about to create will still be linked when they get to it.
 *
 * The window closing is a state worth showing, not one to hide: a prompt that has stopped
 * watching looks exactly like one that never was, and the difference explains why a
 * project did not attach itself.
 */

import { createSignal, onCleanup, onMount, Show } from "solid-js";

/** Inventory §8: the watcher links the first folder it sees within six hours of sending. */
const CLAIM_HOURS = 6;

const CLOCK = new Intl.DateTimeFormat(undefined, { hour: "numeric", minute: "2-digit" });

export function SentBanner(props: { sentAt: string; busy: boolean; onWithdraw: () => void }) {
  const [now, setNow] = createSignal(Date.now());

  onMount(() => {
    const timer = setInterval(() => setNow(Date.now()), 60_000);
    onCleanup(() => clearInterval(timer));
  });

  const expires = () => new Date(props.sentAt).getTime() + CLAIM_HOURS * 3_600_000;
  const watching = () => expires() > now();

  return (
    <output class="banner" classList={{ "tint-confirm": watching(), "tint-caution": !watching() }}>
      <Show
        when={watching()}
        fallback={
          <span>
            The claim window has closed. New folders will no longer link themselves to this prompt —
            send it again to start watching.
          </span>
        }
      >
        <span>
          Sent. Curio is watching for the next project folder until{" "}
          <time datetime={new Date(expires()).toISOString()}>{CLOCK.format(expires())}</time>.
        </span>
        <button
          type="button"
          class="pill pill-outline"
          disabled={props.busy}
          title={props.busy ? "Working…" : "Stop linking new folders to this prompt"}
          onClick={props.onWithdraw}
        >
          Stop watching
        </button>
      </Show>
    </output>
  );
}
