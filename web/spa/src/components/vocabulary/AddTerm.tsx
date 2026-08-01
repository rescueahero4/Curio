import { createSignal, For, Show } from "solid-js";
import { ChevronRight } from "~/components/icons";
import { Popover } from "~/components/library/Popover";
import { createTerm } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { refreshVocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

/** What the menu offers, in the order the tabs name them. */
const KINDS: { kind: VocabularyKind; label: string; said: string }[] = [
  {
    kind: "families",
    label: "Aesthetic Family",
    said: "A group Curio sorts captures into, and judges new ones against.",
  },
  { kind: "types", label: "Design Type", said: "What a thing is — a landing page, a dashboard." },
  { kind: "tags", label: "Tag", said: "Anything else worth being able to search by." },
];

/**
 * Add a word by hand — to any of the three collections, from one control.
 *
 * This replaces a card that sat above the table and only ever added to the tab you were
 * already on. Two things were wrong with that. It spent a permanent block of the page on
 * the rarest action here — a tag or a type is normally created by using it on an item, not
 * by being typed into a form — and it made "add a design type" a two-step job that started
 * with changing tabs for a reason that had nothing to do with reading.
 *
 * So the collection is the first thing the menu asks and the form is the second, in the same
 * panel, anchored under the button that opened it. `Popover` already owns the hard parts of
 * that: it clamps itself into the viewport, traps Tab, closes on Escape ahead of the table's
 * selection, and returns focus to the trigger.
 *
 * Families ask for a description in the same breath as the name, because that description is
 * the rubric Curio judges new captures against — a family created without one matches on its
 * name alone, and nothing later says so.
 */
export function AddTerm(props: {
  /** Show what was just added. Adding a tag from the families tab is otherwise invisible. */
  onAdded: (kind: VocabularyKind) => void;
}) {
  return (
    <Popover label="Add" title="Add to the vocabulary" width="19rem" outlined>
      {(close) => <AddPanel close={close} onAdded={props.onAdded} />}
    </Popover>
  );
}

/**
 * The panel's two steps.
 *
 * Split from the trigger so that closing the popover unmounts it, which is what resets the
 * menu: a panel reopened after adding a tag should ask which collection again rather than
 * reopening on the form that was last used.
 */
function AddPanel(props: { close: () => void; onAdded: (kind: VocabularyKind) => void }) {
  const [chosen, setChosen] = createSignal<(typeof KINDS)[number] | null>(null);

  return (
    <Show
      when={chosen()}
      fallback={
        <For each={KINDS}>
          {(entry) => (
            <button type="button" class="menu-item" onClick={() => setChosen(entry)}>
              <span class="flex min-w-0 flex-1 flex-col">
                <span>{entry.label}</span>
                <span class="text-ink-faint text-xs">{entry.said}</span>
              </span>
              <ChevronRight class="chevron" />
            </button>
          )}
        </For>
      }
    >
      {(entry) => (
        <NewTermForm
          kind={entry().kind}
          noun={entry().label}
          onBack={() => setChosen(null)}
          onAdded={() => {
            props.onAdded(entry().kind);
            props.close();
          }}
        />
      )}
    </Show>
  );
}

/** The fields one collection needs, and nothing the other two would have added. */
function NewTermForm(props: {
  kind: VocabularyKind;
  noun: string;
  onBack: () => void;
  onAdded: () => void;
}) {
  const [name, setName] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [problem, setProblem] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const blocked = () => {
    if (paused()) return "Curio is paused. Resume from the tray icon to add names.";
    if (busy()) return "Adding…";
    if (!name().trim()) return `Type a ${props.noun.toLowerCase()} first.`;
    return undefined;
  };

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (blocked()) return;

    setBusy(true);
    setProblem(null);
    try {
      await createTerm(props.kind, {
        name: name().trim(),
        ...(props.kind === "families" ? { description: description().trim() } : {}),
      });
      await refreshVocabulary();
      setName("");
      setDescription("");
      props.onAdded();
    } catch (error) {
      setProblem(error instanceof ApiError ? error.message : "Could not add that.");
    }
    setBusy(false);
  }

  return (
    <form class="flex flex-col gap-2" onSubmit={submit}>
      {/* The panel does not carry a title of its own, so this row is both the heading and
          the way back — a user who picked the wrong collection should not have to close the
          popover and start again. */}
      <div class="flex items-center gap-1 border-line border-b pb-2">
        <button type="button" class="pill pill-icon" onClick={props.onBack}>
          <ChevronRight class="chevron rotate-180" />
          <span class="sr-only">Back to the list of collections</span>
        </button>
        <span class="font-medium text-sm">New {props.noun}</span>
      </div>

      <label class="flex flex-col gap-1 text-sm">
        <span class="text-ink-muted">Name</span>
        {/*
         * Focused explicitly rather than with `autofocus`, which browsers honour
         * inconsistently on an element inserted after load — and this one always is. It also
         * has to happen: the menu button that was clicked is being unmounted underneath the
         * pointer, so without this, focus lands on `<body>` and the popover's Tab trap has
         * nothing inside it to hold.
         */}
        <input
          ref={(node) => queueMicrotask(() => node.focus())}
          type="text"
          class="field field-block"
          value={name()}
          onInput={(event) => setName(event.currentTarget.value)}
        />
      </label>

      <Show when={props.kind === "families"}>
        <label class="flex flex-col gap-1 text-sm">
          <span class="text-ink-muted">Description</span>
          <textarea
            class="field field-block"
            rows="3"
            value={description()}
            onInput={(event) => setDescription(event.currentTarget.value)}
          />
          <span class="text-ink-faint text-xs">
            What Curio matches against, and what the chip expands to in a prompt.
          </span>
        </label>
      </Show>

      <button type="submit" class="pill pill-ink self-end" disabled={!!blocked()} title={blocked()}>
        Add
      </button>

      <Show when={problem()}>
        <output class="banner tint-caution">{problem()}</output>
      </Show>
    </form>
  );
}
