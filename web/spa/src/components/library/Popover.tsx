import { createEffect, createSignal, type JSX, on, onCleanup, Show } from "solid-js";
import { ChevronDown } from "~/components/icons";
import { createEscapeLayer } from "~/lib/keyboard";

const FOCUSABLE = "a[href], button:not(:disabled), input:not(:disabled), [tabindex='0']";

/** Breathing room kept between the panel and the edge of the window. */
const MARGIN = 8;

/** Below this much room, a side counts as too cramped to open into if the other is better. */
const MIN_PANEL = 220;

/** The panel's own furniture — filter box, padding, action row — above the scrolling list. */
const CHROME = 96;

/** A list shorter than this is not worth scrolling; the panel is allowed to be cramped. */
const MIN_LIST = 120;

interface Placement {
  side: "top" | "bottom";
  /** Pixels from the wrapper's left edge, after clamping into the viewport. */
  left: number;
  /** How tall the scrolling list may be. */
  list: number;
  /** How tall the panel itself may be, before it starts scrolling. */
  max: number;
  /** How wide it may be, so a panel wider than the window is narrowed rather than cut off. */
  maxWidth: number;
}

/** The scroll height a list had before any of this existed, used until one is measured. */
const RESTING_LIST = 256;

/**
 * The shortest a panel is allowed to be squeezed to.
 *
 * Below this it stops being a panel and becomes a slot, and scrolling a form through a
 * 60px window is worse than letting it run a little past the fold.
 */
const MIN_PANEL_HEIGHT = 140;

/**
 * The window, without the scrollbar.
 *
 * `innerWidth`/`innerHeight` count the scrollbar as usable space, and the page under these
 * popovers almost always has one. Clamping against them places a panel up to fifteen pixels
 * too far right — hard against the edge with its last column of text under the scrollbar,
 * which is exactly the "it fits, but part of it is obscured" case this whole function
 * exists to prevent. `clientWidth`/`clientHeight` on the root element are the same numbers
 * minus the scrollbars, which is what a panel actually has to fit inside. (`100vw` in CSS
 * has the same fault, and is why the width bound below is measured here instead.)
 */
