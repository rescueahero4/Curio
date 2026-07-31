/**
 * The hidden `section` attribute, and the ghost text it drives (FR-12, R-FE-17).
 *
 * The server owns the sections. `GET /api/prompts/template` answers with them, this
 * extension is handed the result, and nothing here knows how many there are or what they
 * say — a client that hard-coded the list would drift the moment the template changed.
 *
 * Ghost text is a decoration rather than content, and that is the load-bearing part:
 * decorations live outside the document, so an untouched section carries no text to
 * autosave, no text to serialize, and no text to accidentally paste into someone's brief.
 * Every section stays deletable — the attribute rides on an ordinary paragraph.
 */

import { Extension } from "@tiptap/core";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import { Decoration, DecorationSet } from "@tiptap/pm/view";
import type { PromptTemplate } from "~/lib/types";

export interface SectionOptions {
  /** Section id → placeholder. Empty until the template has been fetched. */
  ghosts: Record<string, string>;
}

/** The class the host element's `::before` rule hangs off. */
export const GHOST_CLASS = "curio-ghost";

export const Sections = Extension.create<SectionOptions>({
  name: "sections",

  addOptions() {
    return { ghosts: {} };
  },

  addGlobalAttributes() {
    return [
      {
        types: ["paragraph"],
        attributes: {
          section: {
            default: null,
            parseHTML: (element) => element.getAttribute("data-section"),
            renderHTML: (attributes) =>
              attributes.section ? { "data-section": attributes.section as string } : {},
          },
        },
      },
    ];
  },

  addProseMirrorPlugins() {
    const { ghosts } = this.options;

    return [
      new Plugin({
        key: new PluginKey("curio-section-ghosts"),
        props: {
          decorations(state) {
            const decorations: Decoration[] = [];

            state.doc.descendants((node, position) => {
              if (node.type.name !== "paragraph" || node.content.size > 0) return;

              const id = typeof node.attrs.section === "string" ? node.attrs.section : null;
              const ghost = id ? ghosts[id] : undefined;
              if (!ghost) return;

              decorations.push(
                Decoration.node(position, position + node.nodeSize, {
                  class: GHOST_CLASS,
                  "data-ghost": ghost,
                }),
              );
            });

            return DecorationSet.create(state.doc, decorations);
          },
        },
      }),
    ];
  },
});

/** The `{id: ghost}` map the extension wants, from the shape the server sends. */
export function ghostMap(sections: PromptTemplate["sections"]): Record<string, string> {
  return Object.fromEntries(sections.map((section) => [section.id, section.ghost]));
}
