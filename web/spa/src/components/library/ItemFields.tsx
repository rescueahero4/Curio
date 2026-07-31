import { Show } from "solid-js";
import type { SaveState } from "~/components/library/autosave";
import { paused } from "~/lib/http";
import type { Item, ItemPatch } from "~/lib/types";

/**
 * The editable text of an item (FR-8).
 *
 * There is no Save button, and that is the design: edits go out 600 ms after the typing
 * stops. What replaces the button is an honest status line — "Saving…", "Saved", or the
 * reason it did not — because an autosaving form that says nothing is indistinguishable
 * from one that is losing work.
 */
export function ItemFields(props: {
  item: Item;
  state: SaveState;
  problem: string | null;
  onEdit: (patch: ItemPatch) => void;
}) {
  const blocked = () =>
    paused() ? "Curio is paused. Edits are not being saved right now." : undefined;

  return (
    <div class="flex flex-col gap-3">
      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">Name</span>
        <input
          type="text"
          class="field field-block text-lg"
          value={props.item.name}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) => props.onEdit({ name: event.currentTarget.value })}
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">Short description</span>
        <textarea
          class="field field-block"
          rows="3"
          value={props.item.short_description}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) => props.onEdit({ short_description: event.currentTarget.value })}
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">Source URL</span>
        <input
          type="url"
          class="field field-block"
          placeholder="https://"
          value={props.item.source_url ?? ""}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) =>
            props.onEdit({ source_url: event.currentTarget.value.trim() || null })
          }
        />
      </label>

      <label class="flex flex-col gap-1">
        <span class="text-sm text-ink-muted">
          Image prompt — what Curio would say to regenerate this look
        </span>
        <textarea
          class="field field-block font-mono text-xs"
          rows="4"
          value={props.item.image_recipe ?? ""}
          disabled={!!blocked()}
          title={blocked()}
          onInput={(event) =>
            props.onEdit({ image_recipe: event.currentTarget.value.trim() || null })
          }
        />
      </label>

      <p class="text-xs text-ink-faint" aria-live="polite">
        <Show when={props.problem} fallback={<Status state={props.state} item={props.item} />}>
          {(message) => <span class="text-caution">{message()}</span>}
        </Show>
      </p>
    </div>
  );
}

function Status(props: { state: SaveState; item: Item }) {
  return (
    <Show when={props.state === "idle"} fallback={props.state === "saving" ? "Saving…" : "Saved."}>
      {props.item.last_edited_by === "user"
        ? "Edited by you. Re-assessment will keep your name."
        : "Described by Curio. Anything you change here is kept."}
    </Show>
  );
}
