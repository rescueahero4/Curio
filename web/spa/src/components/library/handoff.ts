import type { Item } from "~/lib/types";

/**
 * The item's directory on disk, absolute.
 *
 * This is what a user pastes to an agent, so it has to be the real path rather than a URL:
 * the agent reads `screenshot.png` and `item.md` from the filesystem, and it has no session
 * cookie with which to ask the server for them.
 *
 * The separator follows the data root the server reported rather than a platform guess —
 * the SPA runs in a browser and cannot tell what the host is, but the path it was given
 * already knows.
 */
export function itemDirectory(dataRoot: string, id: string): string {
  const separator = dataRoot.includes("\\") ? "\\" : "/";
  const trimmed = dataRoot.replace(/[\\/]+$/, "");
  return [trimmed, "items", id].join(separator);
}

/**
 * The brief, as text.
 *
 * Assembled here rather than fetched: `item.md` beside the screenshot is a write-only
 * projection of the same row (Inventory §10.19), and reading it back to copy it would put a
 * second source of truth between the database and the clipboard.
 *
 * **The field labels stay English in both languages, deliberately.** This text is not read
 * on a screen — it is pasted to an agent, alongside the `item.md` the server writes, and that
 * file's headings are the server's and are English whatever the dashboard is set to. Two
 * projections of one row disagreeing about what the fields are called is the failure worth
 * avoiding here; a Japanese heading on the clipboard copy would buy nothing.
 */
export function briefText(item: Item, directory: string | null): string {
  const lines = [item.name, "", item.short_description, ""];

  if (item.families.length) {
    lines.push(`Families: ${item.families.map((family) => family.name).join(", ")}`);
  }
  if (item.design_types.length) lines.push(`Design types: ${item.design_types.join(", ")}`);
  if (item.tags.length) lines.push(`Tags: ${item.tags.join(", ")}`);
  if (item.source_url) lines.push(`Source: ${item.source_url}`);
  if (directory) lines.push(`Files: ${directory}`);

  return lines.join("\n").trim();
}
