import { Show } from "solid-js";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";

/**
 * The paused state, at banner level and nowhere else (R-FE-8, D25).
 *
 * Pause is a soft-disable: mutations refuse with a 503, reads keep working everywhere. A
 * full-page interstitial would take away a library the server is still perfectly willing
 * to serve — so this is a strip above the content, the content stays live underneath, and
 * every mutating control disables itself with the same explanation.
 *
 * It also must never present as an error. The 503s are the app doing what the user asked
 * from the tray, and an error banner would describe their own choice as a fault.
 */
export function PausedBanner() {
  return (
    <Show when={paused()}>
      <output class="banner tint-caution mt-3 w-full">
        <strong class="font-semibold">{t("shell.paused.title")}</strong>
        {/* One key for the whole explanation, and the emphasis that used to sit on
            "Resume" is gone with it. A `<strong>` mid-sentence needs the clause either
            side of it to stay in place, and the tray command lands in a different position
            in Japanese — so the choice was a bold word here or a sentence that reads
            naturally in both languages. The sentence won. */}
        <span>{t("shell.paused.body")}</span>
      </output>
    </Show>
  );
}
