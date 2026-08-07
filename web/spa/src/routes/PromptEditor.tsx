/**
 * The Prompt Helper — a white sheet on a grey desk, and the only lazily-loaded route.
 *
 * TipTap plus ProseMirror is the largest thing the dashboard imports, so this route is the
 * split point: a user browsing their library never downloads an editor they did not open
 * (R-FE-3). What lives here is the arrangement — the document, the rail, the slash menu
 * and the two send actions each own their own file (NFR-8).
 *
 * The desk runs edge to edge and the page lies on it at a reading measure. That is the shape
 * PRD §5 asks for, and it is why this is the one route the shell does not put inside its
 * container: a document is not a view of a collection, and giving it the same padded column
 * as the library made it read as one more card in a list of cards.
 *
 * **This route never serializes a prompt.** It stores TipTap document JSON by PATCH; chip
 * expansion, absolute paths, newline collapsing and the `prompts/{id}.md` snapshot are the
 * server's, authoritatively (R-FE-18). Two serializers that agree today will disagree
 * eventually, and the text that disagrees is the text the user pasted into their agent. The
 * Markdown view below is that same server text, fetched — never a local re-derivation.
 */

import { useParams } from "@solidjs/router";
import type { Editor } from "@tiptap/core";
import { createResource, createSignal, onCleanup, Show } from "solid-js";
import { EditorSurface } from "~/components/editor/EditorSurface";
import type { ActionOutcome } from "~/components/editor/handoff";
import { OutcomeToast } from "~/components/editor/OutcomeToast";
import { PromptActions } from "~/components/editor/PromptActions";
import { ghostMap } from "~/components/editor/sections";
import { Toolbar } from "~/components/editor/Toolbar";
import { getPrompt, getPromptTemplate, serializePrompt, updatePrompt } from "~/lib/api";
import { t } from "~/lib/i18n";

/** The same coalescing window the item detail uses (R-FE-13). One habit, one number. */
const AUTOSAVE_MS = 600;

interface Draft {
  doc_json?: unknown;
}

export default function PromptEditor() {
  const params = useParams<{ id: string }>();

  const [prompt] = createResource(() => params.id, getPrompt);
  const [template] = createResource(getPromptTemplate);

  const [editor, setEditor] = createSignal<Editor | null>(null);
  const [saving, setSaving] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);
  const [outcome, setOutcome] = createSignal<ActionOutcome | null>(null);

  /** The server's serialized text, or null while it is being cut. Non-null means show it. */
  const [source, setSource] = createSignal<string | null>(null);
  const [showingSource, setShowingSource] = createSignal(false);

  let timer: ReturnType<typeof setTimeout> | undefined;
  let draft: Draft = {};

  const flush = async () => {
    const changes = draft;
    draft = {};
    if (timer) clearTimeout(timer);
    timer = undefined;
    if (!Object.keys(changes).length) return;

    setSaving(true);
    try {
      await updatePrompt(params.id, changes);
      setProblem(null);
    } catch (error) {
      setProblem(error instanceof Error ? error.message : t("editor.document.failed.save"));
    } finally {
      setSaving(false);
    }
  };

  const queue = (changes: Draft) => {
    draft = { ...draft, ...changes };
    if (timer) clearTimeout(timer);
    timer = setTimeout(() => void flush(), AUTOSAVE_MS);
  };

  // Leaving the route is the one moment a coalesced edit would otherwise be lost.
  onCleanup(() => void flush());

  /**
   * Show the text, or go back to the document.
   *
   * The pending edit is flushed **first**, and that ordering is the whole point: the server
   * serializes what it has stored, so previewing without saving would show the user the
   * prompt as it was 600ms ago and call it what they are about to copy.
   */
  const toggleSource = async (next: boolean) => {
    setShowingSource(next);
    if (!next) return;

    setSource(null);
    try {
      await flush();
      setSource((await serializePrompt(params.id)).text);
    } catch (error) {
      setShowingSource(false);
      setProblem(error instanceof Error ? error.message : t("editor.document.failed.source"));
    }
  };

  const ready = () => {
    const loaded = prompt();
    const scaffold = template();
    return loaded && scaffold ? { loaded, ghosts: ghostMap(scaffold.sections) } : null;
  };

  return (
    <div class="desk min-h-[calc(100vh-var(--topbar-h))]">
      <Show when={editor()}>
        {(instance) => (
          <Toolbar
            editor={instance()}
            saving={saving()}
            source={showingSource()}
            onSource={(next) => void toggleSource(next)}
            trailing={<PromptActions id={params.id} beforeSend={flush} onOutcome={setOutcome} />}
          />
        )}
      </Show>

      {/* 64rem, which is the document's measure and not the viewport's. The library is a
          wall of screenshots and gets the shell's 100rem; a page of prose does not. */}
      <div class="mx-auto flex max-w-5xl flex-col gap-4 px-6 pt-6 pb-24">
        <Show when={problem()}>
          {(text) => <output class="banner tint-caution">{text()}</output>}
        </Show>

        <Show
          when={ready()}
          fallback={
            <p class="py-12 text-center text-sm text-ink-faint">{t("editor.document.opening")}</p>
          }
        >
          {(state) => (
            // The page. The width is set for a comfortable measure rather than for the
            // viewport, and the padding is what makes it read as a document rather than as a
            // panel with text in it.
            <article class="page px-6 py-10 sm:px-14">
              {/* No title field, on purpose. Naming a document is the one question you
                  cannot answer before you have written it, and a prompt that opens on an
                  empty box asks it first. The prompts list names each row after the first
                  line the author wrote — derived server-side on every save, so the list, the
                  project cards and the `prompts/{id}.md` snapshot all read the same name. */}
              <Show when={showingSource()}>
                <Show
                  when={source()}
                  fallback={
                    <p class="min-h-96 text-sm text-ink-faint">
                      {source() === null
                        ? t("editor.document.source.reading")
                        : t("editor.document.source.empty")}
                    </p>
                  }
                >
                  {(text) => <pre class="prompt-source min-h-96">{text()}</pre>}
                </Show>
              </Show>

              <EditorSurface
                doc={state().loaded.doc_json}
                ghosts={state().ghosts}
                hidden={showingSource()}
                onReady={setEditor}
                onChange={(doc) => queue({ doc_json: doc })}
              />
            </article>
          )}
        </Show>

        <p class="max-w-prose text-xs text-ink-faint">{t("editor.document.note")}</p>
      </div>

      {/* Outside the document flow on purpose: it is a moment, not a section, and a copy
          should not reflow the prose the user is reading. */}
      <OutcomeToast outcome={outcome()} onDismiss={() => setOutcome(null)} />
    </div>
  );
}
