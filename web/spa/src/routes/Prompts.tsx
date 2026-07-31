/**
 * Every prompt the user has written (FR-16).
 *
 * A prompt is meant to be reused: the brief that produced a good result is the brief worth
 * running again with one line changed. So the list is the reuse surface, and every row
 * opens the same document rather than a copy of it.
 *
 * Delete is a two-step disarm rather than a `confirm()`. A native dialog steals the whole
 * window for a question the user cannot see the answer to — the row is behind the modal —
 * and it is the one control here that destroys something the server cannot give back.
 */

import { A, useNavigate } from "@solidjs/router";
import { createResource, createSignal, For, onCleanup, Show } from "solid-js";
import { createPrompt, deletePrompt, getPromptTemplate, listPrompts } from "~/lib/api";
import { createEscapeLayer } from "~/lib/keyboard";
import type { Prompt } from "~/lib/types";

/** How long a delete stays armed before it forgets it was asked. */
const DISARM_MS = 5_000;

const WHEN = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

export function Prompts() {
  const navigate = useNavigate();

  const [prompts, { refetch }] = createResource(listPrompts);
  const [armed, setArmed] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);

  let disarmTimer: ReturnType<typeof setTimeout> | undefined;

  const disarm = () => {
    if (disarmTimer) clearTimeout(disarmTimer);
    setArmed(null);
  };

  const arm = (id: string) => {
    setArmed(id);
    if (disarmTimer) clearTimeout(disarmTimer);
    disarmTimer = setTimeout(disarm, DISARM_MS);
  };

  createEscapeLayer("overlay", () => {
    if (!armed()) return false;
    disarm();
    return true;
  });

  onCleanup(() => {
    if (disarmTimer) clearTimeout(disarmTimer);
  });

  const remove = async (id: string) => {
    disarm();
    setBusy(true);
    try {
      await deletePrompt(id);
      await refetch();
      setProblem(null);
    } catch (error) {
      setProblem(error instanceof Error ? error.message : "That prompt could not be deleted.");
    } finally {
      setBusy(false);
    }
  };

  const start = async () => {
    setBusy(true);
    try {
      const created = await createPrompt();
      navigate(`/prompts/${created.id}`);
    } catch (error) {
      setProblem(error instanceof Error ? error.message : "A new prompt could not be started.");
      setBusy(false);
    }
  };

  return (
    <section class="flex flex-col gap-4">
      <header class="flex items-center justify-between gap-4">
        <h1 class="font-semibold text-xl">Prompts</h1>
        <button
          type="button"
          class="pill pill-ink"
          disabled={busy()}
          title={busy() ? "Curio is working on the last request" : "Start from the template"}
          onClick={() => void start()}
        >
          New prompt
        </button>
      </header>

      <Show when={problem()}>
        {(text) => <output class="banner tint-caution">{text()}</output>}
      </Show>

      <Show
        when={prompts()?.length}
        fallback={
          <Show
            when={!prompts.loading}
            fallback={<p class="py-12 text-center text-sm text-ink-faint">Reading your prompts…</p>}
          >
            <Empty />
          </Show>
        }
      >
        <ul class="flex flex-col gap-2">
          <For each={prompts()}>
            {(prompt) => (
              <li class="card flex flex-wrap items-center justify-between gap-3 px-4 py-3">
                <div class="flex min-w-64 flex-1 flex-col">
                  <A href={`/prompts/${prompt.id}`} class="font-medium text-ink">
                    {prompt.title}
                  </A>
                  <span class="text-2xs text-ink-faint">
                    Edited {WHEN.format(new Date(prompt.updated_at))}
                    {sentNote(prompt)}
                  </span>
                </div>

                <div class="flex items-center gap-2">
                  <A href={`/prompts/${prompt.id}`} class="pill pill-outline">
                    Reuse
                  </A>

                  <Show
                    when={armed() === prompt.id}
                    fallback={
                      <button
                        type="button"
                        class="pill"
                        disabled={busy()}
                        title={
                          busy() ? "Curio is working on the last request" : "Delete this prompt"
                        }
                        onClick={() => arm(prompt.id)}
                      >
                        Delete
                      </button>
                    }
                  >
                    <button type="button" class="pill" onClick={disarm}>
                      Keep
                    </button>
                    <button
                      type="button"
                      class="pill tint-caution"
                      disabled={busy()}
                      onClick={() => void remove(prompt.id)}
                    >
                      Delete for good
                    </button>
                  </Show>
                </div>
              </li>
            )}
          </For>
        </ul>
      </Show>
    </section>
  );
}

function sentNote(prompt: Prompt): string {
  return prompt.sent_at ? ` · sent ${WHEN.format(new Date(prompt.sent_at))}` : "";
}

/**
 * The empty state, which teaches the next action rather than reporting the obvious.
 *
 * The section names come from the server. The template is the server's to define, and a
 * list hard-coded here would keep advertising a scaffold that had changed underneath it.
 */
function Empty() {
  const [template] = createResource(getPromptTemplate);

  return (
    <div class="card flex flex-col items-center gap-2 px-6 py-12 text-center">
      <p class="font-medium">No prompts yet.</p>
      <p class="max-w-prose text-ink-muted text-sm">
        New prompt starts you on Curio's gold-standard template — a scaffold you fill in and delete
        from freely. Type <code class="font-mono">/</code> anywhere in it to pull an aesthetic,
        style, type, or item out of your library.
      </p>
      <Show when={template()}>
        {(scaffold) => (
          <p class="text-2xs text-ink-faint">
            {scaffold()
              .sections.map((section) => section.heading)
              .join(" · ")}
          </p>
        )}
      </Show>
    </div>
  );
}
