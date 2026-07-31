import { createSignal, Show } from "solid-js";

/**
 * Delete, behind a count.
 *
 * The confirmation states the number because that is the fact the user can check: "delete
 * 40 items" is verifiable against what they think they selected, where a bare "are you
 * sure" is not. In `matching` mode the number is the server's to know, so the confirmation
 * says what is true — everything the filter matches.
 */
export function BulkDelete(props: {
  count: number | null;
  blocked?: string;
  onConfirm: () => void;
}) {
  const [asking, setAsking] = createSignal(false);

  const subject = () =>
    props.count === null ? "everything this filter matches" : `${props.count} items`;

  return (
    <Show
      when={asking()}
      fallback={
        <button
          type="button"
          class="pill"
          disabled={!!props.blocked}
          title={props.blocked}
          onClick={() => setAsking(true)}
        >
          Delete
        </button>
      }
    >
      <span class="flex items-center gap-2 text-sm text-ink-muted">
        Delete {subject()}?
        <button
          type="button"
          class="pill tint-caution"
          onClick={() => {
            setAsking(false);
            props.onConfirm();
          }}
        >
          Yes, delete
        </button>
        <button type="button" class="pill" onClick={() => setAsking(false)}>
          Keep them
        </button>
      </span>
    </Show>
  );
}
