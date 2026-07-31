/**
 * The formatting bar — and every control on it survives serialization.
 *
 * That is the whole selection rule. The server turns the document into markdown-ish text:
 * bold, italic, code, headings and lists come out the other side; anything else is dropped
 * on the way. A control for a mark the serializer discards is a dead click that looks like
 * it worked, so those controls do not exist rather than being disabled with an excuse.
 *
 * `mousedown` is cancelled on every button: the caret has to stay where the user left it,
 * because a toggle applies to the selection and a lost selection applies it to nothing.
 */

import type { Editor } from "@tiptap/core";
import { createSignal, For, onCleanup, onMount } from "solid-js";

interface Control {
  label: string;
  title: string;
  name: string;
  attrs?: Record<string, unknown>;
  run: (editor: Editor) => void;
}

const CONTROLS: readonly Control[] = [
  {
    label: "B",
    title: "Bold",
    name: "bold",
    run: (editor) => editor.chain().focus().toggleBold().run(),
  },
  {
    label: "I",
    title: "Italic",
    name: "italic",
    run: (editor) => editor.chain().focus().toggleItalic().run(),
  },
  {
    label: "‹›",
    title: "Code",
    name: "code",
    run: (editor) => editor.chain().focus().toggleCode().run(),
  },
  {
    label: "H1",
    title: "Heading 1",
    name: "heading",
    attrs: { level: 1 },
    run: (editor) => editor.chain().focus().toggleHeading({ level: 1 }).run(),
  },
  {
    label: "H2",
    title: "Heading 2",
    name: "heading",
    attrs: { level: 2 },
    run: (editor) => editor.chain().focus().toggleHeading({ level: 2 }).run(),
  },
  {
    label: "H3",
    title: "Heading 3",
    name: "heading",
    attrs: { level: 3 },
    run: (editor) => editor.chain().focus().toggleHeading({ level: 3 }).run(),
  },
  {
    label: "•",
    title: "Bulleted list",
    name: "bulletList",
    run: (editor) => editor.chain().focus().toggleBulletList().run(),
  },
  {
    label: "1.",
    title: "Numbered list",
    name: "orderedList",
    run: (editor) => editor.chain().focus().toggleOrderedList().run(),
  },
];

export function Toolbar(props: { editor: Editor }) {
  // TipTap's active state is not reactive on its own: it is read from the current
  // ProseMirror state, so the bar has to be told when that state moved.
  const [version, setVersion] = createSignal(0);

  onMount(() => {
    const bump = () => setVersion((count) => count + 1);
    props.editor.on("transaction", bump);
    onCleanup(() => props.editor.off("transaction", bump));
  });

  const isActive = (control: Control) => {
    version();
    return props.editor.isActive(control.name, control.attrs);
  };

  return (
    <div class="flex flex-wrap items-center gap-1" role="toolbar" aria-label="Formatting">
      <For each={CONTROLS}>
        {(control) => (
          <button
            type="button"
            class="pill px-2.5"
            classList={{ "pill-current": isActive(control) }}
            aria-pressed={isActive(control)}
            title={control.title}
            onMouseDown={(event) => event.preventDefault()}
            onClick={() => control.run(props.editor)}
          >
            <span aria-hidden="true">{control.label}</span>
            <span class="sr-only">{control.title}</span>
          </button>
        )}
      </For>
    </div>
  );
}
