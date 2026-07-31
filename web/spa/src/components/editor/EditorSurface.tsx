/**
 * The white sheet: toolbar, document, slash menu (PRD §5).
 *
 * This component owns the TipTap instance's whole life. It is created in `onMount` against
 * a Solid-managed div and destroyed in `onCleanup` — leaving it alive past the route would
 * leak a ProseMirror view still holding document listeners, and the leak is invisible
 * until the tab has been open for a day.
 *
 * The editor is created once, from the document it was handed. It is deliberately not
 * reactive to `doc` afterwards: the autosave loop above writes the same document back, and
 * an effect that pushed the server's echo into the view would move the user's caret.
 */

import type { Editor } from "@tiptap/core";
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import type { ChipKind } from "~/components/editor/chips";
import { createPromptEditor } from "~/components/editor/createPromptEditor";
import type { PaletteEntry, PickerEntry } from "~/components/editor/palette";
import { SlashMenu } from "~/components/editor/SlashMenu";
import type { SlashTrigger } from "~/components/editor/slashTrigger";
import { dismissSlash } from "~/components/editor/slashTrigger";
import { Toolbar } from "~/components/editor/Toolbar";

interface Props {
  doc: unknown;
  ghosts: Record<string, string>;
  onChange: (doc: unknown) => void;
}

export function EditorSurface(props: Props) {
  let host!: HTMLDivElement;
  let keys: ((event: KeyboardEvent) => boolean) | null = null;

  const [editor, setEditor] = createSignal<Editor | null>(null);
  const [trigger, setTrigger] = createSignal<SlashTrigger | null>(null);

  onMount(() => {
    const instance = createPromptEditor({
      element: host,
      doc: props.doc,
      ghosts: props.ghosts,
      onChange: props.onChange,
      onSlash: (next) => setTrigger(next),
      onKeyDown: (event) => keys?.(event) ?? false,
    });

    setEditor(instance);
    onCleanup(() => instance.destroy());
  });

  /** Stage one → stage two: the run becomes `/aesthetic:`, and the caret stays put. */
  const choose = (entry: PaletteEntry) => {
    const instance = editor();
    const run = trigger();
    if (!instance || !run) return;
    instance
      .chain()
      .focus()
      .insertContentAt({ from: run.from, to: run.to }, `/${entry.label}:`)
      .run();
  };

  const insert = (chip: ChipKind, picks: readonly PickerEntry[]) => {
    const instance = editor();
    const run = trigger();
    if (!instance || !picks.length || !run) return;

    const content = picks.flatMap((pick, index) => [
      ...(index ? [{ type: "text", text: " " }] : []),
      { type: chip, attrs: { id: pick.id, label: pick.label } },
    ]);

    // The trailing space is not cosmetic: it is the prefix the next `/` needs to open the
    // menu again (R-FE-17).
    instance
      .chain()
      .focus()
      .insertContentAt({ from: run.from, to: run.to }, [...content, { type: "text", text: " " }])
      .run();
  };

  const dismiss = () => {
    const instance = editor();
    if (instance) dismissSlash(instance.view);
  };

  return (
    <div class="sheet flex flex-col gap-4 px-6 py-5">
      <Show when={editor()}>
        {(instance) => (
          <header class="border-line border-b pb-3">
            <Toolbar editor={instance()} />
          </header>
        )}
      </Show>

      <div ref={host} class="text-base text-ink" />

      <Show when={trigger()}>
        {(run) => (
          <SlashMenu
            trigger={run()}
            onKeys={(handler) => {
              keys = handler;
            }}
            onChoose={choose}
            onInsert={insert}
            onDismiss={dismiss}
          />
        )}
      </Show>
    </div>
  );
}
