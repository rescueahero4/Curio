import { createSignal, Show } from "solid-js";
import { type Option, OptionList } from "~/components/library/OptionList";
import { Popover } from "~/components/library/Popover";

/** How a bulk vocabulary edit applies. Never a whole-set replace — see `BulkBar`. */
export type BulkMode = "add" | "remove";

/**
 * One vocabulary panel in the bulk bar: tags, types, or families.
 *
 * Add and remove are separate buttons over one list rather than two panels, because the
 * user is answering one question — "which words" — and only then saying which direction.
 *
 * A name typed here is offered only for **add**: creating a word in order to remove it
 * would be a no-op dressed as an action.
 */
export function BulkVocabPanel(props: {
  title: string;
  options: Option[];
  empty: string;
  /** Tags and types accept a new name; families must already exist to be assigned. */
  allowNew?: boolean;
  blocked?: string;
  onApply: (mode: BulkMode, chosen: string[], fresh: string) => void;
}) {
  const [chosen, setChosen] = createSignal<string[]>([]);
  const [fresh, setFresh] = createSignal("");

  const nothing = () => chosen().length === 0 && !fresh().trim();

  const apply = (mode: BulkMode, close: () => void) => {
    props.onApply(mode, chosen(), mode === "add" ? fresh().trim() : "");
    setChosen([]);
    setFresh("");
    close();
  };

  return (
    // Opens upward: this panel only ever lives in the bulk bar, which is pinned to the
    // bottom of the viewport. See `Popover`'s `placement`.
    <Popover title={props.title} label={props.title} blocked={props.blocked} placement="top">
      {(close) => (
        <>
          <OptionList
            options={props.options}
            selected={chosen()}
            empty={props.empty}
            onToggle={(id) =>
              setChosen((current) =>
                current.includes(id) ? current.filter((value) => value !== id) : [...current, id],
              )
            }
          />

          <Show when={props.allowNew}>
            <input
              type="text"
              class="field field-block"
              placeholder={`New ${props.title.toLowerCase().replace(/s$/, "")}`}
              value={fresh()}
              onInput={(event) => setFresh(event.currentTarget.value)}
            />
          </Show>

          <div class="flex items-center gap-2">
            <button
              type="button"
              class="pill pill-ink flex-1 justify-center"
              disabled={nothing()}
              title={nothing() ? "Choose at least one first." : undefined}
              onClick={() => apply("add", close)}
            >
              Add
            </button>
            <button
              type="button"
              class="pill pill-outline flex-1 justify-center"
              disabled={chosen().length === 0}
              title={chosen().length === 0 ? "Choose at least one first." : undefined}
              onClick={() => apply("remove", close)}
            >
              Remove
            </button>
          </div>
        </>
      )}
    </Popover>
  );
}
