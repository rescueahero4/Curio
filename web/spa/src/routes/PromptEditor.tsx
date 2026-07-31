/**
 * The prompt editor — and the only lazily-loaded route (R-FE-3).
 *
 * **P3.** The editor is TipTap 3's framework-agnostic core mounted into a Solid-managed
 * DOM node, with **no React bindings** (R-FE-16). That choice is about data, not taste:
 * `prompts.doc_json` in every existing vault is TipTap 3 document JSON using
 * TipTap-defined node names and attributes, and the chip nodes, slash trigger rules and
 * section attribute are already expressed as TipTap extensions. Raw ProseMirror would mean
 * re-deriving schema, keymap and paste rules — and re-validating every stored document.
 *
 * What this route owes:
 *
 * - Chip atoms with label fallback: `familyChip` ("◈ "), `tagChip`, `typeChip`, `itemRef`
 *   ("▣ "). A two-stage slash menu triggered only after allowed prefixes — never inside
 *   `http://`, which is the rule that stops the menu firing while someone pastes a URL
 *   (R-FE-17).
 * - A hidden `section` attribute on paragraphs driving ghost text for the eight template
 *   sections.
 * - **It must never serialize a prompt.** The SPA stores document JSON via PATCH; chip
 *   expansion, absolute-path embedding, newline collapsing and the sidecar snapshot are
 *   produced solely by the server, which is authoritative (R-FE-18). Two serializers that
 *   agree today will disagree eventually, and the copied text is what reaches the user's
 *   agent.
 * - Copy Prompt is serialize → clipboard → mark sent. Send to Claude is serialize → copy →
 *   claim → launch, and **a clipboard failure aborts the launch** — otherwise the agent
 *   opens with nothing to paste. The UI says "Asked X to open", never "opened", because it
 *   cannot know (R-FE-15, Inventory §10.22).
 */
export default function PromptEditor() {
  return (
    <section class="flex flex-col gap-3">
      <h1 class="text-xl font-semibold">Prompt</h1>
      <p style={{ color: "var(--color-muted)" }}>
        The editor lands in P3. This route is already the lazy boundary, so the TipTap and
        ProseMirror chunk stays out of every other route's bundle.
      </p>
    </section>
  );
}
