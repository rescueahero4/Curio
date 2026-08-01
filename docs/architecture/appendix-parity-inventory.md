---
id: ARCH-A1
title: "Appendix — Parity Inventory (mined from the Bun/React implementation)"
status: reference
version: 1.0.0
date: 2026-07-30
project: curio
governs: [parity-evidence]
note: "Evidence appendix cited by ARCH-08 as Inventory §n. Read-only record of the old app's behavior; file:line cites refer to the Curiol repo."
---

# Curio parity inventory — mined from the running Bun/React implementation (Curiol repo)

File:line citations refer to the Curiol (Bun) codebase; they are evidence pointers, not targets for the rewrite.

## 1. HTTP API surface (as implemented)

CORS/origin policy: one `allowedOrigin` fn shared by `/api/*` (allowHeaders `content-type, x-curio-token`), `/health`, `/mcp` (allowHeaders `content-type, mcp-protocol-version, mcp-session-id, last-event-id`). Allowed: empty Origin (non-CORS), pinned extension origin `chrome-extension://<id-from-manifest-key>`, and loopback origin whose authority equals the request's own `Host` (never the configured port — DNS-rebinding guard via loopback regex). Route order: system → /api mounts → pair → project static → **/mcp before UI catch-all** → SPA fallback (404s /api|/files|/p/ and build-asset paths).

Routes (auth = none unless noted):
- `GET /health` — unauthenticated, cross-origin-readable BY DESIGN (extension status dot + port walk): `{status:"ok", version, port, items, queue, api_key_configured}`.
- `POST /api/ingest` — multipart; auth `x-curio-token` = pairingToken; fields screenshot (required), source_url|url, title, captured_at, viewport_width/height; 201 `{item_id, status:"processing"}`.
- `GET /api/events` — SSE; `hello` on connect, `ping` every 20 s.
- `GET /api/jobs`, `GET /api/jobs/:id`, `POST /api/jobs/:id/cancel` (also cancels Anthropic batch if result.batch_id).
- `GET /files/items/:id/:file` — jailed; thumb-miss falls back to screenshot.png; `cache-control: private, max-age=60`.
- `POST /api/system/open-skill-file`, `POST /api/system/send-to-claude` (always 200, body carries outcome), `POST /api/system/quit` (auth `x-curio-quit-token` from lock file; 503/401/200), `POST /api/system/reveal {path}`.
- Items: `GET /api/items` (repeatable `type,family,tag,status` facets by id, `q`, `needs_review=1`, limit 1-200 default 60, keyset cursor `created_at|id`), `GET /api/items/count` (same facets → `{count}`; `limit`/`cursor` accepted and ignored), `GET/PATCH/DELETE /api/items/:id`, `POST /api/items` (Tier-0 multipart → `{item_id, job_id, item}`), `POST /api/items/:id/reassess`, `POST /api/items/:id/resolve-grayzone` (`accept|reassign(family_id)|accept_proposal`). PATCH gray-zone rule: patching family_ids on a needs_review item with a gray-zone link auto-promotes to ready.
- Bulk: `POST /api/bulk/retag` (ids XOR filter, cap 500, no-API-key → 409, over-cap → 409 with `matched`+`limit`; → `{job_id, items, via:"batch"|"serial"}`), `POST /api/bulk/edit` (sync; add/remove tags/types/families + delete), `POST /api/bulk/dedupe` (single-flight, `already_running`), `GET /api/bulk/dedupe/latest`.
- Vocabulary: GET/POST + PATCH/DELETE /:id + POST /:id/merge for families, types, AND tags. Delete/rename/merge rebuild FTS + sidecars in-transaction.
- Prompts: GET list, `GET /api/prompts/template`, GET/POST/PATCH/DELETE /:id, `POST /:id/serialize` → `{text, path}`, `POST /:id/sent` + `DELETE /:id/sent` (claim staking).
- Projects: `GET /api/projects` (**runs reconcile + relocateMissing side effects on every call**), `POST /api/projects` (manual register), `PATCH /:id` (prompt_id), `POST /:id/open` → `{url, entry, path}`; `GET /p/:id` redirect + `GET /p/:id/*` static, jailed + `projectServeRefusal` (dotfiles + reserved data-root files: lock, library.db prefix incl -wal/-shm, config.json, .secrets.json, skills/, items/, prompts/); dir without index.html → filtered HTML listing.
- Settings: GET/PUT, `DELETE /api/settings/api-key`, `POST /api/settings/verify-key`. PUT side effects: setApiKey before persist; rebindWatcher on projectsRoot change; applyAutostart only when launchAtLogin in patch; reply includes `restartRequired` (only port change). Response `PublicSettings` = settings minus pairingToken (double-stripped) + `apiKeySet, apiKeyMasked, skillFilePath, version, mcpStdioCommand, mcpStdioArgs, launchAtLogin (OS truth), launchAtLoginSupport{supported,reason}`.
- `POST /api/pair/authorize` → `{token}` — POST-only, GET 404.
- Server: 127.0.0.1 only, 64 MB max body (matches 20k-px stitches), idleTimeout 120 s.

