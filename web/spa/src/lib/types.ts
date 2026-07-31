/**
 * The server's JSON, in TypeScript.
 *
 * snake_case throughout, and deliberately so. These are the wire shapes ARCH-01 owns; a
 * camelCase rename on the way in would put a translation layer between the API contract
 * and every error message, log line and network-tab row that talks about it — and the
 * layer would be the first thing to drift.
 *
 * Nothing here is validated at runtime. The server is same-origin, in-process, and shipped
 * in the same binary; a schema validator on this boundary would guard against a version
 * skew that cannot occur.
 */

export type ItemStatus = "processing" | "ready" | "needs_review" | "assessment_failed";

/** Who last touched a field. Load-bearing: re-assess must not overwrite a user's name. */
export type LastEditedBy = "ai" | "user";

export type CreatedBy = "ai" | "user";

/** One item→family link, carrying the score that decided it. */
export interface ItemFamily {
  id: string;
  name: string;
  score: number;
  gray_zone: boolean;
  ai_proposed: boolean;
}

export interface Item {
  id: string;
  name: string;
  short_description: string;
  source_url: string | null;
  image_recipe: string | null;
  screenshot_path: string;
  thumbnail_path: string | null;
  status: ItemStatus;
  last_edited_by: LastEditedBy;
  error: string | null;
  created_at: string;
  updated_at: string;
  design_types: string[];
  tags: string[];
  families: ItemFamily[];
}

export interface Family {
  id: string;
  name: string;
  description: string;
  created_by: CreatedBy;
  created_at: string;
  updated_at: string;
  item_count: number;
}

/** A design type or a tag. Both are just a name with a usage count. */
export interface Term {
  id: string;
  name: string;
  created_by: CreatedBy;
  item_count: number;
}

export interface Vocabulary {
  families: Family[];
  design_types: Term[];
  tags: Term[];
}

/** Which vocabulary collection an endpoint addresses. */
export type VocabularyKind = "families" | "types" | "tags";

export interface Prompt {
  id: string;
  title: string;
  doc_json: unknown;
  serialized_text: string;
  created_at: string;
  updated_at: string;
  sent_at: string | null;
}

export type ProjectOrigin = "watcher" | "mcp" | "manual";

export interface Project {
  id: string;
  name: string;
  path: string;
  origin: ProjectOrigin;
  prompt_id: string | null;
  status: "present" | "missing";
  fingerprint: string | null;
  detected_at: string;
  last_opened_at: string | null;
}

export type JobStatus = "queued" | "running" | "done" | "failed" | "cancelled";

export interface Job {
  id: string;
  kind: string;
  payload: unknown;
  status: JobStatus;
  attempts: number;
  error: string | null;
  result: unknown | null;
  not_before: string | null;
  created_at: string;
  updated_at: string;
}

/**
 * `job.updated` arrives as a full row on enqueue/start/finish and as a partial
 * `{id, result}` on every progress tick (Inventory §10.30). Consumers merge by id.
 */
export type JobUpdate = { id: string } & Partial<Omit<Job, "id">>;

export interface Page<T> {
  items: T[];
  next_cursor: string | null;
}

/** The six fields `/health` answers with, and no more (R-SEC-11, Inventory §10.29). */
export interface Health {
  status: string;
  version: string;
  port: number;
  items: number;
  queue: number;
  api_key_configured: boolean;
}

/** The facets `GET /api/items` accepts. Facets AND across kinds, OR within one (FR-10). */
export interface ItemFilter {
  type?: string[];
  family?: string[];
  tag?: string[];
  status?: ItemStatus[];
  q?: string;
  needs_review?: boolean;
}

export interface ItemQuery extends ItemFilter {
  limit?: number;
  cursor?: string | null;
}

export type ItemPatch = Partial<
  Pick<Item, "name" | "short_description" | "source_url" | "image_recipe"> & {
    tags: string[];
    design_types: string[];
    family_ids: string[];
  }
