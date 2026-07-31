/** One project: what it is, where it is, and the two things you can do with it. */

import { createSignal, Show } from "solid-js";
import { MISSING_EXPLANATION, NO_FRONT_DOOR, PAUSED_REASON } from "~/components/projects/copy";
import { PromptLink } from "~/components/projects/PromptLink";
import { openProject, revealPath } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import type { Project, ProjectOrigin, Prompt } from "~/lib/types";

/** `POST /api/system/reveal` always answers 200; the outcome is in the body (§10.22). */
interface Outcome {
  asked: boolean;
  message: string;
}

const ORIGIN_LABEL: Record<ProjectOrigin, string> = {
  watcher: "Found by Curio",
  mcp: "Registered by an agent",
  manual: "Added by you",
};

const WHEN = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

export function ProjectCard(props: {
  project: Project;
  prompts: Prompt[];
  onChanged: (project: Project) => void;
}) {
  const [note, setNote] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);
  const [frontDoor, setFrontDoor] = createSignal(true);

  const missing = () => props.project.status === "missing";
  const launchReason = () => {
    if (paused()) return PAUSED_REASON;
    if (missing()) return "The folder is gone, so there is nothing to serve.";
    if (!frontDoor()) return NO_FRONT_DOOR;
    return undefined;
  };

  async function launch() {
    setBusy(true);
    setNote(null);
    try {
      const opened = await openProject(props.project.id);
      if (!opened.entry) {
        setFrontDoor(false);
        setNote(NO_FRONT_DOOR);
        return;
      }
      setFrontDoor(true);
      const tab = window.open(opened.url, "_blank");
      if (tab) {
        tab.opener = null;
      } else {
        setNote(`Your browser blocked the new tab. The project is at ${opened.url}.`);
      }
    } catch (error) {
      setNote(error instanceof ApiError ? error.message : "Curio could not open that project.");
    } finally {
      setBusy(false);
    }
  }

  async function reveal() {
    setBusy(true);
    setNote(null);
    try {
      // Verbatim, and deliberately phrased as a request: nothing here can know whether a
      // file manager actually opened, so claiming it did would be a guess (§10.22).
      setNote(((await revealPath(props.project.path)) as Outcome).message);
    } catch (error) {
      setNote(
        error instanceof ApiError && error.isPaused
          ? PAUSED_REASON
          : "Curio could not ask your file manager to open.",
      );
    } finally {
      setBusy(false);
    }
  }

  return (
    <article
      class="card flex flex-col gap-3 p-4"
      classList={{ "opacity-60": missing() }}
      aria-label={props.project.name}
    >
      <header class="flex flex-wrap items-baseline justify-between gap-2">
        <h2 class="text-lg font-semibold">{props.project.name}</h2>
        <span class="pill pill-outline text-2xs">{ORIGIN_LABEL[props.project.origin]}</span>
      </header>

      <p class="font-mono text-xs break-all text-ink-faint select-all">{props.project.path}</p>

      <dl class="flex flex-wrap gap-x-6 gap-y-1 text-xs text-ink-muted">
        <div class="flex gap-2">
          <dt>Detected</dt>
          <dd>
            <time class="numeric" datetime={props.project.detected_at}>
              {when(props.project.detected_at)}
            </time>
          </dd>
        </div>
        <div class="flex gap-2">
          <dt>Last opened</dt>
          <dd>
            <Show when={props.project.last_opened_at} fallback={<span class="numeric">Never</span>}>
              {(at) => (
                <time class="numeric" datetime={at()}>
                  {when(at())}
                </time>
              )}
            </Show>
          </dd>
        </div>
      </dl>

      <Show when={missing()}>
        <p class="banner tint-caution">{MISSING_EXPLANATION}</p>
      </Show>

      <div class="flex flex-wrap items-center gap-2">
        <button
          type="button"
          class="pill pill-ink"
          onClick={() => void launch()}
          disabled={busy() || paused() || missing() || !frontDoor()}
          title={launchReason()}
        >
          Launch
        </button>
        <button
          type="button"
          class="pill pill-outline"
          onClick={() => void reveal()}
          disabled={busy() || paused()}
          title={paused() ? PAUSED_REASON : undefined}
        >
          Open folder
        </button>
      </div>

      <Show when={note()}>
        {(message) => (
          <p role="status" class="text-xs text-ink-muted">
            {message()}
          </p>
        )}
      </Show>

      <PromptLink project={props.project} prompts={props.prompts} onChanged={props.onChanged} />
    </article>
  );
}

function when(iso: string): string {
  const at = new Date(iso);
  return Number.isNaN(at.getTime()) ? iso : WHEN.format(at);
}
