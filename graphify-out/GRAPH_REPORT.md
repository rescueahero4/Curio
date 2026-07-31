# Graph Report - Curio  (2026-08-01)

## Corpus Check
- 202 files · ~154,339 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 1990 nodes · 4682 edges · 114 communities (105 shown, 9 thin omitted)
- Extraction: 99% EXTRACTED · 1% INFERRED · 0% AMBIGUOUS · INFERRED: 27 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `a55db217`
- Run `git rev-parse HEAD` and compare to check if the graph is stale.
- Run `graphify update .` after code changes (no API cost).

## Community Hubs (Navigation)
- [[_COMMUNITY_Community 0|Community 0]]
- [[_COMMUNITY_Community 1|Community 1]]
- [[_COMMUNITY_Community 2|Community 2]]
- [[_COMMUNITY_Community 3|Community 3]]
- [[_COMMUNITY_Community 4|Community 4]]
- [[_COMMUNITY_Community 5|Community 5]]
- [[_COMMUNITY_Community 6|Community 6]]
- [[_COMMUNITY_Community 7|Community 7]]
- [[_COMMUNITY_Community 8|Community 8]]
- [[_COMMUNITY_Community 9|Community 9]]
- [[_COMMUNITY_Community 10|Community 10]]
- [[_COMMUNITY_Community 11|Community 11]]
- [[_COMMUNITY_Community 12|Community 12]]
- [[_COMMUNITY_Community 13|Community 13]]
- [[_COMMUNITY_Community 14|Community 14]]
- [[_COMMUNITY_Community 15|Community 15]]
- [[_COMMUNITY_Community 16|Community 16]]
- [[_COMMUNITY_Community 17|Community 17]]
- [[_COMMUNITY_Community 18|Community 18]]
- [[_COMMUNITY_Community 19|Community 19]]
- [[_COMMUNITY_Community 20|Community 20]]
- [[_COMMUNITY_Community 21|Community 21]]
- [[_COMMUNITY_Community 22|Community 22]]
- [[_COMMUNITY_Community 23|Community 23]]
- [[_COMMUNITY_Community 24|Community 24]]
- [[_COMMUNITY_Community 25|Community 25]]
- [[_COMMUNITY_Community 26|Community 26]]
- [[_COMMUNITY_Community 27|Community 27]]
- [[_COMMUNITY_Community 28|Community 28]]
- [[_COMMUNITY_Community 29|Community 29]]
- [[_COMMUNITY_Community 30|Community 30]]
- [[_COMMUNITY_Community 31|Community 31]]
- [[_COMMUNITY_Community 32|Community 32]]
- [[_COMMUNITY_Community 33|Community 33]]
- [[_COMMUNITY_Community 34|Community 34]]
- [[_COMMUNITY_Community 35|Community 35]]
- [[_COMMUNITY_Community 36|Community 36]]
- [[_COMMUNITY_Community 37|Community 37]]
- [[_COMMUNITY_Community 38|Community 38]]
- [[_COMMUNITY_Community 39|Community 39]]
- [[_COMMUNITY_Community 40|Community 40]]
- [[_COMMUNITY_Community 41|Community 41]]
- [[_COMMUNITY_Community 42|Community 42]]
- [[_COMMUNITY_Community 43|Community 43]]
- [[_COMMUNITY_Community 44|Community 44]]
- [[_COMMUNITY_Community 45|Community 45]]
- [[_COMMUNITY_Community 46|Community 46]]
- [[_COMMUNITY_Community 47|Community 47]]
- [[_COMMUNITY_Community 48|Community 48]]
- [[_COMMUNITY_Community 49|Community 49]]
- [[_COMMUNITY_Community 50|Community 50]]
- [[_COMMUNITY_Community 51|Community 51]]
- [[_COMMUNITY_Community 52|Community 52]]
- [[_COMMUNITY_Community 53|Community 53]]
- [[_COMMUNITY_Community 54|Community 54]]
- [[_COMMUNITY_Community 55|Community 55]]
- [[_COMMUNITY_Community 56|Community 56]]
- [[_COMMUNITY_Community 57|Community 57]]
- [[_COMMUNITY_Community 58|Community 58]]
- [[_COMMUNITY_Community 59|Community 59]]
- [[_COMMUNITY_Community 60|Community 60]]
- [[_COMMUNITY_Community 61|Community 61]]
- [[_COMMUNITY_Community 62|Community 62]]
- [[_COMMUNITY_Community 64|Community 64]]
- [[_COMMUNITY_Community 65|Community 65]]
- [[_COMMUNITY_Community 67|Community 67]]
- [[_COMMUNITY_Community 68|Community 68]]
- [[_COMMUNITY_Community 69|Community 69]]
- [[_COMMUNITY_Community 70|Community 70]]
- [[_COMMUNITY_Community 71|Community 71]]
- [[_COMMUNITY_Community 72|Community 72]]
- [[_COMMUNITY_Community 73|Community 73]]
- [[_COMMUNITY_Community 74|Community 74]]
- [[_COMMUNITY_Community 75|Community 75]]
- [[_COMMUNITY_Community 76|Community 76]]
- [[_COMMUNITY_Community 77|Community 77]]
- [[_COMMUNITY_Community 78|Community 78]]
- [[_COMMUNITY_Community 79|Community 79]]
- [[_COMMUNITY_Community 80|Community 80]]
- [[_COMMUNITY_Community 81|Community 81]]
- [[_COMMUNITY_Community 82|Community 82]]
- [[_COMMUNITY_Community 83|Community 83]]
- [[_COMMUNITY_Community 84|Community 84]]
- [[_COMMUNITY_Community 85|Community 85]]
- [[_COMMUNITY_Community 86|Community 86]]
- [[_COMMUNITY_Community 87|Community 87]]
- [[_COMMUNITY_Community 88|Community 88]]
- [[_COMMUNITY_Community 89|Community 89]]
- [[_COMMUNITY_Community 90|Community 90]]
- [[_COMMUNITY_Community 91|Community 91]]
- [[_COMMUNITY_Community 92|Community 92]]
- [[_COMMUNITY_Community 93|Community 93]]
- [[_COMMUNITY_Community 94|Community 94]]
- [[_COMMUNITY_Community 95|Community 95]]
- [[_COMMUNITY_Community 96|Community 96]]
- [[_COMMUNITY_Community 97|Community 97]]
- [[_COMMUNITY_Community 98|Community 98]]
- [[_COMMUNITY_Community 99|Community 99]]
- [[_COMMUNITY_Community 100|Community 100]]
- [[_COMMUNITY_Community 101|Community 101]]
- [[_COMMUNITY_Community 102|Community 102]]
- [[_COMMUNITY_Community 103|Community 103]]
- [[_COMMUNITY_Community 104|Community 104]]
- [[_COMMUNITY_Community 105|Community 105]]
- [[_COMMUNITY_Community 106|Community 106]]
- [[_COMMUNITY_Community 108|Community 108]]
- [[_COMMUNITY_Community 109|Community 109]]
- [[_COMMUNITY_Community 110|Community 110]]
- [[_COMMUNITY_Community 111|Community 111]]
- [[_COMMUNITY_Community 113|Community 113]]

