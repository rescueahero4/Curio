# Graph Report - Curio  (2026-07-31)

## Corpus Check
- 96 files · ~81,090 words
- Verdict: corpus is large enough that graph structure adds value.

## Summary
- 920 nodes · 1351 edges · 67 communities (61 shown, 6 thin omitted)
- Extraction: 100% EXTRACTED · 0% INFERRED · 0% AMBIGUOUS · INFERRED: 2 edges (avg confidence: 0.8)
- Token cost: 0 input · 0 output

## Graph Freshness
- Built from commit: `083b0a64`
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

## God Nodes (most connected - your core abstractions)
1. `AppState` - 25 edges
2. `Connection` - 20 edges
3. `compilerOptions` - 20 edges
4. `compilerOptions` - 16 edges
5. `Db` - 14 edges
6. `What You Must Do When Invoked` - 12 edges
7. `run()` - 11 edges
8. `Service` - 11 edges
9. `/graphify` - 11 edges
10. `Curio parity inventory — mined from the running Bun/React implementation (Curiol repo)` - 11 edges

## Surprising Connections (you probably didn't know these)
- `create()` --references--> `Connection`  [EXTRACTED]
  crates/curio-db/src/fts.rs → web/extension/src/shared/storage.ts
- `indexed()` --references--> `Connection`  [EXTRACTED]
  crates/curio-db/src/fts.rs → web/extension/src/shared/storage.ts
- `search()` --references--> `Connection`  [EXTRACTED]
  crates/curio-db/src/fts.rs → web/extension/src/shared/storage.ts
- `Db` --references--> `Connection`  [EXTRACTED]
  crates/curio-db/src/lib.rs → web/extension/src/shared/storage.ts
- `configure()` --references--> `Connection`  [EXTRACTED]
  crates/curio-db/src/lib.rs → web/extension/src/shared/storage.ts

## Import Cycles
- None detected.

## Communities (67 total, 6 thin omitted)

### Community 0 - "Community 0"
Cohesion: 0.27
Nodes (7): At a glance, Open questions, Parity obligations, At a glance, Open questions, Parity obligations, The contract

### Community 1 - "Community 1"
Cohesion: 0.13
Nodes (15): At a glance, Data Architecture, Design detail, Graph role (designed in, activated post-v1 per D7), Hybrid query shape (illustrative, non-normative), Identity and time, Migrations, Open questions (+7 more)

### Community 2 - "Community 2"
Cohesion: 0.29
Nodes (7): Bootstrap (new primary path), Capture pipeline (verbatim port — Inventory §7, §10.24), Components & identity, Non-Chrome browsers, Popup (Inventory §7, §10.23), The contract, Worker lifetime & state

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
Cohesion: 0.18
Nodes (10): At a glance, Design detail, Open questions, Parity obligations, Request lifecycle (HTTP), The contract, Token-overhead discipline, Tool surface — parity seven (schemas fixed, Inventory §2) (+2 more)

### Community 12 - "Community 12"
Cohesion: 0.40
Nodes (5): CI pipeline, D0 verification-spike index (release-0) — every D0-verify item in the doc set, Design detail, Dev loop, Why the dependency rules are strict

### Community 13 - "Community 13"
Cohesion: 0.07
Nodes (45): Error, main(), JoinHandle, PathBuf, Receiver, Result, Sender, find_curio() (+37 more)

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
Cohesion: 0.22
Nodes (8): Behavior, Graphify, Live commentary of corrections, Responsding, Scoping, Tool calling issues, When to delegate to Agents, Writing Docs

### Community 25 - "Community 25"
Cohesion: 0.06
Nodes (29): AppShell(), EVENT_NAMES, EventName, events, EventStream, Handler, bootstrapSession(), probeSession() (+21 more)

### Community 26 - "Community 26"
Cohesion: 0.08
Nodes (34): buttons, dot, hint, render(), setCaptureEnabled(), text, clearConnection(), Connection (+26 more)

### Community 27 - "Community 27"
Cohesion: 0.06
Nodes (25): I, Option, decode_base64(), extension_id(), host_is_loopback(), hostname_of(), origin_is_allowed(), sec_fetch_site_is_hostile() (+17 more)

### Community 28 - "Community 28"
Cohesion: 0.09
Nodes (29): Arc, AtomicBool, Debug, FnOnce, Formatter, Into, Json, Mutex (+21 more)

### Community 29 - "Community 29"
Cohesion: 0.09
Nodes (26): Drop, File, HANDLE, Path, check(), is_source(), walk(), a_dead_pid_reads_as_stale() (+18 more)

### Community 30 - "Community 30"
Cohesion: 0.10
Nodes (22): main(), OffsetDateTime, Send, usage(), quote_ident(), Clock, Embedder, EventSink (+14 more)

