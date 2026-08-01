/**
 * Every endpoint the dashboard calls, named once.
 *
 * Thin by design: the transport, the error contract and the paused signal live in
 * `http.ts`, and the wire shapes live in `types.ts`. What is left here is the URL map — so
 * that "which routes does the SPA depend on" is answerable by reading one file, and a
 * server-side rename breaks in one place instead of thirty.
 */

import { del, get, patch, post, postForm, put } from "~/lib/http";
import type {
  BulkEdit,
  CreatedItem,
  GrayZoneDecision,
  Health,
  Item,
  ItemFilter,
  ItemPatch,
  ItemQuery,
  Job,
  LaunchOutcome,
  OpenedProject,
  Page,
  Project,
  Prompt,
  PromptTemplate,
  SerializedPrompt,
  Settings,
  SettingsPatch,
  Vocabulary,
  VocabularyKind,
} from "~/lib/types";

/* -- health ------------------------------------------------------------------------- */

export function getHealth(): Promise<Health> {
  return get<Health>("/health");
}

/* -- items -------------------------------------------------------------------------- */

export function listItems(query: ItemQuery = {}): Promise<Page<Item>> {
  return get<Page<Item>>(`/api/items${itemQuery(query)}`);
}

/**
 * How many items a filter matches, without fetching them.
 *
 * `listItems` deliberately answers "is there another page" rather than a total, which is
 * what a scrolling grid needs and what a badge cannot use. `limit` and `cursor` are not
 * sent: paging has no meaning for a count.
 */
export function countItems(filter: ItemFilter = {}): Promise<{ count: number }> {
  return get<{ count: number }>(`/api/items/count${itemQuery(filter)}`);
}

export function getItem(id: string): Promise<Item> {
  return get<Item>(`/api/items/${encodeURIComponent(id)}`);
}

/** One PATCH stamps `last_edited_by = user`; bulk edits deliberately do not (§10.12). */
export function updateItem(id: string, changes: ItemPatch): Promise<Item> {
  return patch<Item>(`/api/items/${encodeURIComponent(id)}`, changes);
}

export function deleteItem(id: string): Promise<void> {
  return del<void>(`/api/items/${encodeURIComponent(id)}`);
}

/** Ingestion. The screenshot is mandatory — there is no item without one (FR-2). */
export function createItem(input: {
  screenshot: File;
  source_url?: string;
  title?: string;
}): Promise<CreatedItem> {
  const form = new FormData();
  form.append("screenshot", input.screenshot, input.screenshot.name);
  if (input.source_url) form.append("source_url", input.source_url);
  if (input.title) form.append("title", input.title);
  return postForm<CreatedItem>("/api/items", form);
}

export function reassessItem(id: string): Promise<{ job_id: string }> {
  return post<{ job_id: string }>(`/api/items/${encodeURIComponent(id)}/reassess`);
}

/** The one-decision gray-zone resolution: keep, move, or accept Curio's proposal (FR-7). */
export function resolveGrayZone(id: string, decision: GrayZoneDecision): Promise<Item> {
  return post<Item>(`/api/items/${encodeURIComponent(id)}/resolve-grayzone`, decision);
}

/* -- bulk --------------------------------------------------------------------------- */

export function bulkEdit(edit: BulkEdit): Promise<{ changed: number }> {
  return post<{ changed: number }>("/api/bulk/edit", edit);
}

/* -- vocabulary --------------------------------------------------------------------- */

export function getVocabulary(): Promise<Vocabulary> {
  return get<Vocabulary>("/api/vocabulary");
}

export function createTerm(
  kind: VocabularyKind,
  body: { name: string; description?: string },
): Promise<unknown> {
  return post(`/api/vocabulary/${kind}`, body);
}

export function updateTerm(
  kind: VocabularyKind,
  id: string,
  body: { name?: string; description?: string },
): Promise<unknown> {
  return patch(`/api/vocabulary/${kind}/${encodeURIComponent(id)}`, body);
}

export function deleteTerm(kind: VocabularyKind, id: string): Promise<void> {
  return del<void>(`/api/vocabulary/${kind}/${encodeURIComponent(id)}`);
}

/** Merge keeps the links: MAX(score), MIN(gray_zone) on families (§10.14). */
export function mergeTerm(kind: VocabularyKind, id: string, into: string): Promise<unknown> {
  return post(`/api/vocabulary/${kind}/${encodeURIComponent(id)}/merge`, { into });
}

/* -- prompts ------------------------------------------------------------------------ */

export function listPrompts(): Promise<Prompt[]> {
  return get<Prompt[]>("/api/prompts");
}

export function getPromptTemplate(): Promise<PromptTemplate> {
  return get<PromptTemplate>("/api/prompts/template");
}

export function createPrompt(body: { title?: string; doc_json?: unknown } = {}): Promise<Prompt> {
  return post<Prompt>("/api/prompts", body);
}

export function getPrompt(id: string): Promise<Prompt> {
  return get<Prompt>(`/api/prompts/${encodeURIComponent(id)}`);
}

/**
 * Autosave. The document, and nothing else.
 *
 * A prompt's title is not sent because it is not the client's to set: the server names a
 * prompt after its own first line on every save (`curio_core::prompt::title_from`), and the
 * reply carries the name it chose. Sending one would be a write the next keystroke undid.
 */
export function updatePrompt(id: string, changes: { doc_json?: unknown }): Promise<Prompt> {
  return patch<Prompt>(`/api/prompts/${encodeURIComponent(id)}`, changes);
}

