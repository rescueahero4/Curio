/**
 * "Saved changes · Undo", as one badge that leaves on its own.
 *
 * The one acknowledgement a page with no Save button gets, so it has to do three things at
 * once: say the write landed, offer the way back, and then get out of the way. A "Saved" that
 * stays on screen stops being read within a day of using the app — by the third visit it is
 * furniture, and furniture cannot confirm anything. Eight seconds is long enough to notice
 * and to reach for Undo, and the bar along the bottom is what makes the disappearance feel
 * like a clock running out rather than the UI losing its place.
 *
 * The countdown holds while the pointer or the keyboard is on the badge. Undo is the whole
 * reason the badge exists; a timer that kept running while the user was reaching for it would
 * be the one moment the component is guaranteed to fail at its job.
 *
 * Dismissal is the caller's to perform. This component asks — it does not know what state the
 * badge was rendered from, and a component that hid itself would leave that state saying
 * "saved" forever with nothing on screen to match.
 */

import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { Cross } from "~/components/icons";
import { t } from "~/lib/i18n";

/** How long the badge holds. Mirrored by `--badge-hold` in styles.css, which draws it. */
const HOLD_MS = 8000;
/** The fade out. Mirrored by `--duration-fade`; the caller is told once it has finished. */
const FADE_MS = 200;

export function SavedBadge(props: {
  /**
   * What landed. `shell.saved.label` unless a caller has something more exact to say — and
   * a caller that passes one is passing an already-translated string, not a key.
   */
  label?: string;
  /** Omit when there is nothing to put back — the offer then never appears. */
  onUndo?: () => void;
  /** The timer ran out, or the user closed it. Clear whatever state rendered the badge. */
  onDismiss: () => void;
}) {
  const [leaving, setLeaving] = createSignal(false);

  let remaining = HOLD_MS;
  let startedAt = 0;
  let countdown: number | undefined;
  let fade: number | undefined;

  function run() {
    startedAt = Date.now();
    countdown = window.setTimeout(leave, remaining);
  }

  /** Stop the clock and keep what is left of it, so a hover costs the user nothing. */
  function hold() {
    if (countdown === undefined) return;
    window.clearTimeout(countdown);
    countdown = undefined;
    remaining = Math.max(0, remaining - (Date.now() - startedAt));
  }

  function resume() {
    if (countdown !== undefined || leaving()) return;
    run();
  }

  function leave() {
    countdown = undefined;
    if (leaving()) return;
    setLeaving(true);
    fade = window.setTimeout(() => props.onDismiss(), FADE_MS);
  }

  onMount(run);
  onCleanup(() => {
    window.clearTimeout(countdown);
    window.clearTimeout(fade);
  });

  return (
    // `focusin`/`focusout` rather than `focus`/`blur`: the events that matter come from the
    // two buttons inside, and only these two cross the boundary of the element listening.
    //
    // `role="status"` sits out here rather than on the green body so the whole badge is the
    // live region — and it is what tells a linter that an element carrying mouse handlers is
    // not a button in disguise. The pointer here only pauses a clock; everything a user can
    // actually press is a real button inside.
    <span
      class="badge-saved"
      role="status"
      data-leaving={leaving() ? "true" : "false"}
      onMouseEnter={hold}
      onMouseLeave={resume}
      onFocusIn={hold}
      onFocusOut={resume}
    >
      <span class="badge-saved-body">
        <strong class="badge-saved-said">{props.label ?? t("shell.saved.label")}</strong>

        <Show when={props.onUndo}>
          <button type="button" class="badge-saved-undo" onClick={() => props.onUndo?.()}>
            {t("shell.saved.undo")}
          </button>
        </Show>

        {/* The clock, drawn. `aria-hidden` because a screen reader has already been told
            what happened and cannot be shown eight seconds draining away. */}
        <span class="badge-saved-clock" aria-hidden="true" />
      </span>

      <button
        type="button"
        class="badge-saved-close"
        aria-label={t("common.dismiss")}
        title={t("common.dismiss")}
        onClick={leave}
      >
        <Cross class="h-2.5 w-2.5" />
      </button>
    </span>
  );
}