function viewport(): { width: number; height: number } {
  const root = document.documentElement;
  return {
    width: root.clientWidth || window.innerWidth,
    height: root.clientHeight || window.innerHeight,
  };
}

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
  /**
   * Which way the panel opens. Defaults to `bottom`, which is right for a trigger sitting
   * at the top of the page.
   *
   * A trigger in the bulk bar is the opposite case: that bar is pinned to the bottom of the
   * viewport, so a panel opening downward has nowhere to go — it runs off the bottom edge,
   * and the bar shifts to try to make room for it. Those triggers pass `top` and the panel
   * grows up over the grid, which is the space that is actually there.
   */
  placement?: "top" | "bottom";
  /**
   * Carry the border at rest, rather than only once open.
   *
   * The library's filter row is a line of controls sitting on the bare ground with nothing
   * else to group them, and a borderless pill there reads as a label. The bulk bar's
   * triggers are already inside a card, where the same border would be a box in a box.
   */
  outlined?: boolean;
  /**
   * What the trigger is made of. `pill` is the chrome everywhere else in the app; `tool` is
   * the Prompt Helper's rail, where sixteen bordered pills side by side read as sixteen
   * competing buttons rather than as one instrument. The panel is identical either way.
   */
  variant?: "pill" | "tool";
  children: (close: () => void) => JSX.Element;
}) {
  /**
   * Where the panel sits before it has been measured: anchored to the trigger on the side
   * the caller asked for, with the list at its old fixed height. That way the first paint is
   * the previous, well-behaved-in-the-common-case placement rather than something visibly
   * wrong that then jumps. The real measurement lands in the same frame.
   */
  const resting = (): Placement => ({
    side: props.placement === "top" ? "top" : "bottom",
    left: 0,
    list: RESTING_LIST,
    max: viewport().height,
    maxWidth: viewport().width - MARGIN * 2,
  });

  const [open, setOpen] = createSignal(false);
  const [place, setPlace] = createSignal<Placement>(resting());
  let wrapper: HTMLDivElement | undefined;
  let trigger: HTMLButtonElement | undefined;
  let panel: HTMLDivElement | undefined;

  const close = () => {
    setOpen(false);
    trigger?.focus();
  };

  /**
   * Fit the panel to the room that actually exists around the trigger.
   *
   * Anchoring alone is not enough. A trigger near the right edge of the window opens a
   * 17rem panel straight off the side of the screen, and one near the bottom opens a long
   * list past the fold — in both cases the part that is cut off is silently unreachable,
   * because the panel is clipped rather than scrolled to.
   *
   * So three things are decided here, all from one measurement:
   *
   * * **Side** — `props.placement` is a preference, not an instruction. It is honoured
   *   whenever the panel fits, and overruled when the other side has meaningfully more
   *   room, which is what makes a picker near the bottom of the page open upward by itself.
   * * **Left** — the panel is clamped into the viewport with a margin, so it slides along
   *   the trigger rather than hanging off the edge.
   * * **`--option-max`** — whatever vertical room is left after the panel's own chrome
   *   becomes the list's scroll height, so a long vocabulary scrolls *inside* the panel
   *   instead of extending it past the edge.
   * * **`max-height`** — the same room, applied to the panel itself. `--option-max` only
   *   reaches panels whose tall part is an `.option-scroll` list; one built out of a form
   *   has no such element and would run straight off the bottom of a short window with the
   *   overflowing part silently unreachable. Bounding the panel makes the *panel* scroll in
   *   that case, which is the only thing that works for content this cannot know the shape
   *   of.
   */
  const measure = () => {
    if (!trigger || !wrapper || !panel) return;

    const anchor = trigger.getBoundingClientRect();
    const frame = wrapper.getBoundingClientRect();
    const view = viewport();

    // Keyed by side name, so a chosen side indexes straight into the room it has.
    const room = {
      top: anchor.top - MARGIN,
      bottom: view.height - anchor.bottom - MARGIN,
    };

    const wanted = props.placement === "top" ? "top" : "bottom";
    const other = wanted === "top" ? "bottom" : "top";
    const side = room[wanted] < MIN_PANEL && room[other] > room[wanted] ? other : wanted;

    // `offsetWidth` rather than the `width` prop: the prop is a CSS length this code would
    // otherwise have to parse, and the panel is already laid out by the time this runs —
    // including the `max-width` below, so this is the width it will actually occupy rather
    // than the one it asked for.
    const width = panel.offsetWidth;
    const left = Math.max(MARGIN, Math.min(anchor.left, view.width - MARGIN - width));

    setPlace({
      side,
      // Back into the wrapper's coordinates, since the panel is positioned against it.
      left: left - frame.left,
      list: Math.max(MIN_LIST, room[side] - CHROME),
      max: Math.max(MIN_PANEL_HEIGHT, room[side]),
      // No floor, unlike the height: a panel narrowed past comfort is still readable, where
      // one squeezed past `MIN_PANEL_HEIGHT` is a slot. Width has no equivalent failure.
      maxWidth: view.width - MARGIN * 2,
    });
  };

  /**
   * Re-measure while open, because the room can change without the popover doing anything:
   * the window resizes, or the page scrolls and carries the trigger toward an edge.
   *
   * `capture` catches scrolls in any ancestor, not just the document — several of these
   * triggers live inside panes that scroll on their own.
   */
  createEffect(
    on(open, (isOpen) => {
      if (!isOpen) {
        setPlace(resting());
        return;
      }

      queueMicrotask(measure);
      window.addEventListener("resize", measure);
      window.addEventListener("scroll", measure, { capture: true, passive: true });
      onCleanup(() => {
        window.removeEventListener("resize", measure);
        window.removeEventListener("scroll", measure, { capture: true });
      });
    }),
  );

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
    <div ref={wrapper} class="relative">
      <button
        ref={trigger}
        type="button"
        class={props.variant === "tool" ? "tool-btn" : "pill"}
        classList={{
          "pill-current": props.variant !== "tool" && props.active,
          "pill-outline": props.variant !== "tool" && (open() || props.outlined),
        }}
        data-on={props.variant === "tool" && open() ? "true" : "false"}
        aria-expanded={open()}
        aria-haspopup="dialog"
        disabled={!!props.blocked}
        title={props.blocked}
        onClick={() => setOpen((was) => !was)}
      >
        {props.label}
        {/* The one thing the label cannot say: this opens something. Kept faint, because
            it is a hint about the control rather than part of what the control is named —
            and it points at the panel once the panel is there. */}
        <ChevronDown class={open() ? "chevron chevron-open" : "chevron"} />
      </button>

      <Show when={open()}>
        <div
          ref={panel}
          role="dialog"
          aria-label={props.title}
          tabindex="-1"
          class="panel float absolute z-40 flex flex-col gap-2 overflow-y-auto p-2 outline-none"
          style={{
            width: props.width ?? "17rem",
            /*
             * The horizontal half of the same problem. The left clamp slides a panel along
             * its trigger until it fits, but nothing slides a panel that is *wider than the
             * window* into view — on a narrow viewport a 17rem panel is simply too big, and
             * clamping only decides which of its edges gets cut off. This is what makes it
             * stop being too big, and it comes before the clamp: `offsetWidth` is read after
             * this applies, so the clamp is working with the width the panel really has.
             */
            "max-width": `${place().maxWidth}px`,
            "max-height": `${place().max}px`,
            left: `${place().left}px`,
            top: place().side === "bottom" ? "100%" : "auto",
            bottom: place().side === "top" ? "100%" : "auto",
            "margin-top": place().side === "bottom" ? "0.25rem" : "0",
            "margin-bottom": place().side === "top" ? "0.25rem" : "0",
            "--option-max": `${place().list}px`,
          }}
          onKeyDown={trapTab}
        >
          {props.children(close)}
        </div>
      </Show>
    </div>
  );
}
