import type { Selection } from "~/components/library/selection";
import { bulkEdit } from "~/lib/api";
import { ApiError } from "~/lib/http";
import { t } from "~/lib/i18n";
import type { BulkEdit, ItemFilter } from "~/lib/types";

/** What a bulk run left behind. `over-cap` is a refusal, and it carries both numbers. */
export type BulkOutcome =
  | { kind: "done"; changed: number }
  | { kind: "over-cap"; matched: number; limit: number }
  | { kind: "error"; message: string };

/**
 * Which items the run acts on: `ids` XOR `filter`, exactly one.
 *
 * "Both" has no meaning and "neither" would quietly mean the whole library, so the server
 * refuses either; this is the one place the choice is made, so no call site can get it
 * wrong by adding a field.
 */
export function bulkTarget(
  selection: Selection,
  filter: ItemFilter,
): Pick<BulkEdit, "ids" | "filter"> {
  if (selection.matching()) return { filter: asPairs(filter) };
  return { ids: selection.picked() };
}

export async function runBulk(edit: BulkEdit): Promise<BulkOutcome> {
  try {
    const { changed } = await bulkEdit(edit);
    return { kind: "done", changed };
  } catch (error) {
    if (error instanceof ApiError) {
      const cap = error.overCap;
      // R-FE-11: the refusal is only actionable with both numbers. Never trim to the cap
      // and proceed — that edits a set the user did not choose and cannot see.
      if (cap) return { kind: "over-cap", ...cap };
      if (error.isPaused) {
        return { kind: "error", message: t("library.bulk.errors.paused") };
      }
      // The server's own words, which arrive in English whatever the dashboard is set to.
      // Passed through rather than guessed at — see `ItemDetail`'s `explain`.
      return { kind: "error", message: error.message };
    }
    return { kind: "error", message: t("library.bulk.errors.failed") };
  }
}

/**
 * The filter, in the shape the endpoint parses.
 *
 * `POST /api/bulk/edit` takes the filter as **repeatable pairs** — the same
 * `?tag=a&tag=b` facets `GET /api/items` accepts — because that is what lets one kind carry
 * a set without a delimiter a tag name could contain.
 */
function asPairs(filter: ItemFilter): [string, string][] {
  const pairs: [string, string][] = [];
  for (const value of filter.type ?? []) pairs.push(["type", value]);
  for (const value of filter.family ?? []) pairs.push(["family", value]);
  for (const value of filter.tag ?? []) pairs.push(["tag", value]);
  for (const value of filter.status ?? []) pairs.push(["status", value]);
  if (filter.q) pairs.push(["q", filter.q]);
  if (filter.needs_review) pairs.push(["needs_review", "1"]);
  return pairs;
}
