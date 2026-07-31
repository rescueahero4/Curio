/**
 * The frame a settings section renders inside: a card, a heading, a save badge, and the two
 * controls every section reaches for.
 *
 * The badge sits in the section header rather than at the top of the page because a save
 * here is scoped to one section — a page-level "Saved" would leave the user guessing which
 * of eight things it meant.
 */

import { createUniqueId, type JSX, Show } from "solid-js";
import type { Saver } from "~/components/settings/save";

export function Section(props: {
  title: string;
  blurb?: JSX.Element;
  saver?: Saver;
  children: JSX.Element;
}) {
  return (
    <section class="card flex flex-col gap-3 p-5">
      <header class="flex flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h2 class="text-lg font-semibold">{props.title}</h2>
        <Show when={props.saver}>{(saver) => <SaveBadge saver={saver()} />}</Show>
      </header>
      <Show when={props.blurb}>
        <p class="max-w-prose text-sm text-ink-muted">{props.blurb}</p>
      </Show>
      {props.children}
    </section>
  );
}

export function SaveBadge(props: { saver: Saver }) {
  const state = () => props.saver.state();
  const undoable = () => {
    const current = state();
    return current.kind === "saved" && current.undoable;
  };
  const refusal = () => {
    const current = state();
    return current.kind === "refused" ? current.message : null;
  };

  return (
    <span class="flex flex-wrap items-center gap-2 text-xs">
      <Show when={state().kind === "saving"}>
        <span class="text-ink-faint">Saving…</span>
      </Show>

      <Show when={state().kind === "saved"}>
        <span class="text-confirm">Saved</span>
        <Show when={undoable()}>
          <button type="button" class="pill pill-outline" onClick={() => props.saver.undo()}>
            Undo
          </button>
        </Show>
      </Show>

      <Show when={state().kind === "reverted"}>
        <span class="text-ink-muted">Put back.</span>
      </Show>

      <Show when={state().kind === "paused"}>
        <span class="text-caution">
          Curio is paused, so this did not save. Resume from the tray icon and try again.
        </span>
      </Show>

      <Show when={refusal()}>
        {(message) => (
          <span role="alert" class="text-caution">
            {message()}
          </span>
        )}
      </Show>
    </span>
  );
}

/**
 * A labelled field. The hint sits under the control, where a reader looking at it will be.
 *
 * The child is a function of the generated id so the label can point at the control by
 * `for` rather than by containment — a wrapping label reads as unlabelled to the linter and,
 * more to the point, to anything walking the accessibility tree by relationship.
 */
export function Field(props: {
  label: string;
  hint?: JSX.Element;
  children: (id: string) => JSX.Element;
}) {
  const id = createUniqueId();

  return (
    <div class="flex flex-col gap-1">
      <label class="text-sm font-medium" for={id}>
        {props.label}
      </label>
      {props.children(id)}
      <Show when={props.hint}>
        <span class="text-xs text-ink-faint">{props.hint}</span>
      </Show>
    </div>
  );
}

/** A toggle that always says why it is unavailable, because a silent one lies (PRD §5). */
export function CheckField(props: {
  label: string;
  checked: boolean;
  disabled?: boolean;
  reason?: string;
  onChange: (next: boolean) => void;
}) {
  return (
    <div class="flex flex-col gap-1">
      <label class="flex w-fit items-center gap-2 text-sm">
        <input
          type="checkbox"
          checked={props.checked}
          disabled={props.disabled}
          onChange={(event) => props.onChange(event.currentTarget.checked)}
        />
        <span>{props.label}</span>
      </label>
      <Show when={props.disabled && props.reason}>
        <span class="text-xs text-ink-faint">{props.reason}</span>
      </Show>
    </div>
  );
}
