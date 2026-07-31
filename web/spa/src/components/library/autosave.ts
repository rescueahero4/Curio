import { createSignal, onCleanup } from "solid-js";
import { ApiError } from "~/lib/http";
import type { ItemPatch } from "~/lib/types";

/** R-FE-13. Long enough to coalesce a sentence, short enough that a tab-away is safe. */
const COALESCE_MS = 600;

export type SaveState = "idle" | "saving" | "saved" | "error";

/**
 * Field-level autosave, and the guard that goes with it.
 *
 * Two things are load-bearing here. **Coalescing**: a PATCH per keystroke would write a
 * hundred rows for one sentence and stamp `last_edited_by = user` a hundred times, so edits
 * queue and go out 600 ms after the typing stops.
 *
 * **The pending-patch guard**: the server echoes every PATCH back as `item.updated`, and
 * that echo is by definition older than whatever the user has typed since. A field with an
 * edit queued or in flight therefore ignores server truth until its own save lands —
 * without that, the cursor jumps back mid-sentence on a slow save. The guard is narrow on
 * purpose: it covers named fields, not the whole item, so everything the user is *not*
 * typing into still follows the server (ARCH-03 store discipline).
 */
export function createAutosave(save: (patch: ItemPatch) => Promise<unknown>) {
  const [queuedKeys, setQueuedKeys] = createSignal<string[]>([]);
  const [inFlight, setInFlight] = createSignal<string[]>([]);
  const [state, setState] = createSignal<SaveState>("idle");
  const [problem, setProblem] = createSignal<string | null>(null);

  let queued: ItemPatch = {};
  let timer: ReturnType<typeof setTimeout> | undefined;
  let sending = false;

  function edit(patch: ItemPatch): void {
    queued = { ...queued, ...patch };
    setQueuedKeys(Object.keys(queued));
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void send(), COALESCE_MS);
  }

  async function send(): Promise<void> {
    timer = undefined;
    if (sending || !Object.keys(queued).length) return;

    const patch = queued;
    queued = {};
    setQueuedKeys([]);
    setInFlight(Object.keys(patch));
    setState("saving");
    sending = true;

    try {
      await save(patch);
      setState("saved");
      setProblem(null);
    } catch (error) {
      setState("error");
      setProblem(explainSave(error));
    } finally {
      sending = false;
      setInFlight([]);
      // Anything typed while that request was out goes now rather than waiting for the
      // next keystroke — otherwise a user who stops typing mid-save loses the tail.
      if (Object.keys(queued).length) void send();
    }
  }

  // Navigating away must not drop the last 600 ms of typing.
  onCleanup(() => {
    if (timer) clearTimeout(timer);
    void send();
  });

  return {
    edit,
    state,
    problem,
    /** Whether server truth for this field must be ignored right now (R-FE-13). */
    guarded: (field: string) => queuedKeys().includes(field) || inFlight().includes(field),
    flush: () => {
      if (timer) clearTimeout(timer);
      return send();
    },
  };
}

export type Autosave = ReturnType<typeof createAutosave>;

function explainSave(error: unknown): string {
  if (!(error instanceof ApiError)) return "That edit did not save.";
  if (error.isPaused) return "Curio is paused, so the edit was not saved. Resume from the tray.";
  if (error.unreachable) return "Curio is not answering, so the edit was not saved.";
  return error.message;
}
