import { ApiError } from "~/lib/http";
import { t } from "~/lib/i18n";

/**
 * What a refused vocabulary write says to the reader.
 *
 * One function rather than the copy that used to sit at the bottom of both `VocabRow` and
 * `VocabBulkBar`. This is not tidying: the row panel and the bulk bar call the same three
 * routes and can be refused for the same three reasons, and two copies of this decision
 * drift into a page that explains a 409 one way when it happens to one word and another way
 * when it happens to thirty.
 *
 * **409 is answered here rather than passed through.** It is the likeliest refusal on this
 * page — renaming onto a name that exists, or adding one — and the server's own sentence for
 * it is English, so forwarding `error.message` put an English sentence in a Japanese banner
 * on the one failure a Japanese reader is most likely to meet. What that sentence adds over
 * the localized one is the name that collided, and on a rename that name is in the field the
 * reader just typed it into, six inches above the banner.
 *
 * **Every other status still forwards the server's words, and those are English.** A 500 or
 * a validation refusal reaches a Japanese reader in English. That is a real gap and it is not
 * closable from here: the fix is either an error code the client can translate or an
 * `Accept-Language` the server honours, and neither exists yet.
 *
 * Both overrides exist because the same refusal has a different next step depending on who
 * asked. A 409 on a rename is one term colliding with another and the answer is merge, which
 * is the control under the message. A 409 on a *create* is the same collision with only one
 * term in it — nothing was made, so there is nothing to merge from, and the Add popover has
 * no merge control in it to point at. Telling that reader to merge sends them looking for a
 * button that is not on the screen.
 */
export function refusal(error: unknown, words: { fallback?: string; taken?: string } = {}): string {
  if (!(error instanceof ApiError)) return words.fallback ?? t("vocabulary.errors.generic");
  if (error.isPaused) return t("vocabulary.errors.paused");
  if (error.status === 409) return words.taken ?? t("vocabulary.errors.taken");
  return error.message;
}