## God Nodes (most connected - your core abstractions)
1. `AppState` - 121 edges
2. `Connection` - 90 edges
3. `Option` - 90 edges
4. `Db` - 46 edges
5. `paused` - 33 edges
6. `Config` - 23 edges
7. `enqueue()` - 20 edges
8. `compilerOptions` - 20 edges
9. `ApiError` - 18 edges
10. `post()` - 18 edges

## Surprising Connections (you probably didn't know these)
- `heading_for()` --references--> `Option`  [EXTRACTED]
  crates/curio-core/src/prompt/template.rs → web/spa/src/components/library/OptionList.tsx
- `AssessmentOutput` --references--> `Option`  [EXTRACTED]
  crates/curio-core/src/assessment.rs → web/spa/src/components/library/OptionList.tsx
- `FamilyDecision` --references--> `Option`  [EXTRACTED]
  crates/curio-core/src/assessment.rs → web/spa/src/components/library/OptionList.tsx
- `decide_families()` --references--> `Option`  [EXTRACTED]
  crates/curio-core/src/assessment.rs → web/spa/src/components/library/OptionList.tsx
- `clean_vocabulary_name()` --references--> `Option`  [EXTRACTED]
  crates/curio-core/src/assessment.rs → web/spa/src/components/library/OptionList.tsx

## Import Cycles
- None detected.

## Communities (114 total, 9 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.13
Nodes (22): At a glance, Open questions, Parity obligations, The contract, At a glance, Open questions, Parity obligations, At a glance (+14 more)

### Community 1 - "Community 1"
Cohesion: 0.13
Nodes (15): At a glance, Data Architecture, Design detail, Graph role (designed in, activated post-v1 per D7), Hybrid query shape (illustrative, non-normative), Identity and time, Migrations, Open questions (+7 more)

