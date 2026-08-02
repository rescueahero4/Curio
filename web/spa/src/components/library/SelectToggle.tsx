/**
 * The selection checkbox, shared by the card and the row.
 *
 * A real `<input type="checkbox">` rather than a styled div, and that is the whole point of
 * pulling it out: selection is the second thing a user does with a library, and both views
 * have to reach it by keyboard. `opacity` is what hides it at rest — `display: none` or a
 * conditional render would take it out of the tab order, so a keyboard user could never
 * reveal it, and `focus-within` is what brings it back when they Tab to it.
 *
 * Shift-click is passed up rather than handled here: what a range means is the caller's
 * question, and it differs between a grid and a list.
 */
export function SelectToggle(props: {
  name: string;
  selected: boolean;
  /** Where the checkbox sits. The card overlays it; the row gives it a column. */
  class?: string;
  onToggle: (extend: boolean) => void;
}) {
  return (
    <label
      class={`flex items-center opacity-0 transition-opacity focus-within:opacity-100 group-hover:opacity-100 ${props.class ?? ""}`}
      classList={{ "opacity-100": props.selected }}
      style={{ "transition-duration": "var(--duration-hover)" }}
    >
      <span class="sr-only">Select {props.name || "this item"}</span>
      <input
        type="checkbox"
        checked={props.selected}
        onClick={(event) => props.onToggle(event.shiftKey)}
      />
    </label>
  );
}
