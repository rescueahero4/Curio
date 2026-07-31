/**
 * The Prompt Helper — a white sheet on a grey desk, and the only lazily-loaded route.
 *
 * TipTap plus ProseMirror is the largest thing the dashboard imports, so this route is the
 * split point: a user browsing their library never downloads an editor they did not open
 * (R-FE-3). What lives here is the arrangement — the document, the toolbar, the slash menu
 * and the two send actions each own their own file (NFR-8).
 *
 * **This route never serializes a prompt.** It stores TipTap document JSON by PATCH; chip
 * expansion, absolute paths, newline collapsing and the `prompts/{id}.md` snapshot are the
 * server's, authoritatively (R-FE-18). Two serializers that agree today will disagree
 * eventually, and the text that disagrees is the text the user pasted into their agent.
 */

import { A, useParams } from "@solidjs/router";
import { createEffect, createResource, createSignal, on, onCleanup, Show } from "solid-js";
import { EditorSurface } from "~/components/editor/EditorSurface";
import { PromptActions } from "~/components/editor/PromptActions";
import { SentBanner } from "~/components/editor/SentBanner";
import { ghostMap } from "~/components/editor/sections";
import { clearPromptSent, getPrompt, getPromptTemplate, updatePrompt } from "~/lib/api";
import type { Prompt } from "~/lib/types";

/** The same coalescing window the item detail uses (R-FE-13). One habit, one number. */
const AUTOSAVE_MS = 600;

interface Draft {
  title?: string;
  doc_json?: unknown;
}

export default function PromptEditor() {
  const params = useParams<{ id: string }>();

  const [prompt] = createResource(() => params.id, getPrompt);
  const [template] = createResource(getPromptTemplate);

  const [title, setTitle] = createSignal("");
  const [sentAt, setSentAt] = createSignal<string | null>(null);
  const [saving, setSaving] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);
  const [withdrawing, setWithdrawing] = createSignal(false);

  createEffect(
    on(prompt, (loaded) => {
      if (!loaded) return;
      setTitle(loaded.title);
      setSentAt(loaded.sent_at);
    }),
  );

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
      setProblem(error instanceof Error ? error.message : "The last edit did not save.");
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

  const withdraw = async () => {
    setWithdrawing(true);
    try {
      await clearPromptSent(params.id);
      setSentAt(null);
    } catch (error) {
      setProblem(error instanceof Error ? error.message : "The claim could not be withdrawn.");
    } finally {
      setWithdrawing(false);
    }
  };

  const onSent = (updated: Prompt) => setSentAt(updated.sent_at);

  const ready = () => {
    const loaded = prompt();
    const scaffold = template();
    return loaded && scaffold ? { loaded, ghosts: ghostMap(scaffold.sections) } : null;
  };

  return (
    <section class="desk flex flex-col gap-4 rounded-sheet p-4 sm:p-6">
      <div class="flex flex-wrap items-start justify-between gap-4">
        <div class="flex min-w-64 flex-1 flex-col gap-2">
          <A href="/prompts" class="text-xs text-ink-faint">
            ← All prompts
          </A>
          <input
            class="field field-block text-lg"
            value={title()}
            aria-label="Prompt title"
            placeholder="Name this prompt"
            onInput={(event) => {
              setTitle(event.currentTarget.value);
              queue({ title: event.currentTarget.value });
            }}
            onBlur={() => void flush()}
          />
          <p class="text-2xs text-ink-faint">{saving() ? "Saving…" : "Saved as you type"}</p>
        </div>

        <PromptActions id={params.id} beforeSend={flush} onSent={onSent} />
      </div>

      <Show when={problem()}>
        {(text) => <output class="banner tint-caution">{text()}</output>}
      </Show>

      <Show when={sentAt()}>
        {(at) => (
          <SentBanner sentAt={at()} busy={withdrawing()} onWithdraw={() => void withdraw()} />
        )}
      </Show>

      <Show
        when={ready()}
        fallback={<p class="py-12 text-center text-sm text-ink-faint">Opening the prompt…</p>}
      >
        {(state) => (
          <EditorSurface
            doc={state().loaded.doc_json}
            ghosts={state().ghosts}
            onChange={(doc) => queue({ doc_json: doc })}
          />
        )}
      </Show>
    </section>
  );
}
