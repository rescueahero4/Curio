/**
 * The document's rail — formatting on the left, what happens to the document on the right.
 *
 * One rail rather than two stacked bands. Formatting a paragraph and copying the prompt are
 * the same kind of thing from the user's side — controls acting on this document — and the
 * chrome above already carries the way back to the list, so a second strip would spend a
 * band of screen saying that twice.
 *
 * **Every control on it survives serialization**, and that is the whole selection rule. The
 * server turns the document into markdown-ish text: bold, italic, code, headings, lists,
 * quotes, code blocks and rules come out the other side; strike and links do not, and
 * `createPromptEditor` switches those extensions off for the same reason (R-FE-18). A
 * control for a mark the serializer discards is a dead click that looks like it worked, so
 * those controls do not exist rather than being disabled with an excuse. The Bun original
 * carried a strike and a link button; they are the one deliberate omission here.
 *
 * `mousedown` is cancelled on every button: the caret has to stay where the user left it,
 * because a toggle applies to the selection and a lost selection applies it to nothing.
 */

import type { Editor } from "@tiptap/core";
import { createSignal, For, type JSX, onCleanup, onMount, Show } from "solid-js";
import { PALETTE } from "~/components/editor/palette";
import {
  Bold,
  BulletList,
  Code,
  CodeBlock,
  Divider,
  Italic,
  Markdown,
  OrderedList,
  Plus,
  Quote,
  Redo,
  Undo,
} from "~/components/icons";
import { Popover } from "~/components/library/Popover";

interface Control {
  icon: (props: { class?: string }) => JSX.Element;
  title: string;
  /** The mark or node this control reports on. Absent means it never reads as pressed. */
  name?: string;
  attrs?: Record<string, unknown>;
  run: (editor: Editor) => void;
}

/** Marks. What a run of selected text is wearing. */
const MARKS: readonly Control[] = [
  {
    icon: Bold,
    title: "Bold",
    name: "bold",
    run: (editor) => editor.chain().focus().toggleBold().run(),
  },
  {
    icon: Italic,
    title: "Italic",
    name: "italic",
    run: (editor) => editor.chain().focus().toggleItalic().run(),
  },
  {
    icon: Code,
    title: "Inline code",
    name: "code",
    run: (editor) => editor.chain().focus().toggleCode().run(),
  },
];

/** Blocks. What the paragraph the caret is in has become. */
const BLOCKS: readonly Control[] = [
  {
    icon: BulletList,
    title: "Bulleted list",
    name: "bulletList",
    run: (editor) => editor.chain().focus().toggleBulletList().run(),
  },
  {
    icon: OrderedList,
    title: "Numbered list",
    name: "orderedList",
    run: (editor) => editor.chain().focus().toggleOrderedList().run(),
  },
  {
    icon: Quote,
    title: "Quote",
    name: "blockquote",
    run: (editor) => editor.chain().focus().toggleBlockquote().run(),
  },
  {
    icon: CodeBlock,
    title: "Code block",
    name: "codeBlock",
    run: (editor) => editor.chain().focus().toggleCodeBlock().run(),
  },
  {
    icon: Divider,
    title: "Divider",
    run: (editor) => editor.chain().focus().setHorizontalRule().run(),
  },
];

/**
 * Two levels, which is what a brief needs: sections, and subsections inside them.
 *
 * The template writes its section names at level 2, so the first entry here is the one a
 * user lands on when they put the caret in "Brief" — renaming a section and writing a new
 * one are the same gesture with the same control, because the scaffold is ordinary content.
 * A third level would be a depth no prompt has ever needed and a `###` an agent reads as
 * noise.
 */
const HEADINGS = [2, 3] as const;

/** Why every editing control is off while the Markdown view is up. */
const SOURCE_REASON = "The Markdown view is read-only — switch back to edit the document";

interface Props {
  editor: Editor;
  /** True while an edit is in flight. The rail is the only place a save is acknowledged. */
  saving: boolean;
  /** The Markdown view is showing, so every control that edits the document is inert. */
  source: boolean;
  onSource: (next: boolean) => void;
  /** What the document itself can have done to it — copy, send, delete. */
  trailing: JSX.Element;
}

