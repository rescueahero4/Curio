import { createSignal, Show } from "solid-js";
import { createTerm } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { refreshVocabulary } from "~/lib/stores";
import type { VocabularyKind } from "~/lib/types";

/**
 * Add a word by hand.
 *
 * Mostly for families: a tag or a type is created by using it on an item, but a family is
 * a thing you decide to have before anything belongs to it — and its description is what
 * Curio will judge new captures against, so it is written here first.
 */
export function NewTerm(props: { kind: VocabularyKind; noun: string }) {
  const [name, setName] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [problem, setProblem] = createSignal<string | null>(null);
  const [busy, setBusy] = createSignal(false);

  const blocked = () => {
    if (paused()) return "Curio is paused. Resume from the tray icon to add names.";
    if (busy()) return "Adding…";
    if (!name().trim()) return `Type a ${props.noun} first.`;
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
    } catch (error) {
      setProblem(error instanceof ApiError ? error.message : "Could not add that.");
    }
    setBusy(false);
  }

  return (
    <form class="card flex flex-col gap-2 p-3" onSubmit={submit}>
      <div class="flex flex-wrap items-end gap-2">
        <label class="flex flex-1 flex-col gap-1 text-sm">
          <span class="text-ink-muted">New {props.noun}</span>
          <input
            type="text"
            class="field field-block"
            value={name()}
            onInput={(event) => setName(event.currentTarget.value)}
          />
        </label>
        <button type="submit" class="pill pill-ink" disabled={!!blocked()} title={blocked()}>
          Add
        </button>
      </div>

      <Show when={props.kind === "families"}>
        <label class="flex flex-col gap-1 text-sm">
          <span class="text-ink-muted">
            Description — what Curio matches against, and what the chip expands to
          </span>
          <textarea
            class="field field-block"
            rows="2"
            value={description()}
            onInput={(event) => setDescription(event.currentTarget.value)}
          />
        </label>
      </Show>

      <Show when={problem()}>
        <output class="banner tint-caution">{problem()}</output>
      </Show>
    </form>
  );
}
