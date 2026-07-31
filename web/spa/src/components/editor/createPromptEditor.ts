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
  "[&_.curio-ghost]:before:pointer-events-none [&_.curio-ghost]:before:float-left",
  "[&_.curio-ghost]:before:h-0 [&_.curio-ghost]:before:text-ink-faint",
  "[&_.curio-ghost]:before:content-[attr(data-ghost)]",
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
