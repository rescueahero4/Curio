import { createSignal, Show } from "solid-js";
import { t } from "~/lib/i18n";

/**
 * Delete, behind a count.
 *
 * The confirmation states the number because that is the fact the user can check: "delete
 * 40 items" is verifiable against what they think they selected, where a bare "are you
 * sure" is not. In `matching` mode the number is the server's to know, so the confirmation
 * says what is true — everything the filter matches.
 *
 * That makes three whole questions rather than one sentence with a subject dropped into it.
 * The subject used to be assembled — `Delete ${count} items?` — which reads as English word
 * order and nothing else; each of the three is now a sentence a translator owns end to end.
 */
export function BulkDelete(props: {
  count: number | null;
  blocked?: string;
  onConfirm: () => void;
}) {
  const [asking, setAsking] = createSignal(false);

  /** English needs a singular; Japanese counts everything in 件 and reuses one sentence. */
  const question = () => {
    const count = props.count;
    if (count === null) return t("library.bulk.delete.askMatching");
    return count === 1
      ? t("library.bulk.delete.askOne", { count })
      : t("library.bulk.delete.askOther", { count });
  };

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
          {t("common.delete")}
        </button>
      }
    >
      <span class="flex items-center gap-2 text-sm text-ink-muted">
        {question()}
        <button
          type="button"
          class="pill tint-caution"
          onClick={() => {
            setAsking(false);
            props.onConfirm();
          }}
        >
          {t("library.bulk.delete.yes")}
        </button>
        <button type="button" class="pill" onClick={() => setAsking(false)}>
          {t("library.bulk.delete.no")}
        </button>
      </span>
    </Show>
  );
}
