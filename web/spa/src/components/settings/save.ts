/**
 * Commit-on-done with an undo, per section.
 *
 * A section owns one saver. The saver holds the patch that puts the old value back, which
 * is what makes "Saved · Undo" honest: the offer exists only when there is something to
 * restore, and the API key never gets one because a write-only field cannot be restored.
 */

import { type Accessor, createSignal } from "solid-js";
import type { Commit } from "~/components/settings/model";
import { ApiError } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { SettingsPatch } from "~/lib/types";

export type SaveState =
  | { kind: "idle" }
  | { kind: "saving" }
  | { kind: "saved"; undoable: boolean }
  | { kind: "reverted" }
  | { kind: "paused" }
  | { kind: "refused"; message: string };

export interface Saver {
  state: Accessor<SaveState>;
  /** Commit `patch`; `undo` is the patch that puts the previous value back. */
  save(patch: SettingsPatch, undo?: SettingsPatch): Promise<void>;
  undo(): void;
  /** The confirmation's eight seconds are up, or the user closed it. */
  dismiss(): void;
}

export function createSaver(commit: Commit): Saver {
  const [state, setState] = createSignal<SaveState>({ kind: "idle" });
  let restore: SettingsPatch | undefined;

  async function run(
    patch: SettingsPatch,
    undo: SettingsPatch | undefined,
    outcome: "saved" | "reverted",
  ): Promise<void> {
    setState({ kind: "saving" });
    try {
      await commit(patch);
      restore = undo;
      setState(
        outcome === "reverted"
          ? { kind: "reverted" }
          : { kind: "saved", undoable: undo !== undefined },
      );
    } catch (error) {
      setState(explain(error));
    }
  }

  return {
    state,
    save: (patch, undo) => run(patch, undo, "saved"),
    undo: () => {
      const patch = restore;
      if (!patch) return;
      restore = undefined;
      void run(patch, undefined, "reverted");
    },
    dismiss: () => {
      // Only ever clears a confirmation. The badge fades for a moment before it says it has
      // gone, and a save started inside that moment must not be wiped by the last one's
      // clock — "Saving…" would vanish and the section would look untouched.
      const current = state();
      if (current.kind !== "saved" && current.kind !== "reverted") return;
      // The offer went with the badge, so what it would have restored goes too.
      restore = undefined;
      setState({ kind: "idle" });
    },
  };
}

/**
 * The two gestures that say "I am done with this field": clicking away, and pressing Enter.
 *
 * There is no save button on this page, so blur alone leaves the keyboard user hunting for
 * somewhere to click. Enter is the same commit, spelled the way a form spells it.
 *
 * Enter does not blur. The field keeps focus, because a typo caught by the badge is fixed
 * where the caret already is. The blur that eventually follows re-runs `commit`, which every
 * caller has already made a no-op by comparing against the value it just saved.
 */
export function blurOrEnter(commit: (input: HTMLInputElement) => void) {
  return {
    onBlur: (event: FocusEvent & { currentTarget: HTMLInputElement }) =>
      commit(event.currentTarget),
    onKeyDown: (event: KeyboardEvent & { currentTarget: HTMLInputElement }) => {
      // `isComposing` is the IME: mid-composition Enter picks a candidate, it does not end
      // the edit, and committing there would save a half-typed value.
      if (event.key !== "Enter" || event.isComposing) return;
      event.preventDefault();
      commit(event.currentTarget);
    },
  };
}

function explain(error: unknown): SaveState {
  // A 503 on a mutation is the pause the user chose from the tray, not a fault (R-FE-8).
  // Rendering it as an error would describe their own decision back to them as a failure.
  // The server's own words when it has them. They arrive in the language the server writes
  // in, which is English — but a validation message naming the field it rejected is still
  // more use than a generic refusal, so it is preferred over the translated fallback.
  if (error instanceof ApiError) {
    return error.isPaused ? { kind: "paused" } : { kind: "refused", message: error.message };
  }
  return { kind: "refused", message: t("settings.save.failed") };
}
