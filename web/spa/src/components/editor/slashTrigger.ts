/**
 * When `/` opens the menu, and when it must not (R-FE-17).
 *
 * One rule with one famous counter-example: the slash has to be at the start of a line or
 * preceded by a space. That single test is what keeps the menu shut inside `http://` —
 * the character before either slash is `:` or `/`, never a space — and pasting a URL into
 * a brief is common enough that the old app encoded the rule rather than hoping.
 *
 * The plugin reports the run and nothing more. It never focuses anything, never renders,
 * and hands keystrokes back to whoever is drawing the menu, so the caret stays in the
 * document while the arrow keys drive a list somewhere else (R-FE-21).
 *
 * ## Why the two stages are a colon in the text
 *
 * A run reads `/aesthetic:warm`, and the colon is really there — the menu's stage is not
 * held in a signal beside the editor. This looks like the more complicated option and is
 * the simpler one, so it is worth saying why before someone tidies it away:
 *
 * - **The caret never leaves the paragraph.** Stage two is reached by writing a colon into
 *   the document, not by moving focus into a search field. That is what makes R-FE-21's
 *   "keyboard-driven without stealing focus from the editor caret" fall out of the design
 *   rather than being defended against every keystroke.
 * - **Undo, backspace and selection keep working.** Backspacing over the colon returns to
 *   the palette because the text says so; no state machine has to be told.
 * - **The user can see what they are filtering.** Hidden stage state shows a filtered list
 *   with no visible cause, and there is nothing to point at when it filters wrongly.
 *
 * The alternative — deleting the run on palette selection and buffering further keystrokes
 * in the menu — silently swallows typing that the document never records.
 */

import { Extension } from "@tiptap/core";
import type { EditorState } from "@tiptap/pm/state";
import { Plugin, PluginKey } from "@tiptap/pm/state";
import type { EditorView } from "@tiptap/pm/view";

/** The live `/…` run: what was typed, and where it sits in the document. */
export interface SlashTrigger {
  query: string;
  from: number;
  to: number;
  /** Viewport coordinates of the slash, for a `position: fixed` menu. */
  left: number;
  top: number;
  bottom: number;
}

export interface SlashOptions {
  onChange: (trigger: SlashTrigger | null) => void;
  /** Returns whether the menu consumed the key. Anything else reaches the document. */
  onKeyDown: (event: KeyboardEvent) => boolean;
}

interface Run {
  query: string;
  from: number;
  to: number;
}

interface State {
  run: Run | null;
  /** The `from` of a run the user dismissed, so Escape does not immediately re-open it. */
  dismissed: number | null;
}

const key = new PluginKey<State>("curio-slash");

/** Start of a text block, an ordinary space, or a non-breaking space. Nothing else. */
const ALLOWED_PREFIXES = ["", " ", " "];

/** What `textBetween` substitutes for an atom, so chip widths do not move the slash. */
const ATOM_PLACEHOLDER = "￼";

export const Slash = Extension.create<SlashOptions>({
  name: "slash",

  addOptions() {
    return { onChange: () => {}, onKeyDown: () => false };
  },

  addProseMirrorPlugins() {
    const { onChange, onKeyDown } = this.options;

    return [
      new Plugin<State>({
        key,

        state: {
          init: (_config, state) => ({ run: detect(state), dismissed: null }),
          apply: (transaction, previous, _old, next) => {
            const run = detect(next);
            if (transaction.getMeta(key)) return { run, dismissed: run ? run.from : null };
            const stale = previous.dismissed !== null && run?.from === previous.dismissed;
            return { run, dismissed: stale ? previous.dismissed : null };
          },
        },

        view: () => ({
          update(view, previous) {
            const now = visible(key.getState(view.state));
            const before = visible(key.getState(previous));
            if (now?.from === before?.from && now?.query === before?.query) return;
            onChange(now ? anchor(view, now) : null);
          },
          destroy: () => onChange(null),
        }),

        props: {
          handleKeyDown: (view, event) =>
            visible(key.getState(view.state)) ? onKeyDown(event) : false,
        },
      }),
    ];
  },
});

/** Close the menu without touching the text the user typed. */
export function dismissSlash(view: EditorView): void {
  view.dispatch(view.state.tr.setMeta(key, true));
}

function visible(state: State | null | undefined): Run | null {
  if (!state || state.dismissed !== null) return null;
  return state.run;
}

function anchor(view: EditorView, run: Run): SlashTrigger {
  const coords = view.coordsAtPos(run.from);
  return { ...run, left: coords.left, top: coords.top, bottom: coords.bottom };
}

function detect(state: EditorState): Run | null {
  const { $from, empty } = state.selection;
  if (!empty || $from.parent.type.spec.code) return null;

  const before = $from.parent.textBetween(0, $from.parentOffset, undefined, ATOM_PLACEHOLDER);
  const slash = before.lastIndexOf("/");
  if (slash < 0) return null;

  const preceding = slash === 0 ? "" : before.charAt(slash - 1);
  if (!ALLOWED_PREFIXES.includes(preceding)) return null;

  const query = before.slice(slash + 1);
  // A space ends the run: the user has moved on, and a menu that survived the space would
  // sit over the sentence they are now writing.
  if (/\s/.test(query)) return null;

  const start = $from.start();
  return { query, from: start + slash, to: start + $from.parentOffset };
}