### Community 2 - "Community 2"
Cohesion: 0.16
Nodes (11): Bootstrap — new primary, layered fallbacks, Bootstrap (new primary path), Capture pipeline (verbatim port — Inventory §7, §10.24), Capture — the sequence that must not change, Components & identity, Design detail, Non-Chrome browsers, Paused state, end to end (+3 more)

### Community 3 - "Community 3"
Cohesion: 0.17
Nodes (11): 10. Gotchas (behavioral invariants the rewrite must preserve), 1. HTTP API surface (as implemented), 2. MCP surface — 7 tools, one factory, both transports, 3. Events, 4. DB (SQLite, WAL, foreign_keys ON, busy_timeout 5000, synchronous NORMAL), 5. Settings / secrets / env, 6. UI routes & behaviors, 7. Extension (MV3, min Chrome 116) (+3 more)

### Community 4 - "Community 4"
Cohesion: 0.18
Nodes (11): Architecture Overview, At a glance, Decision register, Design detail, Document map, Glossary, Open questions, System boundaries (+3 more)

### Community 5 - "Community 5"
Cohesion: 0.18
Nodes (10): 0. Why this rewrite, 1. Goals & non-goals, 2. Users & stories, 3. Functional requirements, 4. Non-functional requirements, 5. UX requirements, 6. Epic list, 7. Done bar (demonstration) (+2 more)

### Community 6 - "Community 6"
Cohesion: 0.22
Nodes (9): Client token lifecycle (R-SEC-17), Design detail, Jail mechanics, Middleware order, Nonce mechanics, Review checklist (R-SEC-16), Threat model, in plain language, Token flow (+1 more)

### Community 7 - "Community 7"
Cohesion: 0.25
Nodes (8): At a glance, Deliberate breaks, Design detail — how to use this document, FR ownership, Inventory §10 invariant ownership, Open questions, Parity Matrix, The contract

### Community 8 - "Community 8"
Cohesion: 0.29
Nodes (7): Boot sequence, Budget (strategy §8 — binding targets), Crates, Design detail, HTTP surface (delta view), Jobs worker, Middleware map (what applies where)

### Community 9 - "Community 9"
Cohesion: 0.29
Nodes (7): Build & embed, Data flow — nonce, session, SSE, stores, Design detail, Pairing page as fallback, React → Solid translation map (non-normative porting guidance), The editor: TipTap core over raw ProseMirror, Why SolidJS — a structural argument, not a benchmark

### Community 10 - "Community 10"
Cohesion: 0.07
Nodes (26): For /graphify add and --watch, For /graphify query, For the commit hook and native CLAUDE.md integration, For --update and --cluster-only, /graphify, Honesty Rules, Interpreter guard for subcommands, Part A - Structural extraction for code files (+18 more)

### Community 11 - "Community 11"
Cohesion: 0.33
Nodes (6): Design detail, Request lifecycle (HTTP), Token-overhead discipline, Tool surface — parity seven (schemas fixed, Inventory §2), Tool surface — semantic two (new in the rewrite, post-v1), Transport / client matrix

### Community 12 - "Community 12"
Cohesion: 0.40
Nodes (5): CI pipeline, D0 verification-spike index (release-0) — every D0-verify item in the doc set, Design detail, Dev loop, Why the dependency rules are strict

### Community 13 - "Community 13"
Cohesion: 0.07
Nodes (34): Error, PathBuf, a_current_directory_segment_is_harmless(), a_jail_inside_a_dot_prefixed_directory_still_serves_its_own_files(), a_path_that_does_not_exist_still_stays_inside(), a_project_file_that_merely_shares_a_prefix_is_served(), a_target_that_cannot_be_placed_relative_to_the_jail_is_refused(), a_traversal_escapes_nothing() (+26 more)

### Community 14 - "Community 14"
Cohesion: 0.22
Nodes (8): graphify reference: extra exports and benchmark, Step 6b - Wiki (only if --wiki flag), Step 7 - Neo4j export (only if --neo4j or --neo4j-push flag), Step 7a - FalkorDB export (only if --falkordb or --falkordb-push flag), Step 7b - SVG export (only if --svg flag), Step 7c - GraphML export (only if --graphml flag), Step 7d - MCP server (only if --mcp flag), Step 8 - Token reduction benchmark (only if total_words > 5000)

### Community 15 - "Community 15"
Cohesion: 0.33
Nodes (5): For /graphify explain, For /graphify path, graphify reference: query, path, explain, Step 0 — Constrained query expansion (REQUIRED before traversal), Step 1 — Traversal

### Community 16 - "Community 16"
Cohesion: 0.40
Nodes (4): Frontmatter (every doc, exactly this shape), Structure (progressive reveal — strict order), Style, TEMPLATE — Authoring rules for Curio architecture documents (binding)

