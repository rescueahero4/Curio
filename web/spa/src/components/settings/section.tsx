/**
 * The frame a settings section renders inside: a rule, a heading, a save badge, and the two
 * controls every section reaches for.
 *
 * Not a card. Nine stacked cards read as nine separate documents, and this page is one
 * document in one scroll — a hairline above each heading is enough to say where a section
 * starts, and it leaves the fields sitting on the page rather than inside a container that
 * has to be visually entered. The left-hand nav in `Settings.tsx` is what carries the
 * structure the cards used to imply, and `id` is the anchor it aims at.
 *
 * The badge sits in the section header rather than at the top of the page because a save
 * here is scoped to one section — a page-level "Saved" would leave the user guessing which
 * of nine things it meant.
 */

import { createUniqueId, type JSX, Show } from "solid-js";
import { SavedBadge } from "~/components/SavedBadge";
import type { Saver } from "~/components/settings/save";
import { t } from "~/lib/i18n";

export function Section(props: {
  /** Anchor target for the settings nav; see SECTION_NAV in routes/Settings.tsx. */
  id: string;
  title: string;
  blurb?: JSX.Element;
  saver?: Saver;
  children: JSX.Element;
}) {
  return (
    // `scroll-mt-24` keeps a jumped-to heading clear of the sticky app header.
    <section id={props.id} class="flex scroll-mt-24 flex-col gap-3 border-t border-line pt-6">
      <header class="flex min-h-5 flex-wrap items-baseline justify-between gap-x-4 gap-y-1">
        <h2 class="text-base font-semibold">{props.title}</h2>
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
  /** The two states that leave on a clock; everything else stays until it is replaced. */
  const confirmation = () => {
    const current = state();
    return current.kind === "saved" || current.kind === "reverted" ? current : null;
  };
  const refusal = () => {
    const current = state();
    return current.kind === "refused" ? current.message : null;
  };

  return (
    <span class="flex flex-wrap items-center gap-2 text-xs">
      <Show when={state().kind === "saving"}>
        <span class="text-ink-faint">{t("common.saving")}</span>
      </Show>

      {/* `keyed`, so a second save inside the first badge's eight seconds mounts a second
          badge rather than reusing one whose clock is nearly out. Each save gets its own. */}
      <Show when={confirmation()} keyed>
        {(current) => (
          <SavedBadge
            label={current.kind === "reverted" ? t("settings.save.reverted") : undefined}
            onUndo={
              current.kind === "saved" && current.undoable ? () => props.saver.undo() : undefined
            }
            onDismiss={() => props.saver.dismiss()}
          />
        )}
      </Show>

      <Show when={state().kind === "paused"}>
        <span class="text-caution">{t("settings.paused.notSaved")}</span>
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
