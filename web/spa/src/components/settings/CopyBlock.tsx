/**
 * A config snippet, with the copy control sitting on the snippet itself.
 *
 * The button used to live up in the header, opposite the label. That put three words of
 * chrome between the reader and the thing they came for, and — with three of these stacked —
 * three identical "Copy" pills in a column, each some distance from the block it belonged
 * to. Floating it into the corner of the `pre` makes the association positional rather than
 * something the reader has to work out, and gives the label its line back.
 *
 * It is a lone icon, so it owns its accessible name (`aria-label`) rather than borrowing one
 * from a caption — the icon file's rule.
 */

import { createSignal, onCleanup, Show } from "solid-js";
import { Check, Copy } from "~/components/icons";
import { t } from "~/lib/i18n";

/** How long the tick stays. Long enough to be read, short enough not to be a state. */
const CONFIRM_MS = 1_600;

export function CopyBlock(props: { label: string; hint?: string; snippet: string }) {
  const [copied, setCopied] = createSignal(false);
  // A flag rather than the sentence itself. Unlike the tick, this has no clock — it stays
  // until the next attempt, which is long enough for the reader to change language under it.
  const [refused, setRefused] = createSignal(false);
  let timer: ReturnType<typeof setTimeout> | undefined;

  onCleanup(() => timer && clearTimeout(timer));

  async function copy() {
    try {
      await navigator.clipboard.writeText(props.snippet);
      setCopied(true);
      setRefused(false);
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => setCopied(false), CONFIRM_MS);
    } catch {
      // The clipboard API is permission-gated and can simply refuse. Claiming a copy that
      // did not happen sends the user to paste nothing into a config file. A refusal needs
      // more than a tick can say, so it keeps the sentence below the block.
      setCopied(false);
      setRefused(true);
    }
  }

  return (
    <div class="flex flex-col gap-2">
      <span class="text-sm font-medium">{props.label}</span>

      <Show when={props.hint}>
        <p class="max-w-prose text-xs text-ink-faint">{props.hint}</p>
      </Show>

      {/* `relative` on the wrapper rather than on the `pre`: the button has to hold its
          corner while the snippet scrolls sideways under it, and a button positioned against
          a scrolling box travels with the content. */}
      <div class="relative">
        <pre class="card overflow-x-auto bg-desk py-3 pr-12 pl-3 font-mono text-xs select-all">
          {props.snippet}
        </pre>
        <button
          type="button"
          class="pill pill-icon pill-outline absolute top-2 right-2"
          aria-label={
            copied()
              ? t("settings.snippet.copied", { label: props.label })
              : t("settings.snippet.copy", { label: props.label })
          }
          title={copied() ? t("common.copied") : t("common.copy")}
          onClick={() => void copy()}
        >
          <Show when={copied()} fallback={<Copy />}>
            <Check class="text-confirm" />
          </Show>
        </button>
      </div>

      <Show when={refused()}>
        <span role="status" class="text-xs text-caution">
          {t("settings.snippet.refused")}
        </span>
      </Show>
    </div>
  );
}