### Community 17 - "Community 17"
Cohesion: 0.50
Nodes (3): For /graphify add, For --watch, graphify reference: add a URL and watch a folder

### Community 18 - "Community 18"
Cohesion: 0.50
Nodes (3): For git commit hook, For native CLAUDE.md integration, graphify reference: commit hook and native CLAUDE.md integration

### Community 19 - "Community 19"
Cohesion: 0.50
Nodes (3): For --cluster-only, For --update (incremental re-extraction), graphify reference: incremental update and cluster-only

### Community 22 - "Community 22"
Cohesion: 0.18
Nodes (10): Behavior, Goal and Architecture, Graphify, Live commentary of corrections, Project, Responsding, Scoping, Tool calling issues (+2 more)

### Community 25 - "Community 25"
Cohesion: 0.08
Nodes (22): EVENT_NAMES, EventName, events, EventStream, Handler, bootstrapSession(), probeSession(), serverIsUp() (+14 more)

### Community 26 - "Community 26"
Cohesion: 0.06
Nodes (57): CreatedBy, Family, a_bulk_add_preserves_a_gray_zone_it_knows_nothing_about(), a_bulk_edit_does_not_claim_a_human_edited_the_fields(), a_bulk_edit_skips_an_item_that_vanished_from_under_the_selection(), add_term(), bulk_edit(), BulkEdit (+49 more)

### Community 27 - "Community 27"
Cohesion: 0.06
Nodes (43): AsHeaderName, Body, Builder, HeaderMap, HttpRequest, Next, Request, a_bearer_token_authenticates() (+35 more)

### Community 28 - "Community 28"
Cohesion: 0.29
Nodes (13): Enqueued, open_in_os(), open_skill_file(), Outcome, publish_job(), quit(), reassess(), reveal() (+5 more)

### Community 29 - "Community 29"
Cohesion: 0.13
Nodes (22): api_key_configured_is_a_boolean_not_a_prefix(), api_key_is_configured(), counts_come_from_the_library(), handler(), Health, snapshot(), state(), status_stays_ok_while_paused() (+14 more)

### Community 30 - "Community 30"
Cohesion: 0.23
Nodes (10): OffsetDateTime, format_millis(), format_seconds(), millisecond_precision_always_emits_three_digits(), now_iso(), now_iso_millis(), parse(), second_precision_is_fixed_width() (+2 more)

### Community 31 - "Community 31"
Cohesion: 0.24
Nodes (6): Formatter, Self, deleted_carries_only_an_id(), Event, EventName, wire_names_are_the_ones_clients_already_listen_for()

### Community 32 - "Community 32"
Cohesion: 0.16
Nodes (9): Default, an_absent_port_stays_absent_through_a_round_trip(), Config, defaults_match_the_shipped_values(), field_names_are_camel_case_on_disk(), Models, SendToClaudeTarget, the_pairing_token_field_is_gone() (+1 more)

### Community 33 - "Community 33"
Cohesion: 0.07
Nodes (27): dependencies, solid-js, @solidjs/router, @tiptap/core, @tiptap/pm, @tiptap/starter-kit, description, devDependencies (+19 more)

### Community 34 - "Community 34"
Cohesion: 0.08
Nodes (23): source, assist, actions, css, parser, files, includes, formatter (+15 more)

### Community 35 - "Community 35"
Cohesion: 0.08
Nodes (28): main(), JoinHandle, MenuItem, open_existing_instance(), open_url(), request_dashboard(), run_app(), run_mcp_stdio() (+20 more)

### Community 36 - "Community 36"
Cohesion: 0.09
Nodes (22): compilerOptions, baseUrl, isolatedModules, jsx, jsxImportSource, lib, module, moduleResolution (+14 more)

### Community 37 - "Community 37"
Cohesion: 0.11
Nodes (18): description, devDependencies, @biomejs/biome, esbuild, @types/chrome, @types/node, typescript, name (+10 more)

### Community 38 - "Community 38"
Cohesion: 0.11
Nodes (17): compilerOptions, isolatedModules, lib, module, moduleResolution, noEmit, noFallthroughCasesInSwitch, noImplicitOverride (+9 more)

### Community 39 - "Community 39"
Cohesion: 0.12
Nodes (15): action, default_popup, default_title, background, service_worker, type, content_scripts, description (+7 more)

### Community 40 - "Community 40"
Cohesion: 0.13
Nodes (14): source, assist, actions, files, includes, formatter, enabled, indentStyle (+6 more)

