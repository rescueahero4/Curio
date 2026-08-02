/**
 * The document: a TipTap instance, and the slash menu that writes into it (PRD §5).
 *
 * This component owns the TipTap instance's whole life. It is created in `onMount` against
 * a Solid-managed div and destroyed in `onCleanup` — leaving it alive past the route would
 * leak a ProseMirror view still holding document listeners, and the leak is invisible
 * until the tab has been open for a day.
 *
 * The editor is created once, from the document it was handed. It is deliberately not
 * reactive to `doc` afterwards: the autosave loop above writes the same document back, and
 * an effect that pushed the server's echo into the view would move the user's caret.
 *
 * What it does **not** own is the page it sits on. The sheet, the title and the rail are the
 * route's, because the rail is chrome that spans the window while the document has a measure
 * — they are two different widths and only one of them is this file's business. The editor
 * is handed upward through `onReady` so the rail can drive it.
 */

import type { Editor } from "@tiptap/core";
import { createSignal, onCleanup, onMount, Show } from "solid-js";
import type { ChipKind } from "~/components/editor/chips";
import { createPromptEditor } from "~/components/editor/createPromptEditor";
import type { PaletteEntry, PickerEntry } from "~/components/editor/palette";
import { SlashMenu } from "~/components/editor/SlashMenu";
import type { SlashTrigger } from "~/components/editor/slashTrigger";
import { dismissSlash } from "~/components/editor/slashTrigger";

interface Props {
  doc: unknown;
  ghosts: Record<string, string>;
  onChange: (doc: unknown) => void;
  /** Hands the live editor to the route, which renders the rail that drives it. */
  onReady: (editor: Editor) => void;
  /**
   * The Markdown view is up. The editor stays mounted and merely hidden: it holds the
   * document, the selection and the undo history, and unmounting it would throw all three
   * away to save nothing.
   */
  hidden?: boolean;
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
    props.onReady(instance);
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
    <div classList={{ hidden: props.hidden }}>
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
