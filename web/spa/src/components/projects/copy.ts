/**
 * The Projects page's copy, in one place.
 *
 * PRD §5 makes the wording a requirement rather than a detail — honest status, empty states
 * that teach the next action, every disabled control saying why. Keeping these sentences
 * together is what stops one of them drifting into "Opened" or "Project removed".
 */

export const PAUSED_REASON = "Curio is paused. Resume from the tray icon.";

/**
 * FR-19. A missing folder is greyed, never removed: the record carries the prompt link, and
 * nothing left on disk can rebuild that.
 */
export const MISSING_EXPLANATION =
  "This folder is no longer on disk. Curio keeps the record because it holds the prompt link — that connection lives here and nowhere else, so deleting the record would lose it for good. Put the folder back at this path and it comes back on the next visit.";

export const NO_FRONT_DOOR =
  "There is no index.html here, or in a v1/v2/… subfolder, so there is no page to launch. Open the folder to see what the tool actually wrote.";

export const EMPTY_TITLE = "No projects yet.";

export const EMPTY_BODY =
  "Curio watches your projects folder and adds any new top-level folder here within about five seconds. Point it at the right folder in Settings, or register one by hand below.";
