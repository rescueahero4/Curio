import { A } from "@solidjs/router";
import { createResource, onCleanup, Show } from "solid-js";
import { getHealth } from "~/lib/api";
import { events } from "~/lib/events";

/** Floor between `/health` reads: a bulk run publishes progress every ten items. */
const REFRESH_FLOOR_MS = 5_000;

/**
 * "Queued — needs an API key" (FR-26, PRD §5 copy tone).
 *
 * The honest version of a failure that used to be silent. Without a key, capture still
 * works end to end — the item lands, the screenshot is stored, the row is browsable — and
 * only the assessment waits. So this says what is true and what happens next, rather than
 * showing an error for work that is merely queued.
 *
 * `/health` is the source because `api_key_configured` is one of its six fields and it is
 * the only unauthenticated read in the app; the queue depth comes along in the same call,
 * which is why the count is free to show.
 */
export function MissingKeyBanner() {
  const [health, { refetch }] = createResource(getHealth);

  let last = Date.now();
  const off = events.on("job.updated", () => {
    if (Date.now() - last < REFRESH_FLOOR_MS) return;
    last = Date.now();
    void refetch();
  });

  const onFocus = () => void refetch();
  window.addEventListener("focus", onFocus);

  onCleanup(() => {
    off();
    window.removeEventListener("focus", onFocus);
  });

  return (
    <Show when={health()?.api_key_configured === false}>
      <output class="banner tint-caution mt-3 w-full">
        <strong class="font-semibold">Queued — needs an API key.</strong>
        <span>
          Captures still land and stay browsable; Curio waits to describe them.{" "}
          <Show when={(health()?.queue ?? 0) > 0}>
            <span class="numeric">{health()?.queue}</span> waiting.{" "}
          </Show>
          Add a key in{" "}
          <A href="/settings" class="underline underline-offset-2">
            Settings
          </A>{" "}
          and the queue drains on its own.
        </span>
      </output>
    </Show>
  );
}
