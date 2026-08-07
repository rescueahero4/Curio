import { createSignal, onCleanup } from "solid-js";
import { t } from "~/lib/i18n";

/** How long "Copied" stays. Long enough to be read, short enough not to be a state. */
const CONFIRM_MS = 1_600;

/**
 * Copy, and say what actually happened.
 *
 * The clipboard write can be refused — a browser can withhold it outside a user gesture, or
 * a policy can disable it — and reporting success anyway is the kind of small lie that ends
 * with a user pasting the wrong thing into an agent. So a refusal says so, in place.
 */
export function CopyButton(props: {
  label: string;
  text: () => string;
  blocked?: string;
  class?: string;
}) {
  const [said, setSaid] = createSignal<string | null>(null);
  let timer: ReturnType<typeof setTimeout> | undefined;

  onCleanup(() => timer && clearTimeout(timer));

  async function copy() {
    try {
      await navigator.clipboard.writeText(props.text());
      setSaid(t("common.copied"));
    } catch {
      setSaid(t("library.copyFailed"));
    }
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => setSaid(null), CONFIRM_MS);
  }

  return (
    <button
      type="button"
      class={props.class ?? "pill pill-outline"}
      disabled={!!props.blocked}
      title={props.blocked}
      onClick={() => void copy()}
    >
      {said() ?? props.label}
    </button>
  );
}
