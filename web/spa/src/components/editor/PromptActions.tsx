/**
 * The two things that happen to a whole document: copy it, destroy it.
 *
 * Copy is the handoff (R-FE-15). It is disabled while it runs — it serializes and writes the
 * clipboard, and a second press mid-sequence would leave the user unable to say which text
 * they are holding. The disabled state carries its reason, per PRD §5 — no dead clicks.
 *
 * There used to be a Send to Claude beside it, taking the primary treatment this button now
 * has. It sent nothing; see `handoff.ts` for what it actually did.
 *
 * The outcome is shown **verbatim** and by the page rather than here: these buttons sit in
 * a rail with no room for a sentence, so it is handed up to somewhere that has.
 *
 * Delete is a two-step disarm rather than a `confirm()`, for the reason the prompt list
 * records: a native dialog steals the whole window for a question about something it is
 * covering up, and this is the one control here that destroys what the server cannot give
 * back.
 */

import { useNavigate } from "@solidjs/router";
import { createSignal, onCleanup, Show } from "solid-js";
import type { ActionOutcome } from "~/components/editor/handoff";
import { copyPrompt } from "~/components/editor/handoff";
import { Trash } from "~/components/icons";
import { deletePrompt } from "~/lib/api";
import { t } from "~/lib/i18n";
import { createEscapeLayer } from "~/lib/keyboard";

/** How long a delete stays armed before it forgets it was asked. Matches the prompt list. */
const DISARM_MS = 5_000;

interface Props {
  id: string;
  /** Flushes any unsaved edit, so the server serializes what is on screen. */
  beforeSend: () => Promise<void>;
  onOutcome: (outcome: ActionOutcome | null) => void;
}

export function PromptActions(props: Props) {
  const navigate = useNavigate();

  const [busy, setBusy] = createSignal(false);
  const [armed, setArmed] = createSignal(false);

  let disarmTimer: ReturnType<typeof setTimeout> | undefined;

  const disarm = () => {
    if (disarmTimer) clearTimeout(disarmTimer);
    setArmed(false);
  };

  const arm = () => {
    setArmed(true);
    if (disarmTimer) clearTimeout(disarmTimer);
    disarmTimer = setTimeout(disarm, DISARM_MS);
  };

  createEscapeLayer("overlay", () => {
    if (!armed()) return false;
    disarm();
    return true;
  });

  onCleanup(() => {
    if (disarmTimer) clearTimeout(disarmTimer);
  });

  const run = async (action: (id: string) => Promise<ActionOutcome>) => {
    setBusy(true);
    props.onOutcome(null);
    try {
      await props.beforeSend();
      props.onOutcome(await action(props.id));
    } catch (error) {
      props.onOutcome({ message: message(error), tone: "caution" });
    } finally {
      setBusy(false);
    }
  };

  const remove = async () => {
    disarm();
    setBusy(true);
    try {
      await deletePrompt(props.id);
      navigate("/prompts");
    } catch (error) {
      props.onOutcome({ message: message(error), tone: "caution" });
      setBusy(false);
    }
  };

  const reason = () => (busy() ? t("editor.actions.busy") : undefined);

  return (
    <>
      <button
        type="button"
        class="pill pill-ink"
        disabled={busy()}
        title={reason() ?? t("editor.actions.copy.hint")}
        onClick={() => void run(copyPrompt)}
      >
        {t("editor.actions.copy.label")}
      </button>

      <Show
        when={armed()}
        fallback={
          <button
            type="button"
            class="tool-btn"
            disabled={busy()}
            aria-label={t("editor.actions.delete.label")}
            title={reason() ?? t("editor.actions.delete.hint")}
            onClick={arm}
          >
            <Trash />
          </button>
        }
      >
        <button type="button" class="tool-btn px-2 text-sm" onClick={disarm}>
          {t("editor.actions.delete.keep")}
        </button>
        <button
          type="button"
          class="pill tint-caution"
          disabled={busy()}
          onClick={() => void remove()}
        >
          {t("editor.actions.delete.confirm")}
        </button>
      </Show>
    </>
  );
}

/**
 * A server sentence if there is one, and Curio's own if there is not.
 *
 * The server's messages are not translated — they arrive as prose from another process, and
 * this is the fallback for the case where there was no sentence at all.
 */
function message(error: unknown): string {
  return error instanceof Error ? error.message : t("editor.actions.failed");
}