### Community 41 - "Community 41"
Cohesion: 0.16
Nodes (4): Access, Refusal, refusals_carry_a_machine_readable_reason(), Tool

### Community 42 - "Community 42"
Cohesion: 0.15
Nodes (12): 1. Correction, 2. Warning, 3. Temporary Ban, 4. Permanent Ban, Attribution, Contributor Covenant Code of Conduct, Enforcement, Enforcement Guidelines (+4 more)

### Community 43 - "Community 43"
Cohesion: 0.19
Nodes (5): Job, JobKind, JobStatus, kind_strings_match_the_stored_values(), status_strings_match_the_check_constraint()

### Community 44 - "Community 44"
Cohesion: 0.08
Nodes (33): BTreeSet, origin_strings_match_the_check_constraint(), Project, ProjectMarker, ProjectOrigin, ProjectStatus, HashMap, Instant (+25 more)

### Community 45 - "Community 45"
Cohesion: 0.21
Nodes (5): CreatedBy, every_kind_has_a_distinct_pair_of_tables(), Family, Term, VocabularyKind

### Community 46 - "Community 46"
Cohesion: 0.13
Nodes (15): D0 Verification Spike — results, **E0 is COMPLETE.**, Findings, How to record a result, New row, added at scaffold, Process — R-DEL-20's merge gate was crossed, and this is the record of it, Release-0 rows, Row 13 — verifying it found that `/ws` had never worked (+7 more)

### Community 47 - "Community 47"
Cohesion: 0.12
Nodes (6): Item, ItemFamily, ItemStatus, LastEditedBy, link(), status_strings_match_the_check_constraint()

### Community 48 - "Community 48"
Cohesion: 0.17
Nodes (24): PausedBanner(), Note(), paused, Settings, SettingsPatch, ApiKeySection(), CopyBlock(), McpSection() (+16 more)

### Community 49 - "Community 49"
Cohesion: 0.12
Nodes (14): indexOf(), ItemsState, [itemsState, setItemsState], JobsState, [jobsState, setJobsState], onItemCreated(), onItemUpdated(), setFilterActive() (+6 more)

### Community 50 - "Community 50"
Cohesion: 0.25
Nodes (8): A note on the D0 spike, Commits and pull requests, Contributing to Curio, Making a decision, One gate, The rules that will bite you first, Two commands to a running app, Which document owns what

### Community 51 - "Community 51"
Cohesion: 0.29
Nodes (7): Editor, Framework & structure, Keyboard & a11y (SHOULD — post-parity baseline), Pairing fallback, Ported UX behaviors (observable contracts, Inventory §6 + §10), State & session, The contract

### Community 52 - "Community 52"
Cohesion: 0.29
Nodes (7): Build, Curio, Documentation, License, Privacy, Repository layout, Shape

### Community 53 - "Community 53"
Cohesion: 0.29
Nodes (6): For contributors, Reporting a vulnerability, Scope, Security Policy, Threat model, What Curio does with your data

### Community 54 - "Community 54"
Cohesion: 0.33
Nodes (5): Build, Curio Capture (MV3), Development installs, Status, The `key` field is load-bearing

### Community 55 - "Community 55"
Cohesion: 0.33
Nodes (5): Checklist, Rules implemented or touched, Security review (R-SEC-16), Verification, What this changes

### Community 56 - "Community 56"
Cohesion: 0.21
Nodes (23): explicit_null(), D, Item, finish(), sync_fts(), a_bulk_touch_does_not_claim_to_be_an_edit(), a_dashboard_edit_stamps_the_user(), a_failed_assessment_preserves_authorship() (+15 more)

### Community 57 - "Community 57"
Cohesion: 0.14
Nodes (38): a_confident_assessment_lands_the_item_ready(), a_gray_zone_assessment_holds_the_item_for_a_decision(), a_proposal_creates_the_family_with_its_description(), a_re_assessment_keeps_a_name_the_user_chose(), accepting_a_proposal_that_does_not_exist_is_refused(), accepting_the_nearest_family_clears_the_badge(), apply_assessment(), family() (+30 more)

### Community 58 - "Community 58"
Cohesion: 0.08
Nodes (32): AppShell(), BrandMark(), MissingKeyBanner(), DESTINATIONS, NavPills(), SearchBox(), TopBar(), attach() (+24 more)

### Community 59 - "Community 59"
Cohesion: 0.60
Nodes (4): isAcceptableToken(), isPairPage(), observer, pickUp()

