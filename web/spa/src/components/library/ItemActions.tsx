import { useNavigate } from "@solidjs/router";
import { createSignal, Show } from "solid-js";
import { CopyButton } from "~/components/library/CopyButton";
import { briefText } from "~/components/library/handoff";
import { deleteItem, reassessItem } from "~/lib/api";
import { paused } from "~/lib/http";
import type { Item } from "~/lib/types";

/**
 * Everything you can do to an item that is not editing a field (R-FE-15b).
 *
 * The agent-path block is the point of the panel: Curio's output is a directory an agent
 * can read, and this is where a user gets hold of it. Copying the path is the handoff —
 * there is no integration to configure.
 */
export function ItemActions(props: {
  item: Item;
  /** The item's absolute directory, once Settings has answered. */
  directory: string | null;
  apiKeySet: boolean | null;
}) {
  const navigate = useNavigate();
  const [asking, setAsking] = createSignal(false);
  const [busy, setBusy] = createSignal<string | null>(null);
  const [problem, setProblem] = createSignal<string | null>(null);

  const pausedReason = () => (paused() ? "Curio is paused. Resume from the tray icon." : undefined);

  async function reassess() {
    setBusy("reassess");
    setProblem(null);
    try {
      await reassessItem(props.item.id);
    } catch {
      setProblem("Could not queue a re-assessment.");
    }
    setBusy(null);
  }

  async function remove() {
    setBusy("delete");
    try {
      await deleteItem(props.item.id);
      navigate("/");
    } catch {
      setProblem("Could not delete this item.");
      setBusy(null);
    }
  }

  return (
    <aside class="flex flex-col gap-3">
      <Show when={props.apiKeySet === false}>
        <section class="banner tint-caution flex-col items-start gap-1">
          <strong class="font-semibold">Waiting for an API key.</strong>
          <span>
            The screenshot is stored and this page is fully editable. Curio writes the name,
            description and families once a key is set in Settings — nothing is lost while it waits.
          </span>
        </section>
      </Show>

      <section class="card flex flex-col gap-2 p-3">
        <h2 class="text-sm font-medium">Hand this to an agent</h2>
        <Show
          when={props.directory}
          fallback={<p class="text-xs text-ink-faint">Asking Curio where this item lives…</p>}
        >
          {(directory) => (
            <>
              <code class="block overflow-x-auto rounded bg-desk px-2 py-1 font-mono text-xs">
                {directory()}
              </code>
              <p class="text-xs text-ink-muted">
                The folder holds <code class="font-mono">screenshot.png</code> and{" "}
                <code class="font-mono">item.md</code>. Paste the path into Claude Code and ask it
                to read them.
              </p>
              <CopyButton label="Copy folder path" text={directory} />
            </>
          )}
        </Show>
      </section>

      <section class="card flex flex-col gap-2 p-3">
        <h2 class="text-sm font-medium">Copy</h2>
        <CopyButton label="Copy brief" text={() => briefText(props.item, props.directory)} />
        <CopyButton
          label="Copy image prompt"
          text={() => props.item.image_recipe ?? ""}
          blocked={
            props.item.image_recipe
              ? undefined
              : "Curio has not written an image prompt for this one yet."
          }
        />
      </section>

      <section class="card flex flex-col gap-2 p-3">
        <h2 class="text-sm font-medium">Item</h2>
        <button
          type="button"
          class="pill pill-outline"
          disabled={!!pausedReason() || busy() === "reassess"}
          title={pausedReason()}
          onClick={() => void reassess()}
        >
          {busy() === "reassess" ? "Queueing…" : "Re-assess"}
        </button>
        <p class="text-xs text-ink-faint">Re-assessment keeps any name you have edited yourself.</p>

        <Show
          when={asking()}
          fallback={
            <button
              type="button"
              class="pill"
              disabled={!!pausedReason()}
              title={pausedReason()}
              onClick={() => setAsking(true)}
            >
              Delete
            </button>
          }
        >
          <div class="flex flex-col gap-2">
            <p class="text-xs text-ink-muted">
              Delete this item and its folder? The screenshot goes with it.
            </p>
            <div class="flex gap-2">
              <button
                type="button"
                class="pill tint-caution"
                disabled={busy() === "delete"}
                onClick={() => void remove()}
              >
                {busy() === "delete" ? "Deleting…" : "Yes, delete"}
              </button>
              <button type="button" class="pill" onClick={() => setAsking(false)}>
                Keep it
              </button>
            </div>
          </div>
        </Show>

        <Show when={problem()}>
          <output class="banner tint-caution">{problem()}</output>
        </Show>
      </section>
    </aside>
  );
}