## 2. MCP surface — 7 tools, one factory, both transports
1. `library_search {query?, design_types?, families?, tags?, limit<=50 def 20}` → `{total, items:[{id,name,short_description,source_url,status,directory,design_types,tags,families:[{name,score}]}]}`; names→ids case-insensitive, unknown silently dropped.
2. `library_get_item {item_id}` → full item + absolute screenshot + sidecar paths.
3. `library_list_vocabulary {}` → families `{id,name,description,item_count}`, types/tags name arrays.
4. `library_create_item {screenshot_path, source_url?, name?}` → copies file in, queues assessment.
5. `library_update_item {item_id, name?, short_description?, image_recipe?, tags?, design_types?}` — tags/types replace whole sets, stamps last_edited_by='ai', writes sidecar, emits item.updated.
6. `prompt_get {prompt_id def "latest"}` → serialized text + structuredContent {id,title}.
7. `project_register {path, name?, prompt_id?}` — origin "mcp"; does NOT set a fingerprint (not rename-followable).
- HTTP: `mcpEnabled` gate read PER REQUEST → 503 JSON-RPC `{code:-32000, data:{reason:"mcp_disabled", …, stdioTransport:"unaffected"}}` when off; stateless (new transport+server per request); GET/DELETE → 405 JSON-RPC.
- stdio: `curio --mcp-stdio` — no HTTP/tray/watcher/worker; stdout reserved for frames, logs → stderr; NOT gated by mcpEnabled. Settings UI shows both config snippets (Claude Code HTTP + Claude Desktop stdio).

## 3. Events
In-process pub/sub → SSE. Names: `item.created` (full Item), `item.updated`, `item.deleted {id}`, `project.detected`, `project.updated`, `job.updated` (full row OR partial `{id, result}` during bulk progress), `vocabulary.updated {}`; stream-level `hello`, `ping` 20 s. Client: single shared EventSource, per-name handlers, reconnect 2 s.

## 4. DB (SQLite, WAL, foreign_keys ON, busy_timeout 5000, synchronous NORMAL)
Tables: items (ULID pk, name, short_description, source_url, image_recipe, screenshot_path rel, thumbnail_path, status processing|ready|needs_review|assessment_failed, last_edited_by ai|user, error, timestamps), aesthetic_families (name UNIQUE, description, created_by), design_types (name UNIQUE), tags (name UNIQUE NOCASE), item_families (pk item+family, score REAL, gray_zone INT, ai_proposed INT), item_types, item_tags, prompts (doc_json TipTap, serialized_text, sent_at), projects (path UNIQUE, origin watcher|mcp|manual, prompt_id FK no ON DELETE, status present|missing, fingerprint, detected_at, last_opened_at), jobs (kind, payload, status 5 states, attempts, error, result, not_before).
- FTS5 `items_fts(item_id UNINDEXED, name, short_description, tags_concat, unicode61)` — OPTIONAL: degrade to LIKE when FTS5 missing; syncFts per write txn; query builder quotes terms, prefix-matches last, ANDs.
- Migrations: user_version stepping, per-migration txn, SchemaTooNewError when db newer than build. v2 jobs.result/prompts.sent_at/projects.fingerprint; v3 jobs.not_before; v4 null non-`mark:` fingerprints.
- **Monotonic ULIDs** (same-ms ordering) — load-bearing for FIFO job claim and keyset pagination.
- `nowIso()` second precision EXCEPT markPromptSent (ms — ordering key).