### Community 61 - "Community 61"
Cohesion: 0.11
Nodes (20): entries, watch, createTerm(), mergeTerm(), refreshVocabulary(), CreatedBy, VocabularyKind, ensureVocabulary() (+12 more)

### Community 62 - "Community 62"
Cohesion: 0.20
Nodes (16): ItemQuery, a_blank_search_does_not_filter_anything_out(), a_status_facet_filters_on_the_stored_string(), a_tag_and_a_type_narrow_the_result(), build_where(), items_come_back_with_their_vocabulary_attached(), items_sharing_a_timestamp_are_ordered_by_id(), matching_ids() (+8 more)

### Community 64 - "Community 64"
Cohesion: 0.18
Nodes (10): Error, From, IntoResponse, an_over_cap_refusal_carries_both_numbers(), ApiError, body_of(), every_error_carries_a_machine_readable_code(), paused_is_a_503_with_a_retry_after() (+2 more)

### Community 65 - "Community 65"
Cohesion: 0.21
Nodes (29): Job, JobStatus, a_crash_leaves_no_job_stranded(), a_finished_job_leaves_the_queue_count(), a_park_that_has_come_due_is_claimable_again(), a_parked_job_is_invisible_until_it_is_due(), cancel(), cancelling_a_finished_job_is_not_an_error() (+21 more)

### Community 67 - "Community 67"
Cohesion: 0.10
Nodes (13): Arc, Debug, FnOnce, MutexGuard, authenticate(), send_state(), serve(), upgrade() (+5 more)

### Community 68 - "Community 68"
Cohesion: 0.07
Nodes (49): basename(), bulkEdit(), cancelJob(), clearApiKey(), clearPromptSent(), deletePrompt(), deleteTerm(), fileUrl() (+41 more)

### Community 69 - "Community 69"
Cohesion: 0.18
Nodes (15): ActionOutcome, copyPrompt(), copyToClipboard(), sendPromptToClaude(), PromptActions(), Props, ghostMap(), CLOCK (+7 more)

### Community 70 - "Community 70"
Cohesion: 0.14
Nodes (15): ids(), list(), collect_names(), get(), hydrate(), map_row(), parse_authorship(), parse_status() (+7 more)

### Community 71 - "Community 71"
Cohesion: 0.24
Nodes (8): Send, Clock, Embedder, EventSink, NullEventSink, SystemClock, the_null_sink_accepts_anything(), Sync

### Community 72 - "Community 72"
Cohesion: 0.16
Nodes (9): createPrompt(), NotFound(), Pair(), Projects(), Prompts(), WHEN, Settings(), App() (+1 more)

### Community 73 - "Community 73"
Cohesion: 0.14
Nodes (33): get(), open(), Opened, resolve_entry(), chip_context(), clear_sent(), create(), CreateBody (+25 more)

### Community 74 - "Community 74"
Cohesion: 0.10
Nodes (23): deleteItem(), resolveGrayZone(), vocabulary, GrayZoneAction, Item, ItemPatch, Autosave, createAutosave() (+15 more)

### Community 75 - "Community 75"
Cohesion: 0.17
Nodes (26): count(), Project, ProjectOrigin, Result, reclaim_orphans(), a_copy_of_a_folder_is_a_different_project(), a_missing_folder_that_reappears_comes_back_present(), a_record_predating_marker_identity_gets_one_backfilled() (+18 more)

### Community 76 - "Community 76"
Cohesion: 0.21
Nodes (23): Prompt, Connection, a_document_round_trips_as_a_value_not_a_string(), a_fresh_claim_is_the_one_a_new_project_gets(), a_new_prompt_starts_from_the_template(), an_expired_claim_is_not_offered(), an_unsent_prompt_stakes_no_claim(), clear_sent() (+15 more)

### Community 77 - "Community 77"
Cohesion: 0.19
Nodes (13): a_family_link_carries_its_score_and_flags(), a_missing_source_url_is_null_rather_than_absent(), an_empty_image_recipe_is_treated_as_none(), an_image_recipe_appears_only_when_there_is_one(), empty_lists_are_written_as_empty_lists(), field(), frontmatter_is_delimited_and_carries_every_field(), item() (+5 more)

### Community 78 - "Community 78"
Cohesion: 0.37
Nodes (15): a_chip_whose_row_is_gone_falls_back_to_its_label(), a_family_chip_carries_its_description(), a_section_with_content_gets_its_heading(), an_item_chip_becomes_an_absolute_path_with_reading_instructions(), an_unknown_node_keeps_its_text(), an_untouched_ghost_section_disappears(), collapse_blank_runs(), context() (+7 more)

