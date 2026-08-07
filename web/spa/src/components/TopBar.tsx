import { A, useNavigate } from "@solidjs/router";
import { createSignal, onCleanup, Show } from "solid-js";
import { BrandMark } from "~/components/BrandMark";
import { NavTabs } from "~/components/NavTabs";
import { SearchBox } from "~/components/SearchBox";
import { createPrompt } from "~/lib/api";
import { paused } from "~/lib/http";
import { t } from "~/lib/i18n";

/**
 * The top bar, in the order PRD §5 and Inventory §6 fix it:
 * mark · Library/Projects/Prompts · centred search · Settings · + Add Item · New Project.
 *
 * The two action buttons are the only place in the chrome that can start a mutation, so
 * they are also the only place the paused state has to reach: both stay visible, disabled,
 * and say why (D25 keeps the library readable — hiding the buttons would make a paused app
 * look like a broken one).
 */
export function TopBar(props: { onAddItem: () => void }) {
  const navigate = useNavigate();
  const scrolled = createScrolled();
  const [starting, setStarting] = createSignal(false);
  const [problem, setProblem] = createSignal<string | null>(null);

  const pausedReason = () => (paused() ? t("shell.actions.pausedReason") : undefined);

  /**
   * "New Project" starts a *prompt*, not a folder (FR-16, Inventory §6).
   *
   * Curio never creates a project directory — an agent does, and the watcher adopts it.
   * So the only thing the user can start from here is the brief: compose it, send it to
   * Claude, and the sent prompt claims the folder that appears (6 h window). Pointing this
   * button at the register-a-folder form inverted that — it asked for the finished thing
   * as the first step.
   */
  async function newProject() {
    if (starting()) return;
    setStarting(true);
    setProblem(null);
    try {
      const prompt = await createPrompt();
      navigate(`/prompts/${prompt.id}`);
    } catch (failure) {
      // Said out loud rather than swallowed: a button that quietly does nothing is
      // indistinguishable from a hang.
      setProblem(failure instanceof Error ? failure.message : t("shell.actions.newProjectFailed"));
    } finally {
      setStarting(false);
    }
  }

  return (
    // The hairline is always there: it is what the section tabs sit on, and a tab strip
    // with nothing under it has no baseline to be a strip of.
    //
    // What is still earned by scrolling is the *frosted* treatment. A border says where the
    // header ends; it cannot say which of two overlapping layers is on top, and that
    // question only exists once content is passing underneath.
    <header
      class="sticky top-0 z-30 border-b border-line transition-[background-color,box-shadow]"
      classList={{
        "glass float": scrolled(),
        "bg-ground": !scrolled(),
      }}
      style={{
        "transition-duration": "var(--duration-hover)",
        "transition-timing-function": "var(--ease-hover)",
      }}
    >
      <div class="mx-auto grid w-full max-w-page grid-cols-1 items-center gap-3 px-6 py-0 lg:grid-cols-[1fr_minmax(0,26rem)_1fr]">
        <div class="flex items-center gap-3">
          <A href="/" class="flex items-center gap-2 text-ink" aria-label={t("shell.brand.home")}>
            <BrandMark class="h-6 w-6" />
            <span class="text-lg font-semibold tracking-tight">Curio</span>
          </A>
          <NavTabs />
        </div>

        <SearchBox />

        <div class="flex items-center justify-start gap-1 lg:justify-end">
          {/* The bare glyph, with no pill around it.

              Settings is the one control here that is not an action — it is a door to
              another page, sitting next to two buttons that do something. Giving it the
              same pill chrome as "+ Add Item" put it in a row of three things that looked
              equally like buttons. What it still keeps is feedback: the icon carries muted
              ink at rest, full ink on hover and on the settings page itself, so it is
              legibly a control rather than an ornament. Focus is unaffected — the ring
              comes from the global `:focus-visible` rule, not from the pill. */}
          <A
            href="/settings"
            class="inline-flex items-center rounded-full p-1.5 transition-[color] hover:text-ink"
            activeClass="text-ink"
            inactiveClass="text-ink-muted"
            aria-label={t("shell.actions.settings")}
            title={t("shell.actions.settings")}
            style={{
              "transition-duration": "var(--duration-hover)",
              "transition-timing-function": "var(--ease-hover)",
            }}
          >
            <GearIcon />
          </A>

          <button
            type="button"
            class="pill pill-ink py-1 px-3"
            onClick={props.onAddItem}
            disabled={paused()}
            title={pausedReason()}
          >
            {t("shell.actions.addItem")}
          </button>

          <button
            type="button"
            class="pill pill-outline py-1 px-3"
            onClick={() => void newProject()}
            disabled={paused() || starting()}
            title={pausedReason()}
          >
            {starting() ? t("shell.actions.newProjectStarting") : t("shell.actions.newProject")}
          </button>
        </div>
      </div>

      {/* The chrome has nowhere else to put a failure, and the two action buttons are the
          only things here that can fail. A row under the bar keeps the message next to the
          control that produced it without displacing the page. */}
      <Show when={problem()}>
        {(message) => (
          <div class="mx-auto w-full max-w-page px-6 pb-2">
            <p role="alert" class="banner tint-caution">
              <span>{message()}</span>
              <button type="button" class="pill pill-outline" onClick={() => setProblem(null)}>
                {t("common.dismiss")}
              </button>
            </p>
          </div>
        )}
      </Show>
    </header>
  );
}

/**
 * True once the page has scrolled off the top.
 *
 * The threshold is a few pixels rather than zero so that the elastic overscroll some
 * trackpads produce at rest does not flicker the header's treatment on and off.
 */
function createScrolled(threshold = 4) {
  const [scrolled, setScrolled] = createSignal(false);
  const read = () => setScrolled(window.scrollY > threshold);

  read();
  window.addEventListener("scroll", read, { passive: true });
  onCleanup(() => window.removeEventListener("scroll", read));

  return scrolled;
}

/**
 * A gear: a toothed ring around a hub.
 *
 * Worth naming what was wrong before, because the mistake is an easy one to repeat. The
 * previous icon was a small circle with six straight spokes radiating out of it — which is
 * the universal drawing of a sun, and reads as a theme or brightness toggle. Settings has
 * one conventional glyph and this is it: the teeth are stubby and squared onto the rim, and
 * the hub is a hole through the middle rather than a filled centre.
 */
function GearIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.5"
      stroke-linejoin="round"
      aria-hidden="true"
    >
      <path d="M 10.17 3.19 L 13.83 3.19 L 13.3 5.73 L 15.51 6.65 L 16.94 4.48 L 19.52 7.06 L 17.35 8.49 L 18.27 10.7 L 20.81 10.17 L 20.81 13.83 L 18.27 13.3 L 17.35 15.51 L 19.52 16.94 L 16.94 19.52 L 15.51 17.35 L 13.3 18.27 L 13.83 20.81 L 10.17 20.81 L 10.7 18.27 L 8.49 17.35 L 7.06 19.52 L 4.48 16.94 L 6.65 15.51 L 5.73 13.3 L 3.19 13.83 L 3.19 10.17 L 5.73 10.7 L 6.65 8.49 L 4.48 7.06 L 7.06 4.48 L 8.49 6.65 L 10.7 5.73 Z" />
      <circle cx="12" cy="12" r="2.9" />
    </svg>
  );
}
