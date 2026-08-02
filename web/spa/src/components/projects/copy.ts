/**
 * The Projects page's copy, in one place.
 *
 * PRD §5 makes the wording a requirement rather than a detail — honest status, empty states
 * that teach the next action, every disabled control saying why. Keeping these sentences
 * together is what stops one of them drifting into "Opened" or "Project removed".
 */

export const PAUSED_REASON = "Curio is paused. Resume from the tray icon.";

/**
 * FR-19. Curio still never removes a missing folder's record on its own — the record carries
 * the prompt link, and nothing left on disk can rebuild it. What changed is who gets to
 * decide: the card now says the short version and offers the two ways out, rather than
 * spending a paragraph arguing for a record the user may have meant to be rid of.
 */
export const MISSING_TITLE = "Can't find project folder";

export const NO_FRONT_DOOR =
  "There is no index.html here, or in a v1/v2/… subfolder, so there is no page to launch. Open the folder to see what the tool actually wrote.";

/** The one sentence that has to land: entries arrive here on their own (FR-17). */
export const SUBTITLE = "Folders your AI tools wrote to the projects root. Detected automatically.";

export const EMPTY_TITLE = "No projects yet.";

/**
 * The empty state teaches the prompt-first route, because that is the one the app is
 * built around: compose a brief, send it to Claude, and the folder it writes is adopted
 * within about five seconds. Registering by hand is the exception and is offered below the
 * list, not here — an empty page is exactly where a user is most likely to mistake the
 * exception for the intended path.
 */
export const EMPTY_BODY =
  "Start a prompt, paste it into your AI tool, and point it at your projects root. The folder it writes shows up here within about five seconds — there is no import step. Check which folder Curio is watching in Settings.";