>;

export interface CreatedItem {
  item_id: string;
  job_id: string;
  item: Item;
}

export type GrayZoneAction = "accept" | "reassign" | "accept_proposal";

export interface GrayZoneDecision {
  action: GrayZoneAction;
  family_id?: string;
}

/**
 * `ids` XOR `filter` — an explicit selection or the current filter, never both
 * (Inventory §6 selection model).
 */
export interface BulkEdit {
  ids?: string[];
  /**
   * The same repeatable query parameters `GET /api/items` takes, as pairs.
   *
   * Pairs rather than an object because a facet is repeatable — `[["tag","a"],["tag","b"]]`
   * has no object equivalent that does not invent a delimiter a tag name could contain.
   * The server declares it as `Vec<(String, String)>`; an object here would 400.
   */
  filter?: [string, string][];
  add_tags?: string[];
  remove_tags?: string[];
  add_types?: string[];
  remove_types?: string[];
  add_families?: string[];
  remove_families?: string[];
  delete?: boolean;
}

/** What the server refused with when a selection exceeds the cap (R-FE-11). */
export interface OverCap {
  matched: number;
  limit: number;
}

export interface SerializedPrompt {
  text: string;
  path: string;
}

export interface OpenedProject {
  url: string;
  entry: string;
  path: string;
}

export interface Thresholds {
  lower: number;
  upper: number;
}

export interface Models {
  vision: string;
  utility: string;
}

export type SendToClaudeTarget = "claude-code" | "claude-desktop" | "clipboard";

/**
 * Settings as the server publishes them.
 *
 * Mirrors `PublicSettings` in `crates/curio-server/src/routes/api/settings.rs` field for
 * field. That projection is built from an allowlist rather than by stripping a spread
 * (R-SEC-10, Inventory §10.5), so anything absent there is absent by decision — no secret
 * can reach this type by being forgotten.
 */
export interface Settings {
  data_root: string;
  projects_root: string;
  /** Absent means the port is ephemeral, which is the default (D10). */
  port: number | null;
  thresholds: Thresholds;
  models: Models;
  mcp_enabled: boolean;
  send_to_claude_target: SendToClaudeTarget;
  launch_at_login: boolean;
  /** The OS is the authority; `reason` says why when it cannot be honoured. */
  launch_at_login_support: { supported: boolean; reason: string | null };
  api_key_set: boolean;
  api_key_masked: string | null;
  /** A previous install's `.secrets.json` is present and its key could not be carried over (D31). */
  api_key_legacy_present: boolean;
  skill_file_path: string;
  version: string;
  mcp_http_url: string;
  mcp_stdio_command: string;
  mcp_stdio_args: string[];
}

/**
 * `api_key` is write-only: it goes up, and only `api_key_masked` comes back.
 *
 * `data_root` and `port` are absent on purpose. Moving a library is not a form field, and
 * the port is read at boot from config or the environment — a settings save cannot change
 * it, so offering the control would be a dead one (ARCH-08 break #9).
 */
export type SettingsPatch = Partial<
  Pick<
    Settings,
    | "projects_root"
    | "thresholds"
    | "models"
    | "mcp_enabled"
    | "send_to_claude_target"
    | "launch_at_login"
  >
> & { api_key?: string };

/** The gold-standard prompt scaffold, owned by the server so no client re-derives it. */
export interface PromptTemplate {
  doc_json: unknown;
  /** The eight sections, in order. `ghost` is placeholder text — never content. */
  sections: { id: string; heading: string; ghost: string }[];
}

/**
 * What the system routes report.
 *
 * `asked`, not `succeeded`: launching an external tool is a request to the OS, and nothing
 * on this side can observe whether it opened. `message` is already phrased that way — show
 * it verbatim rather than composing your own (Inventory §10.22).
 */
export interface LaunchOutcome {
  asked: boolean;
  message: string;
}
