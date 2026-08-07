import { createSignal, For, Show } from "solid-js";
import { ChevronRight } from "~/components/icons";
import { Popover } from "~/components/library/Popover";
import { refusal } from "~/components/vocabulary/errors";
import { createTerm } from "~/lib/api";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";
import { refreshVocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

/** What the menu offers, in the order the tabs name them. */
const KINDS: VocabularyKind[] = ["families", "types", "tags"];

/**
 * What the menu calls a collection, and what a sentence calls it.
 *
 * Two different words on purpose: the menu names the thing about to be made, in the phrasing
 * the PRD uses for it, and the hint under a disabled button needs the same idea inside a
 * sentence. Both are looked up here rather than stored on `KINDS`, because that array is
 * built once at import and would have kept whichever language was loaded then.
 */
function menuLabel(kind: VocabularyKind): string {
  if (kind === "families") return t("vocabulary.add.kinds.families");
  if (kind === "types") return t("vocabulary.add.kinds.types");
  return t("vocabulary.add.kinds.tags");
}

function singular(kind: VocabularyKind): string {
  if (kind === "families") return t("vocabulary.kinds.families.one");
  if (kind === "types") return t("vocabulary.kinds.types.one");
  return t("vocabulary.kinds.tags.one");
}

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
    <Popover label={t("vocabulary.add.label")} title={t("vocabulary.add.title")} outlined>
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
  /*
   * The kind, not the menu entry it was clicked from. Holding the entry would hold the label
   * that was on it at the time, and the form below would go on saying it after a language
   * change; the kind is the part that does not depend on the language.
   */
  const [chosen, setChosen] = createSignal<VocabularyKind | null>(null);

  return (
    <Show
      when={chosen()}
      fallback={
        <For each={KINDS}>
          {(kind) => (
            <button type="button" class="menu-item" onClick={() => setChosen(kind)}>
              <span class="min-w-0 flex-1 truncate">{menuLabel(kind)}</span>
              <ChevronRight class="chevron" />
            </button>
          )}
        </For>
      }
    >
      {(kind) => (
        <NewTermForm
          kind={kind()}
          onBack={() => setChosen(null)}
          onAdded={() => {
            props.onAdded(kind());
            props.close();
          }}
        />
      )}
    </Show>
  );
}

/** The fields one collection needs, and nothing the other two would have added. */
function NewTermForm(props: { kind: VocabularyKind; onBack: () => void; onAdded: () => void }) {
  const [name, setName] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [problem, setProblem] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const blocked = () => {
    if (paused()) return t("vocabulary.add.paused");
    if (busy()) return t("vocabulary.add.busy");
    /* The in-sentence noun rather than the menu's Title Case one lower-cased. `toLowerCase`
       is a no-op on a language without case, and it was only ever standing in for a word
       this dictionary can simply hold. */
    if (!name().trim()) return t("vocabulary.add.needName", { noun: singular(props.kind) });
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
      /* Adding a name that already exists is a 409 exactly like renaming onto one, and it
         used to arrive here as the server's English. Both answers are overridden: this form
         has no merge control to send the reader to, and it knows it was adding rather than
         changing when something other than the API throws. */
      setProblem(
        refusal(error, {
          fallback: t("vocabulary.add.failed"),
          taken: t("vocabulary.add.taken"),
        }),
      );
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
          <span class="sr-only">{t("vocabulary.add.back")}</span>
        </button>
        <span class="font-medium text-sm">
          {t("vocabulary.add.heading", { noun: menuLabel(props.kind) })}
        </span>
      </div>

      <label class="flex flex-col gap-1 text-sm">
        <span class="text-ink-muted">{t("vocabulary.fields.name")}</span>
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
          <span class="text-ink-muted">{t("vocabulary.fields.description")}</span>
          <textarea
            class="field field-block"
            rows="3"
            value={description()}
            onInput={(event) => setDescription(event.currentTarget.value)}
          />
          <span class="text-ink-faint text-xs">{t("vocabulary.add.descriptionHint")}</span>
        </label>
      </Show>

      <button type="submit" class="pill pill-ink self-end" disabled={!!blocked()} title={blocked()}>
        {t("vocabulary.add.submit")}
      </button>

      <Show when={problem()}>
        <output class="banner tint-caution">{problem()}</output>
      </Show>
    </form>
  );
}
