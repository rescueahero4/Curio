/**
 * TipTap, mounted from Solid (R-FE-16).
 *
 * The editor is TipTap 3's framework-agnostic core: a constructor that takes a DOM element
 * and a `destroy()` that gives it back. There are no React bindings here and none in
 * `package.json` — the bindings were always an optional package, and the extension code
 * underneath them is plain ProseMirror plus TipTap's schema helpers.
 *
 * The extension set is chosen against one constraint: **the server's serializer decides
 * what may exist**. Links, underline and strike are switched off because markdown-ish text
 * has nowhere to put them — the serializer would drop the mark, and a user who bolded
 * something and got plain text back has been quietly lied to (R-FE-18).
 */

import type { Content } from "@tiptap/core";
import { Editor } from "@tiptap/core";
import StarterKit from "@tiptap/starter-kit";
import { chipExtensions } from "~/components/editor/chips";
import { Sections } from "~/components/editor/sections";
import type { SlashTrigger } from "~/components/editor/slashTrigger";
import { Slash } from "~/components/editor/slashTrigger";

export interface PromptEditorConfig {
  element: HTMLElement;
  /** The stored TipTap document. The server wrote it; this editor does not invent one. */
  doc: unknown;
  ghosts: Record<string, string>;
  onChange: (doc: unknown) => void;
  onSlash: (trigger: SlashTrigger | null) => void;
  onKeyDown: (event: KeyboardEvent) => boolean;
}

/**
 * The editable element's own classes.
 *
 * Descendant styling lives here rather than in a stylesheet because the nodes are drawn by
 * ProseMirror, not by Solid — there is no component to hang a class on. Every value is a
 * theme token; the ghost rule is the one piece of real CSS, and it reads its text from the
 * decoration's `data-ghost` so no placeholder ever enters the document (FR-12).
 */
const SURFACE = [
  "min-h-96 outline-none",
  "[&_p]:my-2",
  "[&_h1]:mt-6 [&_h1]:mb-2 [&_h1]:text-2xl [&_h1]:font-semibold",
  "[&_h2]:mt-5 [&_h2]:mb-2 [&_h2]:text-xl [&_h2]:font-semibold",
  "[&_h3]:mt-4 [&_h3]:mb-2 [&_h3]:text-lg [&_h3]:font-semibold",
  "[&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5",
  "[&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5",
  "[&_blockquote]:border-line [&_blockquote]:border-l-2 [&_blockquote]:pl-3",
  "[&_blockquote]:text-ink-muted",
  "[&_code]:font-mono [&_code]:text-sm",
  "[&_pre]:bg-desk [&_pre]:rounded-card [&_pre]:p-3 [&_pre]:font-mono [&_pre]:text-sm",
  "[&_hr]:border-line [&_hr]:my-4",
  /*
   * Ghost text, in two pseudo-elements — one that shows and one that measures.
   *
   * The placeholders are worked examples now, several lines each, and that pulls two
   * requirements against each other:
   *
   * 1. **The caret must sit at the start of the line.** The usual trick is `float: left;
   *    height: 0`: a zero-height float intrudes on no line box, so the empty paragraph's
   *    caret stays at x=0 with the hint painted around it. An in-flow hint (`inline-block`)
   *    instead pushes the caret past the whole thing — measured at 862px to the right, on
   *    the example's *last* line, which is where a user clicking an empty section found it.
   * 2. **The paragraph must be as tall as its hint.** A zero-height float reserves no space,
   *    so a five-line example spills straight over the heading below it.
   *
   * `::before` satisfies the first and `::after` the second: the same text, in flow but
   * `invisible`, sizing the box the float paints into. `w-full` on the float so both wrap at
   * exactly the same width — shrink-to-fit would let them disagree, and the box would then
   * be the wrong height for what is drawn in it. `pre-line` on both because a direction is a
   * shape and its line breaks are part of the example.
   *
   * All of it costs nothing the moment the user types: the decoration goes with the first
   * keystroke and the line collapses to the real content.
   */
  "[&_.curio-ghost]:before:pointer-events-none [&_.curio-ghost]:before:float-left",
  "[&_.curio-ghost]:before:h-0 [&_.curio-ghost]:before:w-full",
  "[&_.curio-ghost]:before:whitespace-pre-line [&_.curio-ghost]:before:text-ink-faint",
  "[&_.curio-ghost]:before:content-[attr(data-ghost)]",
  "[&_.curio-ghost]:after:pointer-events-none [&_.curio-ghost]:after:block",
  "[&_.curio-ghost]:after:invisible [&_.curio-ghost]:after:whitespace-pre-line",
  "[&_.curio-ghost]:after:content-[attr(data-ghost)]",
  /*
   * Pulled up by exactly one line, because the caret needs a line box of its own and that
   * line box is already underneath the first line of the hint. Without this every empty
   * section carries a blank line the filled ones do not, and the rhythm of the page changes
   * as the user writes. The token is the same one the surface sets its own line-height from,
   * so the two cannot drift.
   */
  "[&_.curio-ghost]:after:mt-[calc(-1*var(--text-base--line-height))]",
  // A heading opens what follows it rather than closing what came before, so the air goes
  // above. The template's section names are ordinary H2s, which is the point of them: they
  // get the same treatment as one the user writes, because there is no difference.
  "[&>h2:first-child]:mt-0 [&>h3:first-child]:mt-0",
].join(" ");

export function createPromptEditor(config: PromptEditorConfig): Editor {
  return new Editor({
    element: config.element,
    content: config.doc as Content,

    extensions: [
      StarterKit.configure({
        link: false,
        underline: false,
        strike: false,
        // The document is the server's. An extension that appends a paragraph on load
        // would make opening a prompt a change to it.
        trailingNode: false,
      }),
      Sections.configure({ ghosts: config.ghosts }),
      ...chipExtensions,
      Slash.configure({ onChange: config.onSlash, onKeyDown: config.onKeyDown }),
    ],

    editorProps: { attributes: { class: SURFACE } },
    onUpdate: ({ editor }) => config.onChange(editor.getJSON()),
  });
}
