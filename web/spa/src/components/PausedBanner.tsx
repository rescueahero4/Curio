import { Show } from "solid-js";
import { paused } from "~/lib/http";

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
        <strong class="font-semibold">Curio is paused.</strong>
        <span>
          Browsing, search and everything already captured stay available. New captures and edits
          are refused until you choose <strong>Resume</strong> from the tray icon.
        </span>
      </output>
    </Show>
  );
}