### Community 79 - "Community 79"
Cohesion: 0.11
Nodes (18): SectionOptions, Sections, CreatedItem, Family, GrayZoneDecision, Health, ItemFamily, JobStatus (+10 more)

### Community 80 - "Community 80"
Cohesion: 0.19
Nodes (26): Response, a_fresh_nonce_mints_a_session(), a_replayed_nonce_is_rejected(), an_invented_nonce_is_rejected_without_setting_a_cookie(), exchange(), ExchangeRequest, logging_out_clears_the_cookie(), logout() (+18 more)

### Community 81 - "Community 81"
Cohesion: 0.22
Nodes (16): a_fresh_database_reaches_the_latest_version(), a_newer_database_refuses_to_open(), add_column(), at_version(), column_exists(), current_version(), fresh(), migrating_twice_is_a_no_op() (+8 more)

### Community 82 - "Community 82"
Cohesion: 0.18
Nodes (7): I, env_flag(), Invocation, parse(), port_override(), read_port(), usage()

### Community 83 - "Community 83"
Cohesion: 0.13
Nodes (20): an_unknown_parameter_does_not_break_a_bookmarked_url(), bulk_edit(), BulkBody, BulkResult, delete(), list(), pairs(), parse_query() (+12 more)

### Community 84 - "Community 84"
Cohesion: 0.38
Nodes (10): ChipContext, expand_chip(), inline_text(), list_item_text(), marked_text(), paragraph(), push_block(), text() (+2 more)

### Community 85 - "Community 85"
Cohesion: 0.21
Nodes (15): a_fresh_data_root_gets_its_directories_and_a_rubric(), a_malformed_config_is_repaired_rather_than_fatal(), a_quit_token_is_minted_into_the_lock_file(), invalid_values_are_reset_and_written_back_visibly(), load_config(), materialize(), migrate_legacy_root(), mint_quit_token() (+7 more)

### Community 86 - "Community 86"
Cohesion: 0.24
Nodes (9): Receiver, a_subscriber_receives_what_is_published(), clones_share_one_state(), debugging_the_state_does_not_print_the_token(), nonces_round_trip_through_the_state(), pausing_is_reversible_and_reported(), publishing_with_nobody_listening_is_fine(), state() (+1 more)

### Community 87 - "Community 87"
Cohesion: 0.22
Nodes (5): a_new_document_is_a_tiptap_doc_with_one_paragraph_per_section(), empty_document(), ghost_text_is_not_in_the_document(), heading_for(), Section

### Community 88 - "Community 88"
Cohesion: 0.07
Nodes (37): chipExtensions, ChipKind, ChipSpec, label(), SPECS, text(), createPromptEditor(), PromptEditorConfig (+29 more)

### Community 89 - "Community 89"
Cohesion: 0.25
Nodes (7): Generator, generate(), IdGenerator, ids_are_unique(), ids_from_one_generator_always_ascend(), parse(), Ulid

### Community 90 - "Community 90"
Cohesion: 0.18
Nodes (17): a_mask_shows_four_characters_and_no_more(), autostart_support(), AutostartSupport, clear_api_key(), get(), mask(), persist(), project() (+9 more)

### Community 91 - "Community 91"
Cohesion: 0.22
Nodes (16): an_unknown_segment_says_what_was_expected(), announce(), create(), CreateBody, Created, delete(), merge(), MergeBody (+8 more)

### Community 92 - "Community 92"
Cohesion: 0.18
Nodes (8): AtomicBool, Into, Mutex, NonceStore, Notify, RuntimeToken, Sender, Inner

### Community 93 - "Community 93"
Cohesion: 0.18
Nodes (3): createItem(), ApiError, OverCap

### Community 94 - "Community 94"
Cohesion: 0.70
Nodes (4): main(), locate(), read_message(), write_message()

### Community 95 - "Community 95"
Cohesion: 0.15
Nodes (7): a_blank_search_box_does_not_count_as_a_filter(), any_facet_makes_a_query_filtered(), Cursor, enforce_bulk_cap(), ItemQuery, Selection, the_bulk_cap_refuses_rather_than_trims()

### Community 96 - "Community 96"
Cohesion: 0.83
Nodes (3): a_snapshot_carries_the_title_and_the_text(), a_snapshot_warns_that_it_is_not_read_back(), snapshot()

### Community 97 - "Community 97"
Cohesion: 0.25
Nodes (14): a_non_numeric_folder_is_not_a_version(), a_project_with_nothing_to_open_says_so_rather_than_guessing(), a_root_index_beats_a_versioned_one(), a_root_index_is_the_front_door(), known_status(), list(), publish(), register() (+6 more)

