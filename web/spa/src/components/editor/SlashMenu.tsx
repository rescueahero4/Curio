/**
 * The two-stage slash menu (FR-13, R-FE-17, R-FE-21).
 *
 * Nothing in here is focusable. The caret stays in the paragraph the whole time and the
 * keys arrive from the editor's own keydown handler, which is the difference between a
 * menu you can type through and a menu that makes you stop typing to use it.
 *
 * Stage two is a multi-select because that is how the picking actually goes: a brief cites
 * three tags at once far more often than one, and reopening the menu twice to say so is
 * the sort of small tax that stops people using the vocabulary they built.
 */

import {
  createEffect,
  createMemo,
  createResource,
  createSignal,
  For,
  onCleanup,
  onMount,
  Show,
} from "solid-js";
import type { ChipKind } from "~/components/editor/chips";
import type { PaletteEntry, PickerEntry } from "~/components/editor/palette";
import {
  filterPicker,
  loadPicker,
  matchPalette,
  resolvePalette,
  splitQuery,
} from "~/components/editor/palette";
import type { SlashTrigger } from "~/components/editor/slashTrigger";

interface Props {
  trigger: SlashTrigger;
  /** Hands the editor a keydown handler for as long as the menu is open. */
  onKeys: (handler: ((event: KeyboardEvent) => boolean) | null) => void;
  onChoose: (entry: PaletteEntry) => void;
  onInsert: (chip: ChipKind, picks: PickerEntry[]) => void;
  onDismiss: () => void;
}

export function SlashMenu(props: Props) {
  const parts = createMemo(() => splitQuery(props.trigger.query));
  const palette = createMemo(() => matchPalette(parts().head));
  const chosen = createMemo(() => (parts().tail === null ? null : resolvePalette(parts().head)));

  const [entries] = createResource(
    () => chosen()?.kind,
    (kind) => loadPicker(kind),
  );

  const picks = createMemo(() => filterPicker(entries() ?? [], parts().tail ?? ""));

  const options = createMemo<readonly { id: string; label: string; hint: string }[]>(() =>
    chosen() ? picks() : palette().map((entry) => ({ id: entry.kind, ...entry })),
  );

  const [highlight, setHighlight] = createSignal(0);
  const [ticked, setTicked] = createSignal<readonly PickerEntry[]>([]);

  createEffect(() => {
    options();
    setHighlight(0);
  });

  createEffect(() => {
    chosen();
    setTicked([]);
  });

  const move = (step: number) => {
    const count = options().length;
    if (count) setHighlight((index) => (index + step + count) % count);
  };

  const toggle = () => {
    const entry = picks()[highlight()];
    if (!entry) return;
    setTicked((current) =>
      current.some((pick) => pick.id === entry.id)
        ? current.filter((pick) => pick.id !== entry.id)
        : [...current, entry],
    );
  };

  const commit = () => {
    const entry = chosen();
    if (!entry) {
      const choice = palette()[highlight()];
      if (choice) props.onChoose(choice);
      return;
    }
    const selected = ticked().length ? ticked() : picks().slice(highlight(), highlight() + 1);
    if (selected.length) props.onInsert(entry.chip, [...selected]);
  };

  const handle = (event: KeyboardEvent): boolean => {
    switch (event.key) {
      case "ArrowDown":
        move(1);
        return true;
      case "ArrowUp":
        move(-1);
        return true;
      case "Tab":
        if (chosen()) toggle();
        return true;
      case "Enter":
        commit();
        return true;
      case "Escape":
        // Consumed here rather than by the global arbiter: Escape must close this menu and
        // stop, and a bubbling Escape would also reach whatever layer is behind it
        // (R-FE-20's "one Escape does one thing").
        event.stopPropagation();
        props.onDismiss();
        return true;
      default:
        return false;
    }
  };

  onMount(() => props.onKeys(handle));
  onCleanup(() => props.onKeys(null));

  return (
    <div
      class="panel float fixed z-50 max-h-80 w-80 overflow-y-auto py-1"
      style={{ left: `${props.trigger.left}px`, top: `${props.trigger.bottom + 4}px` }}
      role="listbox"
      aria-label={chosen() ? `Pick a ${chosen()?.label}` : "Insert from your library"}
      // The editor keeps the caret: a mousedown that stole focus would collapse the run
      // this menu is anchored to.
      onMouseDown={(event) => event.preventDefault()}
    >
      <Show when={!chosen()}>
        <Rows
          options={palette().map((entry) => ({ id: entry.kind, ...entry }))}
          highlight={highlight()}
          ticked={[]}
          onPick={(index) => {
            const choice = palette()[index];
            if (choice) props.onChoose(choice);
          }}
        />
      </Show>

      <Show when={chosen()}>
        <Show when={!entries.loading} fallback={<Note>Reading your library…</Note>}>
          <Show when={picks().length} fallback={<Note>Nothing here matches yet.</Note>}>
            <Rows
              options={picks()}
              highlight={highlight()}
              ticked={ticked().map((pick) => pick.id)}
              onPick={(index) => {
                const entry = chosen();
                const pick = picks()[index];
                if (!entry || !pick) return;
                const already = ticked().filter((tick) => tick.id !== pick.id);
                props.onInsert(entry.chip, [...already, pick]);
              }}
            />
          </Show>
        </Show>
        <p class="border-line border-t px-3 pt-1.5 pb-1 text-2xs text-ink-faint">
          Tab to tick · Enter to insert{ticked().length ? ` ${ticked().length}` : ""} · Esc to close
        </p>
      </Show>
    </div>
  );
}

function Note(props: { children: string }) {
  return <p class="px-3 py-2 text-sm text-ink-faint">{props.children}</p>;
}

function Rows(props: {
  options: readonly { id: string; label: string; hint: string }[];
  highlight: number;
  ticked: readonly string[];
  onPick: (index: number) => void;
}) {
  return (
    <For each={props.options}>
      {(option, index) => (
        <button
          type="button"
          role="option"
          aria-selected={index() === props.highlight}
          tabIndex={-1}
          class="flex w-full flex-col items-start px-3 py-1.5 text-left"
          classList={{ "bg-desk": index() === props.highlight }}
          onClick={() => props.onPick(index())}
        >
          <span class="text-sm text-ink">
            {props.ticked.includes(option.id) ? "✓ " : ""}
            {option.label}
          </span>
          <Show when={option.hint}>
            <span class="line-clamp-1 text-2xs text-ink-faint">{option.hint}</span>
          </Show>
        </button>
      )}
    </For>
  );
}
