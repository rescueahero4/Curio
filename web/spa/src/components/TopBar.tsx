import { A, useNavigate } from "@solidjs/router";
import { BrandMark } from "~/components/BrandMark";
import { NavPills } from "~/components/NavPills";
import { SearchBox } from "~/components/SearchBox";
import { paused } from "~/lib/http";

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

  const pausedReason = () => (paused() ? "Curio is paused. Resume from the tray icon." : undefined);

  return (
    <header class="sticky top-0 z-30 border-b border-line bg-ground/90 backdrop-blur">
      <div class="mx-auto grid w-full max-w-7xl grid-cols-1 items-center gap-3 px-6 py-3 lg:grid-cols-[1fr_minmax(0,26rem)_1fr]">
        <div class="flex items-center gap-3">
          <A href="/" class="flex items-center gap-2 text-ink" aria-label="Curio — Library">
            <BrandMark class="h-5 w-5" />
            <span class="text-lg font-semibold tracking-tight">Curio</span>
          </A>
          <NavPills />
        </div>

        <SearchBox />

        <div class="flex items-center justify-start gap-1 lg:justify-end">
          <A
            href="/settings"
            class="pill"
            activeClass="pill-current"
            inactiveClass=""
            aria-label="Settings"
            title="Settings"
          >
            <GearIcon />
          </A>

          <button
            type="button"
            class="pill pill-ink"
            onClick={props.onAddItem}
            disabled={paused()}
            title={pausedReason()}
          >
            + Add Item
          </button>

          {/* Registering a project needs a folder to point at, which lives on the Projects
              page — so this opens that page with the register form already up, rather than
              offering a dialog here that would have to ask the same question twice. */}
          <button
            type="button"
            class="pill pill-outline"
            onClick={() => navigate("/projects?new=1")}
            disabled={paused()}
            title={pausedReason()}
          >
            New Project
          </button>
        </div>
      </div>
    </header>
  );
}

function GearIcon() {
  return (
    <svg
      viewBox="0 0 24 24"
      width="16"
      height="16"
      fill="none"
      stroke="currentColor"
      stroke-width="1.6"
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3.2" />
      <path d="M12 2.6v2.6M12 18.8v2.6M4.3 7.3l2.3 1.3M17.4 15.4l2.3 1.3M4.3 16.7l2.3-1.3M17.4 8.6l2.3-1.3" />
    </svg>
  );
}
