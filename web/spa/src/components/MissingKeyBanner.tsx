import { A } from "@solidjs/router";
import { createResource, onCleanup, Show } from "solid-js";
import { getHealth } from "~/lib/api";
import { events } from "~/lib/events";
import { t } from "~/lib/i18n";

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
        {/* Four whole lines rather than one sentence with a count and a link buried in it.
            The count used to be a `<span>` mid-clause and the link used to be one word
            mid-clause, and neither survives a language that puts its verb last — so the
            number goes into the template and the markup wraps the finished sentence. The
            tabular figures come along with it; `.numeric` on a run of words is a no-op.

            They are direct children of the banner, so what separates them is its `gap` and
            not a `{" "}` between the tags. A half-width space after a 。 doubles a gap that
            the full-width box has already left — the punctuation sits in the left half of
            its cell, and the space is drawn on top of the empty right half. */}
        <strong class="font-semibold">{t("shell.missingKey.title")}</strong>
        <span>{t("shell.missingKey.body")}</span>
        <Show when={(health()?.queue ?? 0) > 0}>
          <span class="numeric">
            {t("shell.missingKey.waiting", { count: health()?.queue ?? 0 })}
          </span>
        </Show>
        <A href="/settings" class="underline underline-offset-2">
          {t("shell.missingKey.addKey")}
        </A>
        <span>{t("shell.missingKey.drains")}</span>
      </output>
    </Show>
  );
}
