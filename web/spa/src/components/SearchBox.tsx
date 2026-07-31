import { useLocation, useNavigate, useSearchParams } from "@solidjs/router";
import { createSignal } from "solid-js";
import { createShortcut, isApplePlatform } from "~/lib/keyboard";

/**
 * The centred search (PRD §5), and Cmd/Ctrl-K.
 *
 * Search lives in the URL rather than in a signal here. That is what makes a search
 * result something the user can reload, bookmark or hand to someone else — and it is also
 * what lets the grid own the 200 ms debounce (R-FE-9) without this component knowing
 * anything about timing.
 *
 * Typing from any route lands on the Library, because that is the only place results can
 * be shown. The navigation replaces rather than pushes once already there, so Back means
 * "the page before I started searching" instead of one entry per keystroke.
 */
export function SearchBox() {
  const [searchParams] = useSearchParams();
  const navigate = useNavigate();
  const location = useLocation();

  let input: HTMLInputElement | undefined;
  const [focused, setFocused] = createSignal(false);

  createShortcut({ key: "k", mod: true }, () => {
    input?.focus();
    input?.select();
  });

  const commit = (value: string) => {
    const onLibrary = location.pathname === "/";
    const target = value ? `/?q=${encodeURIComponent(value)}` : "/";
    navigate(target, { replace: onLibrary });
  };

  return (
    <search class="w-full">
      <label class="relative flex items-center">
        <span class="sr-only">Search the library</span>
        <input
          ref={input}
          type="search"
          class="field pr-16"
          placeholder="Search the library"
          value={typeof searchParams.q === "string" ? searchParams.q : ""}
          onInput={(event) => commit(event.currentTarget.value)}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
        />
        {/* The hint disappears on focus: once the field has the caret, the shortcut that
            got it there is noise. */}
        <kbd
          class="pointer-events-none absolute right-3 text-2xs text-ink-faint transition-opacity"
          classList={{ "opacity-0": focused() }}
          style={{ "transition-duration": "var(--duration-hover)" }}
        >
          {isApplePlatform() ? "⌘K" : "Ctrl K"}
        </kbd>
      </label>
    </search>
  );
}