### Community 98 - "Community 98"
Cohesion: 0.07
Nodes (40): AddItemDialog(), File, createEscapeLayer(), items, BulkEdit, ItemFilter, ItemQuery, ItemStatus (+32 more)

### Community 100 - "Community 100"
Cohesion: 0.29
Nodes (5): create(), default_name(), Ingested, Multipart, StatusCode

### Community 101 - "Community 101"
Cohesion: 0.30
Nodes (14): a_prompt_snapshot_carries_the_do_not_edit_header(), an_in_memory_library_projects_nowhere(), item(), no_temporary_file_is_left_behind(), remove_item(), remove_prompt(), removing_a_prompt_snapshot_is_best_effort(), removing_an_item_takes_its_directory_with_it() (+6 more)

### Community 102 - "Community 102"
Cohesion: 0.28
Nodes (9): a_quote_in_the_query_does_not_break_the_expression(), build_match_query(), create(), earlier_words_are_not_wildcarded(), extra_words_narrow_rather_than_widen(), fts5_is_available_because_we_bundle_sqlite(), fts_operators_typed_by_a_user_are_searched_for_not_executed(), indexed() (+1 more)

### Community 103 - "Community 103"
Cohesion: 0.23
Nodes (6): an_unbuilt_dashboard_explains_itself(), Assets, is_built(), is_reserved(), not_built(), serve()

### Community 104 - "Community 104"
Cohesion: 0.44
Nodes (8): a_family_the_user_keeps_holds_on_to_its_score(), editing_an_item_keeps_it_findable_under_its_new_name(), library(), new_item(), set_families(), set_terms(), setting_the_family_set_promotes_an_item_out_of_review(), tags_are_replaced_whole_and_created_on_demand()

### Community 106 - "Community 106"
Cohesion: 0.27
Nodes (8): Drop, HANDLE, a_second_guard_is_refused_while_the_first_is_held(), acquiring_creates_the_parent_directory(), Error, InstanceGuard, the_lock_can_be_taken_and_released(), two_different_libraries_can_both_be_open()

### Community 109 - "Community 109"
Cohesion: 0.50
Nodes (3): Packaging, The NM manifest stays data-driven (R-EXT-20), Uninstall is a feature (R-DEL-11)

### Community 110 - "Community 110"
Cohesion: 0.47
Nodes (4): Infallible, stream(), Sse, SseEvent

### Community 111 - "Community 111"
Cohesion: 0.60
Nodes (3): find_curio(), private_memory_kib(), report()

## Knowledge Gaps
- **392 isolated node(s):** `Section`, `Selection`, `Error`, `Section`, `Access` (+387 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **9 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `Option` connect `Community 56` to `Community 13`, `Community 26`, `Community 27`, `Community 32`, `Community 35`, `Community 43`, `Community 44`, `Community 47`, `Community 57`, `Community 58`, `Community 65`, `Community 67`, `Community 70`, `Community 73`, `Community 74`, `Community 75`, `Community 76`, `Community 77`, `Community 80`, `Community 82`, `Community 83`, `Community 87`, `Community 90`, `Community 91`, `Community 95`, `Community 97`, `Community 98`, `Community 100`, `Community 101`, `Community 102`, `Community 105`?**
  _High betweenness centrality (0.167) - this node is a cross-community bridge._
- **Why does `AppState` connect `Community 67` to `Community 97`, `Community 100`, `Community 58`, `Community 71`, `Community 27`, `Community 73`, `Community 44`, `Community 13`, `Community 110`, `Community 92`, `Community 80`, `Community 83`, `Community 86`, `Community 90`, `Community 91`, `Community 28`, `Community 29`?**
  _High betweenness centrality (0.057) - this node is a cross-community bridge._
- **Why does `Db` connect `Community 26` to `Community 65`, `Community 67`, `Community 70`, `Community 104`, `Community 73`, `Community 75`, `Community 76`, `Community 13`, `Community 86`, `Community 56`, `Community 57`, `Community 91`, `Community 92`, `Community 29`, `Community 62`?**
  _High betweenness centrality (0.056) - this node is a cross-community bridge._
- **What connects `Section`, `Selection`, `Error` to the rest of the system?**
  _392 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 0` be split into smaller, more focused modules?**
  _Cohesion score 0.13368983957219252 - nodes in this community are weakly interconnected._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.13333333333333333 - nodes in this community are weakly interconnected._
- **Should `Community 10` be split into smaller, more focused modules?**
  _Cohesion score 0.07407407407407407 - nodes in this community are weakly interconnected._