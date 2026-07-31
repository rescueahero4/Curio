import { createEffect, createSignal, type JSX, on, onCleanup, Show } from "solid-js";
import { createEscapeLayer } from "~/lib/keyboard";

const FOCUSABLE = "a[href], button:not(:disabled), input:not(:disabled), [tabindex='0']";

/**
 * The one popover the library uses: filter pills, bulk panels, the move-to picker.
 *
 * R-FE-21 asks for three things — focus in on open, Tab kept inside, focus back on the
 * trigger on close — and they are here rather than in each caller because the third one is
 * the one that gets forgotten: a popover that returns focus to `<body>` drops a keyboard
 * user at the top of the page every time they close a filter.
 *
 * Escape is registered as an `overlay` layer, so it closes this before it reaches the grid
 * selection (R-FE-20). Closing by pointer is deliberately *not* a focus return: the user's
 * attention is already wherever they clicked.
 */
export function Popover(props: {
  label: JSX.Element;
  title: string;
  active?: boolean;
  /** Why the trigger is off. Present means disabled — PRD §5: a disabled control says why. */
  blocked?: string;
  width?: string;
  children: (close: () => void) => JSX.Element;
}) {
  const [open, setOpen] = createSignal(false);
  let trigger: HTMLButtonElement | undefined;
  let panel: HTMLDivElement | undefined;

  const close = () => {
    setOpen(false);
    trigger?.focus();
  };

  createEscapeLayer("overlay", () => {
    if (!open()) return false;
    close();
    return true;
  });

  const onPointerDown = (event: PointerEvent) => {
    if (!open()) return;
    const target = event.target as Node | null;
    if (!target || panel?.contains(target) || trigger?.contains(target)) return;
    setOpen(false);
  };
  document.addEventListener("pointerdown", onPointerDown);
  onCleanup(() => document.removeEventListener("pointerdown", onPointerDown));

  createEffect(
    on(
      open,
      (isOpen) => {
        if (isOpen) queueMicrotask(() => panel?.focus());
      },
      { defer: true },
    ),
  );

  const trapTab = (event: KeyboardEvent) => {
    if (event.key !== "Tab" || !panel) return;
    const stops = Array.from(panel.querySelectorAll<HTMLElement>(FOCUSABLE));
    const first = stops[0];
    const last = stops[stops.length - 1];
    if (!first || !last) return;

    const active = document.activeElement;
    if (event.shiftKey && (active === first || active === panel)) {
      event.preventDefault();
      last.focus();
    } else if (!event.shiftKey && active === last) {
      event.preventDefault();
      first.focus();
    }
  };

  return (
    <div class="relative">
      <button
        ref={trigger}
        type="button"
        class="pill"
        classList={{ "pill-current": props.active, "pill-outline": open() }}
        aria-expanded={open()}
        aria-haspopup="dialog"
        disabled={!!props.blocked}
        title={props.blocked}
        onClick={() => setOpen((was) => !was)}
      >
        {props.label}
      </button>

      <Show when={open()}>
        <div
          ref={panel}
          role="dialog"
          aria-label={props.title}
          tabindex="-1"
          class="card absolute left-0 z-30 mt-1 flex flex-col gap-2 p-2 outline-none"
          style={{ width: props.width ?? "17rem" }}
          onKeyDown={trapTab}
        >
          {props.children(close)}
        </div>
      </Show>
    </div>
  );
}
