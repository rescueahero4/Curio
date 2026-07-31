/**
 * Projects (E6, FR-17..FR-19) — the folders your AI tools wrote.
 *
 * `GET /api/projects` repairs as it reads (§10.28): opening this page is when a stale record
 * is most visible, so the list that arrives has already been reconciled against the disk.
 * From then on the shared SSE stream keeps it current — `project.detected` for a folder the
 * watcher just saw, `project.updated` for one that moved, vanished, or came back.
 */

import { A, useSearchParams } from "@solidjs/router";
import { createResource, createSignal, For, onCleanup, onMount, Show } from "solid-js";
import { EMPTY_BODY, EMPTY_TITLE, PAUSED_REASON } from "~/components/projects/copy";
import { ProjectCard } from "~/components/projects/ProjectCard";
import { RegisterProject } from "~/components/projects/RegisterProject";
import { listProjects, listPrompts } from "~/lib/api";
import { events } from "~/lib/events";
import { paused } from "~/lib/http";
import type { Project } from "~/lib/types";

export function Projects() {
  const [params, setParams] = useSearchParams();
  const [projects, { mutate, refetch }] = createResource(listProjects);
  const [prompts] = createResource(listPrompts);

  // The top bar's "New Project" navigates here with ?new=1 rather than offering its own
  // dialog: registering needs a folder path, and this is the page that talks about folders.
  const [registering, setRegistering] = createSignal(params.new === "1");

  onMount(() => {
    const offs = [events.on("project.detected", land), events.on("project.updated", land)];
    onCleanup(() => {
      for (const off of offs) off();
    });
  });

  function land(payload: unknown) {
    const project = payload as Project;
    if (project?.id) merge(project);
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

  function closeRegister() {
    setRegistering(false);
    // A ?new=1 left in the address bar would reopen the form on every reload of a page the
    // user has already finished with.
    if (params.new !== undefined) setParams({ new: null }, { replace: true });
  }

  return (
    <section class="flex flex-col gap-4">
      <header class="flex flex-wrap items-baseline justify-between gap-2">
        <h1 class="text-2xl font-semibold">Projects</h1>
        <button
          type="button"
          class="pill pill-outline"
          onClick={() => (registering() ? closeRegister() : setRegistering(true))}
          disabled={paused() && !registering()}
          title={paused() && !registering() ? PAUSED_REASON : undefined}
        >
          {registering() ? "Close" : "Register a folder"}
        </button>
      </header>

      <Show when={registering()}>
        <RegisterProject onRegistered={merge} onClose={closeRegister} />
      </Show>

      <Show when={projects.error}>
        <div class="banner tint-caution">
          <span>Curio could not read your projects.</span>
          <button type="button" class="pill pill-outline" onClick={() => void refetch()}>
            Try again
          </button>
        </div>
      </Show>

      <Show when={projects.loading}>
        <p class="text-sm text-ink-muted">Looking for your projects…</p>
      </Show>

      <Show when={projects()?.length === 0}>
        <div class="card flex flex-col gap-2 p-5">
          <h2 class="text-lg font-semibold">{EMPTY_TITLE}</h2>
          <p class="max-w-prose text-sm text-ink-muted">{EMPTY_BODY}</p>
          <A href="/settings" class="pill pill-outline w-fit">
            Open Settings
          </A>
        </div>
      </Show>

      <div class="grid gap-4 lg:grid-cols-2">
        <For each={projects() ?? []}>
          {(project) => (
            <ProjectCard project={project} prompts={prompts() ?? []} onChanged={merge} />
          )}
        </For>
      </div>
    </section>
  );
}