## 5. Settings / secrets / env
- Data root: `CURIO_DATA_ROOT` (~ expanded) → deprecated `CURIOL_DATA_ROOT` (warn) → one-time `~/Curiol`→`~/Curio` rename (only default root, target absent, legacy has library.db|config.json; never merge; failure → fall back; delete stale curiol.lock). Materialize root, skills/, items/, prompts/, projects/; seed skills/visual-assessment.md once, never overwrite.
- config.json: dataRoot, projectsRoot, port (1024-65535 def 4321), thresholds{lower,upper} def 0.4/0.5 (PUT rejects lower>upper), models{vision:"claude-sonnet-5", utility:"claude-haiku-4-5"}, mcpEnabled def false, sendToClaudeTarget claude-code|claude-desktop|clipboard, launchAtLogin (stored but OS is authority), pairingToken (24 B base64url, minted first run). Patch schema omits dataRoot+pairingToken, adds write-only apiKey. `CURIO_PORT`/`CURIOL_PORT` override. Rewritten every boot/save. `CURIO_NO_OPEN=1`/`--no-open`.
- Secrets: env `ANTHROPIC_API_KEY` → `.secrets.json` (0600) backends: dpapi (Win, secret via env not argv), keychain (macOS `security`, service curio-anthropic-api-key), file (AES-256-GCM, scrypt key `curio:<hostname>:<username>`). Masked `sk-ant-…xxxx`.

## 6. UI routes & behaviors
Routes: `/` Library, `/items/:id`, `/projects`, `/prompts`, `/prompts/:id` (lazy-split), `/vocabulary`, `/settings`, `/pair` (above catch-all, eager), `*` NotFound.
- Library: 200 ms debounced search; view mode (comfortable/dense/list) in localStorage `curio.view` (migrates once from `curio.dense`); filter row links to Vocabulary and badges the review queue from `GET /api/items/count?needs_review=1`, debounced 400 ms on item events; infinite scroll (IntersectionObserver 400 px rootMargin, keyset cursor); SSE: item.created prepends ONLY when no filter/search active; item.updated replaces in place; Escape clears selection; filter change voids "matching" selection, keeps picked ids.
- Selection: two modes `picked(ids)` | `matching` (= the filter); shift-click range from anchor; cap-awareness ≤500, over-cap = named refusal (`overCapNote`), never trims; resolveRun → {ids}|{filter,count}|none.
- BulkBar: picks up in-flight bulk_retag on mount; job.updated partial merges; running = progress + Cancel; finished = summary/"Cancelled after N"/error + Dismiss; 5 actions cheapest-first (3 edit panels, AI Re-tag popover [augment|replace + optional instruction], Delete with count-confirm); `notice` keeps bar mounted after delete clears selection; vocab edits keep selection, delete clears.
- ConsistencyPass (Vocabulary page): latest dedupe result survives reload; merges applied client-side via merge endpoints; per-group Merge / Keep both.
- ItemDetail: autosave PATCH 600 ms coalesced; pending-patch guard suppresses SSE overwrite of in-flight edits; gray-zone panel: Keep nearest / Accept proposal (only when ai_proposed link) / Move-to select (score 1.0); families edited whole-set (retained keep score, new = 1.0); Copy Brief, Copy Image Prompt, Re-assess, Delete; "Waiting for an API key" panel; agent-path copy block.
- Settings page: pairing via `<Link to="/pair" reloadDocument>` (content script only injects on document load) + manual "Show pairing token"; key set/verify/clear; models; thresholds; MCP toggle + both snippets; autostart w/ unsupported reason; sendToClaudeTarget; restartRequired notice.
- PromptEditor (TipTap → SolidJS equivalent needed): autosave; Copy Prompt = serialize→clipboard→markPromptSent; Send to Claude = serialize→copy→claim→launch (clipboard failure aborts launch; "Asked X to open" phrasing, never "opened").
- Editor: slash `/` (no menu after `http://` — allowedPrefixes [" "]); two-stage SlashMenu (palette: aesthetic|style|type|item + aliases → live multi-select); chip atoms familyChip("◈ "), tagChip, typeChip, itemRef("▣ ") all with label fallback; hidden `section` attr on paragraphs drives ghost-text from 7 template sections (Brief, Intent, Guardrails — Always, Guardrails — Never, Design Direction, Important, Output). The count read "8" here and the enumeration beside it has always listed seven; the original's `TEMPLATE_SECTIONS` holds seven, and FR-12 names seven. Each section is a real editable heading plus a paragraph, and each ghost is a worked example from the ACME brief rather than a description of what belongs there.
- Serialization is SERVER-side, authoritative: TipTap JSON → markdown-ish; chips: family → `Aesthetic: {name} — {description}`; tag/type → name; itemRef → `Reference: {name} — {abs dir} — read screenshot.png and item.md before designing; match feel, not content.`; collapse 3+ newlines; snapshot `prompts/{id}.md` with "Edits here are not read back" header.
- AppShell: nav Library/Projects/Prompts, centered search, + Add Item dialog (drop/paste/picker), New Project = create-then-navigate (no /prompts/new URL).
- Pair page: token absent from DOM until "Authorize this browser" click; then hidden `<div id="curio-pairing-handoff" data-curio-pairing-token>`; re-click harmless.

