/**
 * Blur-commit with an undo, per section.
 *
 * A section owns one saver. The saver holds the patch that puts the old value back, which
 * is what makes "Saved · Undo" honest: the offer exists only when there is something to
 * restore, and the API key never gets one because a write-only field cannot be restored.
 */

import { type Accessor, createSignal } from "solid-js";
import type { Commit } from "~/components/settings/model";
import { ApiError } from "~/lib/http";
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
  };
}

function explain(error: unknown): SaveState {
  // A 503 on a mutation is the pause the user chose from the tray, not a fault (R-FE-8).
  // Rendering it as an error would describe their own decision back to them as a failure.
  if (error instanceof ApiError) {
    return error.isPaused ? { kind: "paused" } : { kind: "refused", message: error.message };
  }
  return { kind: "refused", message: "Curio could not save that." };
}