export function Toolbar(props: Props) {
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
    // A disabled control must not also read as applied: in the Markdown view the frozen
    // editor state would otherwise leave buttons stuck pressed with nothing the user can do.
    return !props.source && !!control.name && props.editor.isActive(control.name, control.attrs);
  };

  const heading = () => {
    version();
    return HEADINGS.find((level) => props.editor.isActive("heading", { level })) ?? null;
  };

  const can = (action: "undo" | "redo") => {
    version();
    return action === "undo" ? props.editor.can().undo() : props.editor.can().redo();
  };

  /**
   * The `/` palette's four commands, reachable by mouse.
   *
   * This types `/aesthetic:` at the caret rather than opening the picker directly, and the
   * indirection is the point: the slash trigger already watches the document, so the same
   * one interaction runs whether the user typed the slash or clicked for it. A second path
   * into the picker would be a second thing to keep working (R-FE-17).
   */
  const insert = (label: string, close: () => void) => {
    close();
    props.editor.chain().focus().insertContent(`/${label}:`).run();
  };

  return (
    <div
      // Sticky at `lg` and up only. `--topbar-h` is the bar's height as a **single row**,
      // which is what it is at `lg`; below that the search box drops to its own line and the
      // bar grows past it. A rail pinned 47px down while the thing above it is 133px tall
      // does not stay visible, it hides — so under `lg` it scrolls away with the document,
      // which is the honest behaviour rather than a broken pin.
      class="glass z-20 border-line border-b lg:sticky"
      style={{ top: "var(--topbar-h)" }}
      role="toolbar"
      aria-label="Prompt document"
    >
      {/* Wider than the page below it, narrower than the shell above it. At the sheet's own
          64rem the controls and the actions do not fit on one line — which was the whole
          point of merging them — and at the shell's 100rem the two groups drift so far apart
          that the rail stops reading as one instrument. 78rem is where both hold. */}
      <div class="mx-auto flex max-w-[78rem] flex-wrap items-center gap-0.5 px-6 py-1.5">
        <ToolButton
          title="Undo"
          shortcut="Ctrl-Z"
          disabled={props.source || !can("undo")}
          onClick={() => props.editor.chain().focus().undo().run()}
        >
          <Undo />
        </ToolButton>
        <ToolButton
          title="Redo"
          shortcut="Ctrl-Shift-Z"
          disabled={props.source || !can("redo")}
          onClick={() => props.editor.chain().focus().redo().run()}
        >
          <Redo />
        </ToolButton>

        <span class="tool-sep" aria-hidden="true" />

        <Popover
          label={<span class="text-sm">{heading() ? `Heading ${heading()}` : "Text"}</span>}
          title="Block type"
          variant="tool"
          width="11rem"
          blocked={props.source ? SOURCE_REASON : undefined}
        >
          {(close) => (
            <div class="flex flex-col">
              <BlockChoice
                label="Text"
                on={heading() === null}
                onPick={() => {
                  props.editor.chain().focus().setParagraph().run();
                  close();
                }}
              />
              <For each={HEADINGS}>
                {(level) => (
                  <BlockChoice
                    label={`Heading ${level}`}
                    markdown={"#".repeat(level)}
                    on={heading() === level}
                    onPick={() => {
                      props.editor.chain().focus().setNode("heading", { level }).run();
                      close();
                    }}
                  />
                )}
              </For>
            </div>
          )}
        </Popover>

        <span class="tool-sep" aria-hidden="true" />

        <For each={MARKS}>
          {(control) => (
            <ToolButton
              title={control.title}
              on={isActive(control)}
              disabled={props.source}
              onClick={() => control.run(props.editor)}
            >
              {control.icon({})}
            </ToolButton>
          )}
        </For>

        <span class="tool-sep" aria-hidden="true" />

        <For each={BLOCKS}>
          {(control) => (
            <ToolButton
              title={control.title}
              on={isActive(control)}
              disabled={props.source}
              onClick={() => control.run(props.editor)}
            >
              {control.icon({})}
            </ToolButton>
          )}
        </For>

        <span class="tool-sep" aria-hidden="true" />

        <Popover
          label={
            <span class="flex items-center gap-1 text-sm">
              <Plus />
              Insert
            </span>
          }
          title="Insert from the library"
          variant="tool"
          width="17rem"
          blocked={props.source ? SOURCE_REASON : undefined}
        >
          {(close) => (
            <div class="flex flex-col">
              <For each={PALETTE}>
                {(entry) => (
                  <button
                    type="button"
                    class="rounded-card px-2.5 py-1.5 text-left hover:bg-desk"
                    onClick={() => insert(entry.label, close)}
                  >
                    <span class="block text-base capitalize">{entry.label}</span>
                    <span class="block text-2xs text-ink-faint">{entry.hint}</span>
                  </button>
                )}
              </For>
            </div>
          )}
        </Popover>

        <span class="ml-auto flex items-center gap-1">
          {/* The only acknowledgement a document with no Save button gets. It says nothing
              at rest on purpose — "Saved" that never goes away stops being read. */}
          <Show when={props.saving}>
            <span class="mr-1 text-2xs text-ink-faint">Saving…</span>
          </Show>

          <button
            type="button"
            class="tool-btn px-2 text-sm"
            data-on={props.source ? "true" : "false"}
            aria-pressed={props.source}
            title="Show the text this prompt becomes when it is copied"
            onClick={() => props.onSource(!props.source)}
          >
            <Markdown />
            Markdown
          </button>

          <span class="tool-sep" aria-hidden="true" />

          {props.trailing}
        </span>
      </div>
    </div>
  );
}

function ToolButton(props: {
  title: string;
  shortcut?: string;
  on?: boolean;
  disabled?: boolean;
  onClick: () => void;
  children: JSX.Element;
}) {
  return (
    <button
      type="button"
      class="tool-btn"
      data-on={props.on ? "true" : "false"}
      aria-pressed={props.on ?? false}
      aria-label={props.title}
      title={props.disabled ? SOURCE_REASON : titleFor(props)}
      disabled={props.disabled}
      // The editor loses its selection to a plain click, and a formatting button with
      // nothing selected formats nothing.
      onMouseDown={(event) => event.preventDefault()}
      onClick={() => props.onClick()}
    >
      {props.children}
    </button>
  );
}

function titleFor(props: { title: string; shortcut?: string }): string {
  return props.shortcut ? `${props.title} · ${props.shortcut}` : props.title;
}

function BlockChoice(props: { label: string; markdown?: string; on: boolean; onPick: () => void }) {
  return (
    <button
      type="button"
      class="flex items-center justify-between rounded-card px-2.5 py-1.5 text-left text-base hover:bg-desk"
      classList={{ "font-medium": props.on }}
      onMouseDown={(event) => event.preventDefault()}
      onClick={() => props.onPick()}
    >
      {props.label}
      <Show when={props.markdown}>
        <span class="font-mono text-2xs text-ink-faint">{props.markdown}</span>
      </Show>
    </button>
  );
}