## 7. Extension (MV3, min Chrome 116)
- Permissions activeTab, scripting, storage, tabs; host_permissions 127.0.0.1 + localhost; pinned manifest `key` fixes the extension id (server pins the origin — one fact in two files).
- Modes: fold (frameBudget 1, default; anything ≠ "full" → fold = stale-popup safety) | full (budget 60, cap 20,000 device px).
- Pipeline: findServer → require token → active tab http(s) → suppressPageChrome FIRST (kill smooth-scroll, hide scrollbar — hide reflows layout, so suppress-then-measure) → measure → primeLazyContent (full only) → frame loop (scroll, sleep 550 ms captureVisibleTab rate limit, capture; hideFixedElements after frame 1, full only, <90% viewport height, visibility:hidden) → teardown UNCONDITIONAL success AND failure (restore fixed, scroll-behavior, style, scroll) → OffscreenCanvas stitch at offset*dpr → PNG → multipart with `x-curio-token`.
- Port probing: stored port → walk 4321-4331, 800 ms timeout; persist working port. storage.local keys: exactly `port`, `token` (mode NOT stored).
- Pairing pickup: content script on /pair only, MutationObserver, idempotent; gates: pathname /pair, element id, ≤512 chars, printable ASCII.
- Popup: listeners before first await; buttons disabled in HTML; save-token enabled immediately; both capture buttons disable together (shared tab scroll); toasts ("Capturing the visible area…", "Stitching the full page… don't switch tabs.", "Added ✓" + close 900 ms, "Captures will queue: no API key…"); 401 → "Pairing token rejected…". Openers use `http://localhost:{port}`.

## 8. Shell
- Boot order: --mcp-stdio branch → loadSettings → findLiveInstance (lock + /health probe; live → open browser + exit; stale lock deleted) → findAvailablePort (4321→4331) → persistPort → acquireLock (curio.lock `{pid, port, startedAt, quitToken}` 0600, quitToken 32 B hex) → openDatabase → startWorker → startWatcher → serve → quit handler → autostart backend → tray → openInBrowser.
- Tray (M5 pending in Bun app): interface onOpenLibrary/onOpenProjects/onNewPrompt/onQuit; currently signal handlers (SIGINT/SIGTERM/SIGBREAK). openNewPrompt = POST /api/prompts → open /prompts/{id}.
- Shutdown: tray.dispose → stopWorker → await stopWatcher → server.stop → closeDatabase → releaseLock → exit 0. Quit route: timing-safe token compare, answer HTTP first, shutdown after 150 ms grace.
- Autostart: backend seam; Windows HKCU Run key (`reg add /f`, quoted exe path; refuses in source mode); macOS/linux "politely unsupported" (rewrite: SMAppService); state read back from OS.
- Watcher: chokidar projectsRoot depth 0, awaitWriteFinish {2000,200}; addDir → register, unlinkDir → markMissing. Identity: `.curio-project` marker `{id: ulid, tool:"curio", note}` → fingerprint `mark:<ulid>`; read-only on scan, minted only on adoption; copies get fresh identity; inode fingerprints proven unsound (ext4/APFS reuse; birthtime coarse — 200 mkdirs → 6 distinct birthtimes). Prompt claim on first sighting only, 6 h window, claim cleared even when expired. resolveEntryPoint: index.html at root else newest numeric subfolder (v1..v5) with index.html else listing.