export function deletePrompt(id: string): Promise<void> {
  return del<void>(`/api/prompts/${encodeURIComponent(id)}`);
}

/** Serialization is the server's, authoritatively. The SPA never expands a chip (R-FE-18). */
export function serializePrompt(id: string): Promise<SerializedPrompt> {
  return post<SerializedPrompt>(`/api/prompts/${encodeURIComponent(id)}/serialize`);
}

export function markPromptSent(id: string): Promise<Prompt> {
  return post<Prompt>(`/api/prompts/${encodeURIComponent(id)}/sent`);
}

export function clearPromptSent(id: string): Promise<Prompt> {
  return del<Prompt>(`/api/prompts/${encodeURIComponent(id)}/sent`);
}

/* -- projects ----------------------------------------------------------------------- */

/** Repairs as it reads: this call reconciles missing and relocated folders (§10.28). */
export function listProjects(): Promise<Project[]> {
  return get<Project[]>("/api/projects");
}

export function registerProject(body: {
  path: string;
  name?: string;
  prompt_id?: string;
}): Promise<Project> {
  return post<Project>("/api/projects", body);
}

export function updateProject(
  id: string,
  changes: { prompt_id?: string | null },
): Promise<Project> {
  return patch<Project>(`/api/projects/${encodeURIComponent(id)}`, changes);
}

export function openProject(id: string): Promise<OpenedProject> {
  return post<OpenedProject>(`/api/projects/${encodeURIComponent(id)}/open`);
}

/* -- settings ----------------------------------------------------------------------- */

export function getSettings(): Promise<Settings> {
  return get<Settings>("/api/settings");
}

export function saveSettings(changes: SettingsPatch): Promise<Settings> {
  // No `restart_required` in the reply: the port is ephemeral or pinned at boot, so no
  // settings change can require a restart (ARCH-08 break #9).
  return put<Settings>("/api/settings", changes);
}

export function clearApiKey(): Promise<void> {
  return del<void>("/api/settings/api-key");
}

export function verifyApiKey(body: { api_key?: string } = {}): Promise<{ ok: boolean }> {
  return post<{ ok: boolean }>("/api/settings/verify-key", body);
}

/* -- system ------------------------------------------------------------------------- */

export function revealPath(path: string): Promise<LaunchOutcome> {
  return post<LaunchOutcome>("/api/system/reveal", { path });
}

export function openSkillFile(): Promise<LaunchOutcome> {
  return post("/api/system/open-skill-file");
}

/**
 * Best-effort handoff. Answers 200 with the outcome in the body even when the launch
 * failed, because "asked Claude Code to open" is the only claim we can honestly make
 * (§10.22) — the caller reads the body rather than the status.
 */
export function sendToClaude(text: string): Promise<LaunchOutcome> {
  return post<LaunchOutcome>("/api/system/send-to-claude", { text });
}

/* -- jobs --------------------------------------------------------------------------- */

export function listJobs(): Promise<Job[]> {
  return get<Job[]>("/api/jobs");
}

export function cancelJob(id: string): Promise<Job> {
  return post<Job>(`/api/jobs/${encodeURIComponent(id)}/cancel`);
}

/* -- files -------------------------------------------------------------------------- */

/** A screenshot or thumbnail URL. Jailed and cookie-authed like everything else. */
export function fileUrl(itemId: string, file: string): string {
  return `/files/items/${encodeURIComponent(itemId)}/${encodeURIComponent(file)}`;
}

/**
 * The image to show for an item.
 *
 * Falls back to the screenshot when no thumbnail was written — `sharp` is optional in the
 * pipeline, so a missing thumbnail is an ordinary outcome rather than a fault (§10.20).
 */
export function itemImageUrl(item: {
  id: string;
  screenshot_path: string;
  thumbnail_path: string | null;
}): string {
  return fileUrl(item.id, basename(item.thumbnail_path ?? item.screenshot_path));
}

/**
 * The image for the **item page**, where the capture is the subject.
 *
 * Deliberately not [`itemImageUrl`]. That one answers with the grid's thumbnail, which is
 * 640 px wide and cropped to the first fold — correct on a card, and on this page it turned
 * a 20,000-pixel full-page capture into a blurry crop of its masthead.
 *
 * `detail.jpg` is not recorded on the row: it is a derived file the server discovers by
 * name, and the file route answers with the full screenshot when it is absent. So this URL
 * is correct for an item captured today and for one captured before the derivative existed.
 */
export function itemDetailImageUrl(item: { id: string }): string {
  return fileUrl(item.id, "detail.jpg");
}

function basename(path: string): string {
  const parts = path.split(/[\\/]/);
  return parts[parts.length - 1] ?? path;
}

/**
 * Build the item query string.
 *
 * `type`, `family`, `tag` and `status` repeat — one parameter per selected facet — because
 * the server ANDs across kinds and ORs within one (FR-10). Comma-joining them would make
 * a tag literally named "a,b" indistinguishable from two tags.
 */
function itemQuery(query: ItemQuery): string {
  const params = new URLSearchParams();

  for (const value of query.type ?? []) params.append("type", value);
  for (const value of query.family ?? []) params.append("family", value);
  for (const value of query.tag ?? []) params.append("tag", value);
  for (const value of query.status ?? []) params.append("status", value);

  if (query.q) params.set("q", query.q);
  if (query.needs_review) params.set("needs_review", "1");
  if (query.limit !== undefined) params.set("limit", String(query.limit));
  if (query.cursor) params.set("cursor", query.cursor);

  const encoded = params.toString();
  return encoded ? `?${encoded}` : "";
}
