/**
 * The four chip atoms (R-FE-17).
 *
 * A chip stores `{id, label}` and nothing else. The id is what the server resolves when it
 * serializes; the label is what this editor draws — which is the whole of the fallback
 * rule. A family deleted six months after the prompt was written changes what the chip
 * *expands to*, never whether it *reads*: the word the author chose is already in the
 * document, so the brief still parses as a sentence (Inventory §6).
 *
 * Node views are plain DOM built with `document.createElement` — no framework binding of
 * any kind, which is what lets identical extension code mount under Solid (R-FE-16).
 */

import { mergeAttributes, Node } from "@tiptap/core";
import { t } from "~/lib/i18n";

/** The four node types, by name. These names are the server's serializer contract. */
export type ChipKind = "familyChip" | "tagChip" | "typeChip" | "itemRef";

interface ChipSpec {
  kind: ChipKind;
  /** Drawn before the label. Two of the four carry a glyph; tags and types are the word. */
  prefix: string;
  /**
   * What kind of thing this chip is: its tooltip, and the name a screen reader announces.
   *
   * The node view pairs it with `role="img"` and `aria-label`, which is what an atom needs
   * — the caret cannot enter a chip, so there is nothing to read character by character,
   * and without a name the ◈ and ▣ glyphs are announced as themselves or dropped. A `title`
   * alone would not have done it: on a bare `<span>` it is a tooltip, and whether it is ever
   * spoken is up to the screen reader. Both are set; the `title` is for the pointer.
   *
   * A thunk, read when a node view is built, because this array is module-level and a
   * string resolved here would be fixed at import time. It is read *outside* any reactive
   * owner — ProseMirror builds these, not Solid — so a chip already on screen keeps the
   * language it was drawn in until its node view is rebuilt. Worth more than a second
   * rendering path for a name on an atom whose visible text is the author's own word
   * (R-FE-17).
   */
  role: () => string;
}

const SPECS: readonly ChipSpec[] = [
  { kind: "familyChip", prefix: "◈ ", role: () => t("editor.chip.role.family") },
  { kind: "tagChip", prefix: "", role: () => t("editor.chip.role.tag") },
  { kind: "typeChip", prefix: "", role: () => t("editor.chip.role.type") },
  { kind: "itemRef", prefix: "▣ ", role: () => t("editor.chip.role.reference") },
];

const CHIP_CLASS =
  "inline-flex items-baseline gap-1 rounded-pill border border-line bg-desk px-2 " +
  "align-baseline text-sm text-ink";

function chipNode(spec: ChipSpec) {
  return Node.create({
    name: spec.kind,
    group: "inline",
    inline: true,
    atom: true,
    selectable: true,

    addAttributes() {
      return {
        id: {
          default: "",
          parseHTML: (element) => element.getAttribute("data-id") ?? "",
          renderHTML: (attributes) => ({ "data-id": attributes.id as string }),
        },
        label: {
          default: "",
          parseHTML: (element) => element.getAttribute("data-label") ?? "",
          renderHTML: (attributes) => ({ "data-label": attributes.label as string }),
        },
      };
    },

    parseHTML() {
      return [{ tag: `span[data-chip="${spec.kind}"]` }];
    },

    renderHTML({ node, HTMLAttributes }) {
      return [
        "span",
        mergeAttributes(HTMLAttributes, { "data-chip": spec.kind, class: CHIP_CLASS }),
        text(node.attrs, spec),
      ];
    },

    addNodeView() {
      return ({ node }) => {
        const dom = document.createElement("span");
        dom.className = CHIP_CLASS;
        dom.dataset.chip = spec.kind;
        dom.contentEditable = "false";

        // Said once, worn twice: `title` for the pointer, `aria-label` for the reader.
        // `role="img"` is what makes the label *replace* the glyph rather than sit beside
        // it — and it is honest about the node, which is one indivisible thing to everything
        // outside it. It is set here only: `renderHTML` below is the serialization path, and
        // presentation attributes do not belong in the HTML the server round-trips.
        const name = `${spec.role()}: ${label(node.attrs)}`;
        dom.title = name;
        dom.setAttribute("role", "img");
        dom.setAttribute("aria-label", name);

        dom.textContent = text(node.attrs, spec);
        return { dom };
      };
    },
  });
}

function label(attrs: Record<string, unknown>): string {
  const stored = typeof attrs.label === "string" ? attrs.label : "";
  // An id with no label is a document written by something other than this editor. Showing
  // the id beats showing an empty box the caret can neither enter nor explain.
  return stored || (typeof attrs.id === "string" ? attrs.id : "");
}

function text(attrs: Record<string, unknown>, spec: ChipSpec): string {
  return `${spec.prefix}${label(attrs)}`;
}

/** The four chip extensions, in the order the slash palette offers them. */
export const chipExtensions = SPECS.map(chipNode);
