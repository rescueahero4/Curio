import { createEffect, createSignal, on, onCleanup, Show } from "solid-js";
import { createItem } from "~/lib/api";
import { ApiError, paused } from "~/lib/http";
import { createEscapeLayer } from "~/lib/keyboard";

/**
 * Add an item by drop, paste or file picker (FR-2).
 *
 * A screenshot is mandatory — there is no item without one — so the submit button is
 * disabled until there is a file, and it says so. Same for the paused state. PRD §5 makes
 * that a requirement rather than a nicety: a disabled control that does not explain itself
 * is indistinguishable from a broken one.
 *
 * Paste is bound to the document while the dialog is open, because a screenshot on the
 * clipboard has no element to be dropped on.
 */
export function AddItemDialog(props: { open: boolean; onClose: () => void }) {
  const [file, setFile] = createSignal<File | null>(null);
  const [preview, setPreview] = createSignal<string | null>(null);
  const [sourceUrl, setSourceUrl] = createSignal("");
  const [title, setTitle] = createSignal("");
  const [over, setOver] = createSignal(false);
  const [busy, setBusy] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);

  let panel: HTMLDivElement | undefined;
  let opener: HTMLElement | null = null;

  createEscapeLayer("overlay", () => {
    if (!props.open || busy()) return false;
    close();
    return true;
  });

  // Focus moves in on open and returns to whatever opened the dialog on close (R-FE-21).
  createEffect(
    on(
      () => props.open,
      (open) => {
        if (open) {
          opener = document.activeElement as HTMLElement | null;
          queueMicrotask(() => panel?.focus());
          return;
        }
        reset();
        opener?.focus();
        opener = null;
      },
      { defer: true },
    ),
  );

  const onPaste = (event: ClipboardEvent) => {
    if (!props.open) return;
    const pasted = Array.from(event.clipboardData?.files ?? []).find(isImage);
    if (pasted) accept(pasted);
  };
  document.addEventListener("paste", onPaste);
  onCleanup(() => {
    document.removeEventListener("paste", onPaste);
    revoke();
  });

  function accept(next: File) {
    revoke();
    setFile(next);
    setPreview(URL.createObjectURL(next));
    setProblem(null);
  }

  function revoke() {
    const url = preview();
    if (url) URL.revokeObjectURL(url);
    setPreview(null);
  }

  function reset() {
    revoke();
    setFile(null);
    setSourceUrl("");
    setTitle("");
    setProblem(null);
    setOver(false);
  }

  function close() {
    if (busy()) return;
    props.onClose();
  }

  /** Why the submit button is off, or `undefined` when it is on. */
  const blocked = () => {
    if (paused()) return "Curio is paused. Resume from the tray icon to add items.";
    if (!file()) return "Add a screenshot first — every item needs one.";
    if (busy()) return "Adding…";
    return undefined;
  };

  async function submit(event: SubmitEvent) {
    event.preventDefault();
    const screenshot = file();
    if (!screenshot || busy() || paused()) return;

    setBusy(true);
    setProblem(null);
    try {
      await createItem({
        screenshot,
        source_url: sourceUrl().trim() || undefined,
        title: title().trim() || undefined,
      });
      setBusy(false);
      props.onClose();
    } catch (error) {
      setBusy(false);
      setProblem(explain(error));
    }
  }

  return (
    <Show when={props.open}>
      <div class="fixed inset-0 z-40 flex items-start justify-center p-6 pt-24">
        {/* A real button, not a click-handling div: the backdrop is a way out of the
            dialog, and a way out has to be reachable by keyboard as well as by mouse. */}
        <button
          type="button"
          class="absolute inset-0 bg-ink/20"
          aria-label="Close without adding"
          onClick={close}
        />
        <div
          ref={panel}
          role="dialog"
          aria-modal="true"
          aria-label="Add an item"
          tabindex="-1"
          class="sheet relative w-full max-w-lg p-5 outline-none"
        >
          <h2 class="text-lg font-semibold">Add an item</h2>
          <p class="mt-1 text-sm text-ink-muted">
            Drop an image, paste one, or choose a file. Curio describes it once it lands.
          </p>

          <form class="mt-4 flex flex-col gap-3" onSubmit={submit}>
            <DropZone
              over={over()}
              preview={preview()}
              name={file()?.name ?? null}
              onOver={setOver}
              onFile={accept}
            />

            <label class="flex flex-col gap-1 text-sm">
              <span class="text-ink-muted">Source URL (optional)</span>
              <input
                type="url"
                class="field field-block"
                placeholder="https://"
                value={sourceUrl()}
                onInput={(event) => setSourceUrl(event.currentTarget.value)}
              />
            </label>

            <label class="flex flex-col gap-1 text-sm">
              <span class="text-ink-muted">Title (optional — Curio suggests one)</span>
              <input
                type="text"
                class="field field-block"
                value={title()}
                onInput={(event) => setTitle(event.currentTarget.value)}
              />
            </label>

            <Show when={problem()}>
              <output class="banner tint-caution">{problem()}</output>
            </Show>

            <div class="mt-1 flex items-center justify-end gap-2">
              <Show when={blocked()}>
                <span class="mr-auto text-xs text-ink-faint">{blocked()}</span>
              </Show>
              <button type="button" class="pill pill-outline" onClick={close} disabled={busy()}>
                Cancel
              </button>
              <button type="submit" class="pill pill-ink" disabled={!!blocked()} title={blocked()}>
                {busy() ? "Adding…" : "Add item"}
              </button>
            </div>
          </form>
        </div>
      </div>
    </Show>
  );
}

function DropZone(props: {
  over: boolean;
  preview: string | null;
  name: string | null;
  onOver: (over: boolean) => void;
  onFile: (file: File) => void;
}) {
  let picker: HTMLInputElement | undefined;

  return (
    <>
      <button
        type="button"
        class="card flex w-full flex-col items-center gap-2 p-4 text-sm text-ink-muted"
        classList={{ "border-line-strong": props.over }}
        onClick={() => picker?.click()}
        onDragOver={(event) => {
          event.preventDefault();
          props.onOver(true);
        }}
        onDragLeave={() => props.onOver(false)}
        onDrop={(event) => {
          event.preventDefault();
          props.onOver(false);
          const dropped = Array.from(event.dataTransfer?.files ?? []).find(isImage);
          if (dropped) props.onFile(dropped);
        }}
      >
        <Show
          when={props.preview}
          fallback={<span>Drop an image here, paste, or choose a file</span>}
        >
          {(url) => <img src={url()} alt="" class="shot max-h-56 object-contain" />}
        </Show>
        <Show when={props.name}>
          <span class="text-xs text-ink-faint">{props.name}</span>
        </Show>
      </button>
      <input
        ref={picker}
        type="file"
        accept="image/*"
        class="hidden"
        onChange={(event) => {
          const chosen = event.currentTarget.files?.[0];
          if (chosen) props.onFile(chosen);
          event.currentTarget.value = "";
        }}
      />
    </>
  );
}

function isImage(file: File): boolean {
  return file.type.startsWith("image/");
}

function explain(error: unknown): string {
  if (!(error instanceof ApiError)) return "Could not add the item.";
  if (error.isPaused) return "Curio is paused. Resume from the tray icon and try again.";
  if (error.sessionExpired) return "Curio restarted. Open the dashboard from the tray again.";
  if (error.unreachable) return "Curio is not answering. Is it still running?";
  return error.message;
}