## 9. Jobs & AI
- Kinds: assess_item {itemId}; bulk_retag {item_ids 1-500, mode replace|augment, instruction ≤2000}; vocab_dedupe {}.
- Worker: single loop; claim = oldest queued where not_before null|due, atomic mark running; settings re-read per job; wake via notify + 2 s poll; startup reclaims orphaned running → queued.
- Failure: MissingApiKeyError → requeue WITHOUT burning attempt, 30 s backoff, item stays processing (FR-26 queueing); else retry attempts<3, backoff 2000·attempts²; exhausted → job failed + item assessment_failed. JobParkedError → stay queued, not_before set, attempt refunded. Cancellation polled at every boundary; route also cancels Anthropic batch.
- Bulk retag: membership FROZEN at enqueue (ids only); <8 serial (resume via progress.done), ≥8 Batch API (custom_id = item id), park + 5 s poll; progress in jobs.result {total,done,changed,failed,batch_id,via,note}; publish every 10; replace-mode finish prunes orphan vocab. cleanVocabularyNames: trim, ≤60 chars, case-insensitive dedupe. Augment = union; no-op detect case-insensitive.
- Dedupe: single utility call; schema has `reason` as FIRST property (generation-order fix); empty merge = withdrawal; post-filter drops hallucinated names + self-merges; result stored, never auto-applied.
- Anthropic: SDK per call, maxRetries 2. Vision: max_tokens 8000, structured output json_schema, effort medium, system = TWO cache breakpoints (rubric | vocabulary — single breakpoint measured 0 cache reads); user turn = base64 image + source/title/thresholds ("do not apply thresholds yourself"). Utility: max_tokens 2000, single cached block, NO `effort` (Haiku rejects it). Batch API: submit/status/cancel/results-by-custom_id. verifyApiKey = 16-token "Reply with OK.".
- Assessment output: {name_suggestion, short_description, design_types[], tags[], family_scores[{family, score}], new_family_proposal{name,description}|null, image_recipe|null}; zod re-validated. decideFamilies: best ≥ upper → assign all ≥ upper, ready; lower ≤ best < upper → nearest only, gray_zone, needs_review; best < lower + proposal → create family ai, link 1.0 ai_proposed, ready; else needs_review. User-renamed items keep their name.
- Images: sharp optional; thumbnail 640 px WebP q82 cropped to first fold (viewport aspect, fallback 16/10, clamp 0.5-4); vision payload ≤1568 px WebP q88, tall crop 4× width; ALL failures degrade to full-res PNG.

## 10. Gotchas (behavioral invariants the rewrite must preserve)
1. Extension origin pinned to manifest key — one fact in two files.
2. Same-origin check follows request Host, never configured port; loopback regex blocks DNS rebinding.
3. Quit token header must never enter /api/* CORS allowHeaders (paired extension must not get a kill switch). Quit token ≠ pairing token.
4. projectServeRefusal: dotfiles + reserved data-root files (lock, library.db prefix, config.json, .secrets.json, skills/, items/, prompts/); case-folded win32 only; judged on resolved target. (Leak precedent: quit token via /p/<id>/curio.lock.)
5. publicSettings strips pairingToken from a spread (schema omission alone defeated by spread).
6. /mcp above UI catch-all; disabled GET /mcp = 503 JSON-RPC, never SPA HTML; mcpEnabled per-request.
7. Two vision cache breakpoints; no effort param on utility calls.
8. Dedupe schema property order (reason first); empty merge = withdrawal.
9. Monotonic ULIDs; job claim tie-breaks on id.
10. parkJob refunds the attempt; not_before cleared by finish/requeue.
11. Bulk membership frozen at enqueue; over-cap = 409 refusal naming matched+limit, never trim.
12. last_edited_by: single PATCH stamps user; bulk touches only updated_at; MCP update stamps ai; re-assess keeps user names; failure preserves value.
13. Gray-zone one-way doors: whole-set PATCH clears gray zones + may auto-ready; bulk add preserves them; human-picked score = 1.0.
14. Family merge keeps MAX(score), MIN(gray_zone); tag/type merge = INSERT OR IGNORE + delete; prune orphan vocab after deletes/replaces.
15. Second-precision timestamps except markPromptSent (ms).
16. deletePrompt nulls projects.prompt_id first (FK no ON DELETE).
17. Marker-file identity: read-only scans, mint on adoption, fresh id for copies, fingerprint never overwritten (migration to change format).
18. Legacy env compat (CURIOL_*) + one-time data-root migration before target mkdir.
19. Sidecar = write-only projection, regenerated every mutation, DB wins.
20. Thumb-404 falls back to screenshot only when filename contains "thumb"; sharp fully optional.
21. Pairing handoff DOM element IS the authorization; strict gates.
22. Send-to-Claude ordering: serialize→copy→claim→launch; clipboard failure aborts; "asked to open" phrasing.
23. Popup listeners before await; buttons HTML-disabled; capture buttons disable together.
24. Capture: suppress-then-measure; hide fixed after frame 1 full-only; unconditional teardown.
25. item.created ignored by grid when filter/search active.
26. Bulk delete keeps bar mounted via notice.
27. FTS optional → LIKE fallback.
28. GET /api/projects performs repair side effects.
29. /health cross-origin-readable is a contract (fields included).
30. job.updated fires enqueue/start/progress/finish; UIs merge partials by id.
31. 64 MB body cap ↔ 20k px stitch cap.
32. Current deps for reference: hono, @modelcontextprotocol/sdk, @anthropic-ai/sdk, chokidar, sharp, ulid, yaml, zod; React 19, react-router 7, TipTap 3, Tailwind 4.