### Community 31 - "Community 31"
Cohesion: 0.11
Nodes (15): Error, main(), Display, From, Self, Error, deleted_carries_only_an_id(), Event (+7 more)

### Community 32 - "Community 32"
Cohesion: 0.10
Nodes (15): Default, Generator, an_absent_port_stays_absent_through_a_round_trip(), Config, defaults_match_the_shipped_values(), field_names_are_camel_case_on_disk(), Models, SendToClaudeTarget (+7 more)

### Community 33 - "Community 33"
Cohesion: 0.08
Nodes (24): dependencies, solid-js, @solidjs/router, description, devDependencies, @biomejs/biome, tailwindcss, @tailwindcss/vite (+16 more)

### Community 34 - "Community 34"
Cohesion: 0.08
Nodes (23): source, assist, actions, css, parser, files, includes, formatter (+15 more)

### Community 35 - "Community 35"
Cohesion: 0.11
Nodes (10): MenuItem, run(), ServiceThreadConfig, Status, icon(), the_icon_buffer_is_the_size_it_claims(), TrayMenu, StatusSender (+2 more)

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
Cohesion: 0.20
Nodes (4): JobKind, JobStatus, kind_strings_match_the_stored_values(), status_strings_match_the_check_constraint()

### Community 44 - "Community 44"
Cohesion: 0.27
Nodes (10): BTreeSet, HashMap, Metadata, Package, PackageId, check(), find_package(), Reach (+2 more)

### Community 45 - "Community 45"
Cohesion: 0.22
Nodes (3): CreatedBy, every_kind_has_a_distinct_pair_of_tables(), VocabularyKind

### Community 46 - "Community 46"
Cohesion: 0.20
Nodes (10): D0 Verification Spike — results, Findings, How to record a result, New row, added at scaffold, Release-0 rows, Row 1 — the single-instance guard is per-library, not per-machine, Row 2 — the measurement method changes the answer by three orders of magnitude, Row 7 — rmcp has moved to a major version the contract does not cover (+2 more)

### Community 47 - "Community 47"
Cohesion: 0.25
Nodes (3): ItemStatus, LastEditedBy, status_strings_match_the_check_constraint()

### Community 48 - "Community 48"
Cohesion: 0.25
Nodes (3): origin_strings_match_the_check_constraint(), ProjectOrigin, ProjectStatus

### Community 49 - "Community 49"
Cohesion: 0.25
Nodes (7): At a glance, Bootstrap — new primary, layered fallbacks, Capture — the sequence that must not change, Design detail, Open questions, Parity obligations, Paused state, end to end

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
Cohesion: 0.40
Nodes (4): At a glance, Open questions, Parity obligations, The contract

### Community 57 - "Community 57"
Cohesion: 0.40
Nodes (4): At a glance, Open questions, Parity obligations, The contract

### Community 58 - "Community 58"
Cohesion: 0.40
Nodes (3): Packaging, The NM manifest stays data-driven (R-EXT-20), Uninstall is a feature (R-DEL-11)

### Community 59 - "Community 59"
Cohesion: 0.60
Nodes (4): isAcceptableToken(), isPairPage(), observer, pickUp()

## Knowledge Gaps
- **335 isolated node(s):** `Error`, `Assets`, `UserEvent`, `$schema`, `includes` (+330 more)
  These have ≤1 connection - possible missing edges or undocumented components.
- **6 thin communities (<3 nodes) omitted from report** — run `graphify query` to explore isolated nodes.

## Suggested Questions
_Questions this graph is uniquely positioned to answer:_

- **Why does `AppState` connect `Community 28` to `Community 25`, `Community 13`?**
  _High betweenness centrality (0.076) - this node is a cross-community bridge._
- **Why does `Db` connect `Community 26` to `Community 28`, `Community 13`?**
  _High betweenness centrality (0.070) - this node is a cross-community bridge._
- **Why does `Inner` connect `Community 28` to `Community 26`, `Community 30`?**
  _High betweenness centrality (0.058) - this node is a cross-community bridge._
- **What connects `Error`, `Assets`, `UserEvent` to the rest of the system?**
  _335 weakly-connected nodes found - possible documentation gaps or missing edges._
- **Should `Community 1` be split into smaller, more focused modules?**
  _Cohesion score 0.13333333333333333 - nodes in this community are weakly interconnected._
- **Should `Community 10` be split into smaller, more focused modules?**
  _Cohesion score 0.07407407407407407 - nodes in this community are weakly interconnected._
- **Should `Community 13` be split into smaller, more focused modules?**
  _Cohesion score 0.0662004662004662 - nodes in this community are weakly interconnected._