/**
 * What just happened, said once and then gone.
 *
 * Copy Prompt's result is a moment, not a state: the user pressed a button, it worked, and
 * nothing about the page is different afterwards. A banner in the document flow said
 * otherwise — it pushed the prose down, stayed until something else replaced it, and read as
 * a condition the prompt was now in.
 *
 * ## A refusal does not expire
 *
 * Only the confirmation is ephemeral. A clipboard that refused means the user is holding
 * nothing and is about to paste stale text into an agent, and a message that erases itself
 * after a second and a half is how they miss that. So the caution tone stays until the next
 * action clears it, and only the success dismisses itself.
 */

import { createEffect, onCleanup, Show } from "solid-js";
import type { ActionOutcome } from "~/components/editor/handoff";
import { Check } from "~/components/icons";

/** Matches `CopyButton`: long enough to be read, short enough not to be a state. */
const CONFIRM_MS = 1_600;

export function OutcomeToast(props: { outcome: ActionOutcome | null; onDismiss: () => void }) {
  let timer: ReturnType<typeof setTimeout> | undefined;

  const stop = () => {
    if (timer) clearTimeout(timer);
    timer = undefined;
  };

  createEffect(() => {
    stop();
    // Reading `outcome` inside the effect is what re-arms the timer on a second copy: the
    // same message twice is a new moment, and it should get its full dwell each time.
    if (props.outcome?.tone !== "confirm") return;
    timer = setTimeout(props.onDismiss, CONFIRM_MS);
  });

  onCleanup(stop);

  return (
    <Show when={props.outcome}>
      {(result) => (
        <output
          // `.banner` aligns to baseline, which drops the check mark below the text it sits
          // beside. Centred here rather than there: the banner is prose, this is a label.
          class="banner fixed bottom-6 left-1/2 z-50 w-auto -translate-x-1/2 flex-nowrap items-center whitespace-nowrap shadow-lg"
          classList={{
            "tint-confirm": result().tone === "confirm",
            "tint-caution": result().tone === "caution",
          }}
        >
          <Show when={result().tone === "confirm"}>
            <Check />
          </Show>
          {result().message}
        </output>
      )}
    </Show>
  );
}
