/**
 * Projects (E6, FR-17..FR-19) — the folders your AI tools wrote.
 *
 * `GET /api/projects` repairs as it reads (§10.28): opening this page is when a stale record
 * is most visible, so the list that arrives has already been reconciled against the disk.
 * From then on the shared SSE stream keeps it current — `project.detected` for a folder the
 * watcher just saw, `project.updated` for one that moved, vanished, or came back.
 */

import { A } from "@solidjs/router";
import { createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { ProjectCard } from "~/components/projects/ProjectCard";
import { RegisterProject } from "~/components/projects/RegisterProject";
import { listProjects, listPrompts } from "~/lib/api";
import { events } from "~/lib/events";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { Project } from "~/lib/types";

export function Projects() {
  const [projects, { mutate, refetch }] = createResource(listProjects);
  const [prompts] = createResource(listPrompts);

  // Registering by hand is the escape hatch for a folder that will never appear under the
  // watched root — a project on another drive, a checked-out worktree. It is deliberately
  // not the page's primary action: the ordinary way a project arrives here is that an agent
  // wrote it and the watcher adopted it, with no user step at all (FR-17).
  const [registering, setRegistering] = createSignal(false);

  onMount(() => {
    const offs = [
      events.on("project.detected", land),
      events.on("project.updated", land),
      events.on("project.removed", gone),
    ];
    onCleanup(() => {
      for (const off of offs) off();
    });
  });

  function land(payload: unknown) {
    const project = payload as Project;
    if (project?.id) merge(project);
  }

  /** `project.removed` carries an id and nothing else — the record it named is gone. */
  function gone(payload: unknown) {
    const id = (payload as { id?: string })?.id;
    if (id) drop(id);
  }

  function merge(project: Project) {
    mutate((list) => {
      const current = list ?? [];
      const index = current.findIndex((known) => known.id === project.id);
      if (index < 0) return [project, ...current];
      const next = current.slice();
      next[index] = project;
      return next;
    });
  }

  function drop(id: string) {
    mutate((list) => (list ?? []).filter((known) => known.id !== id));
  }

  return (
    <section class="flex flex-col gap-4">
      <header class="flex flex-wrap items-baseline justify-between gap-2">
        <div>
          <h1 class="text-2xl font-semibold">{t("projects.title")}</h1>
          <p class="mt-1 text-sm text-ink-muted">{t("projects.subtitle")}</p>
        </div>
        <Show when={projects()?.length}>
          {(count) => (
            // The whole count is one string, number included. English inflects the noun and
            // Japanese puts a counter between the two, so a `<span>` around the digit with
            // the word beside it would have been an English-shaped sentence either way.
            <span class="text-sm tabular-nums text-ink-muted">
              {count() === 1
                ? t("projects.count.one", { count: count() })
                : t("projects.count.other", { count: count() })}
            </span>
          )}
        </Show>
      </header>

      <Show when={projects.error}>
        <div class="banner tint-caution">
          <span>{t("projects.failed")}</span>
          <button type="button" class="pill pill-outline" onClick={() => void refetch()}>
            {t("projects.retry")}
          </button>
        </div>
      </Show>

      <Show when={projects.loading}>
        <p class="text-sm text-ink-muted">{t("projects.loading")}</p>
      </Show>

      <Show when={projects()?.length === 0}>
        <div class="card flex flex-col gap-2 p-5">
          <h2 class="text-lg font-semibold">{t("projects.empty.title")}</h2>
          <p class="max-w-prose text-sm text-ink-muted">{t("projects.empty.body")}</p>
          <A href="/settings" class="pill pill-outline w-fit">
            {t("projects.empty.settings")}
          </A>
        </div>
      </Show>

      {/* Four across, like the library. The old two-column grid was sized for cards that
          carried a block of labelled metadata and a row of buttons; a tile does not need
          half the viewport. */}
      <div class="grid grid-cols-1 gap-x-5 gap-y-8 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4">
        <For each={projects() ?? []}>
          {(project) => (
            <ProjectCard
              project={project}
              prompts={prompts() ?? []}
              onChanged={merge}
              onRemoved={drop}
            />
          )}
        </For>
      </div>

      {/* Below the list, in quiet type: the rare case should be reachable without competing
          with the ordinary one for the eye. */}
      <footer class="mt-2 border-t border-line pt-4">
        <Show
          when={registering()}
          fallback={
            // A lead sentence and a button label, not one sentence with a control inside
            // it: the question and the offer survive translation as wholes, where a subject
            // in one key and a predicate in another would land in the wrong order in one of
            // the two languages. The full stop went with the split — it belonged to a
            // sentence that no longer runs through the button.
            //
            // The gap between the two is layout rather than a literal space, because the
            // space that reads right after "machine?" is wrong after 。
            <p class="flex flex-wrap items-baseline gap-1 text-sm text-ink-muted">
              <span>{t("projects.register.lead")}</span>
              <button
                type="button"
                class="underline decoration-line underline-offset-4 hover:text-ink"
                onClick={() => setRegistering(true)}
                disabled={paused()}
                title={paused() ? t("projects.paused") : undefined}
              >
                {t("projects.register.open")}
              </button>
            </p>
          }
        >
          <RegisterProject onRegistered={merge} onClose={() => setRegistering(false)} />
        </Show>
      </footer>
    </section>
  );
}
