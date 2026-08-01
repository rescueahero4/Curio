/**
 * One project: what it is, where it is, and the one thing you mostly want to do with it.
 *
 * The tile *is* the launch button. A project has no screenshot, so the space a library item
 * spends on its picture is free here — and giving it to the primary action means the card
 * needs no primary button underneath, which is what let the rest of the controls drop to
 * quiet text. The previous card offered five button-shaped things at equal weight (launch,
 * open folder, a prompt select, open prompt, an origin badge that was not a control at all),
 * and a grid of those reads as a form rather than a catalogue.
 */

import { createSignal, Show } from "solid-js";
import { MISSING_EXPLANATION, NO_FRONT_DOOR, PAUSED_REASON } from "~/components/projects/copy";
import { PromptLink } from "~/components/projects/PromptLink";
import { openProject, revealPath } from "~/lib/api";
import { absoluteTime, relativeTime } from "~/lib/format";
import { ApiError, paused } from "~/lib/http";
import type { Project, ProjectOrigin, Prompt } from "~/lib/types";

/** `POST /api/system/reveal` always answers 200; the outcome is in the body (§10.22). */
interface Outcome {
  asked: boolean;
  message: string;
}

/**
 * Only the unusual origins are named.
 *
 * "Found by Curio" is true of nearly every project and therefore tells the reader nothing —
 * it was a badge on every card carrying no signal. The other two do earn their line: they
 * behave differently (no marker file, so no rename-following, and the watcher's
 * missing-reconciliation skips them), so when one is wrong the origin is the explanation.
 */
const ORIGIN_NOTE: Partial<Record<ProjectOrigin, string>> = {
  mcp: "added by an agent",
  manual: "added by you",
};

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
    return "Launch in a new tab";
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
    // No card chrome. The tile carries the only border on the card, and the text below sits
    // on the page ground — a box around text that is already grouped by proximity is a
    // second boundary doing the first one's job. It also keeps the prompt popover free to
    // open past the card's edge, which an `overflow-hidden` card would have clipped.
    <article class="flex flex-col gap-3" aria-label={props.project.name}>
      <button
        type="button"
        class="group relative aspect-[4/3.2] overflow-hidden rounded-xl border border-line bg-card transition-colors disabled:cursor-not-allowed"
        classList={{ "opacity-60": missing() }}
        onClick={() => void launch()}
        disabled={busy() || paused() || missing() || !frontDoor()}
        title={launchReason()}
      >
        {/* A project has no preview to show, so the placeholder carries the affordance
            instead of pretending to be one. */}
        <div class="grid h-full place-items-center">
          <span class="pill pill-outline transition-colors group-enabled:group-hover:border-ink group-enabled:group-hover:bg-ink group-enabled:group-hover:text-ground">
            {launchLabel(busy(), missing(), frontDoor())}
          </span>
        </div>
        <Show when={missing()}>
          <span class="badge tint-caution absolute top-2.5 left-2.5">missing</span>
        </Show>
      </button>

      <div class="flex min-w-0 flex-col gap-1">
        <h2 class="truncate font-medium text-ink">{props.project.name}</h2>
        <p class="truncate font-mono text-2xs text-ink-faint" title={props.project.path}>
          {props.project.path}
        </p>

        {/* One line, three facts, no labels: "detected" and "opened" are the labels. */}
        <p class="flex flex-wrap items-center gap-x-3 text-xs text-ink-faint">
          <time
            datetime={props.project.detected_at}
            title={absoluteTime(props.project.detected_at)}
          >
            detected {relativeTime(props.project.detected_at)}
          </time>
          <Show when={props.project.last_opened_at}>
            {(at) => (
              <time datetime={at()} title={absoluteTime(at())}>
                opened {relativeTime(at())}
              </time>
            )}
          </Show>
          <Show when={ORIGIN_NOTE[props.project.origin]}>{(what) => <span>{what()}</span>}</Show>
        </p>

        <div class="mt-1 flex flex-wrap items-center gap-x-3 gap-y-1 text-xs">
          <button
            type="button"
            class="text-ink-muted underline decoration-line underline-offset-4 hover:text-ink disabled:no-underline disabled:hover:text-ink-muted"
            onClick={() => void reveal()}
            disabled={busy() || paused()}
            title={paused() ? PAUSED_REASON : undefined}
          >
            Open folder
          </button>
          <PromptLink project={props.project} prompts={props.prompts} onChanged={props.onChanged} />
        </div>

        <Show when={missing()}>
          <p class="banner tint-caution mt-1 text-xs">{MISSING_EXPLANATION}</p>
        </Show>

        <Show when={note()}>
          {(message) => (
            <p role="status" class="mt-1 text-xs text-ink-muted">
              {message()}
            </p>
          )}
        </Show>
      </div>
    </article>
  );
}

function launchLabel(busy: boolean, missing: boolean, frontDoor: boolean): string {
  if (missing) return "Folder missing";
  if (!frontDoor) return "No page to launch";
  return busy ? "Opening…" : "Launch ↗";
}
