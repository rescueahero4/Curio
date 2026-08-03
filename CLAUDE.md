## Project
### Goal and Architecture
- Orient understanding of the project, objectives and wotk items from PRD 
`docs\PRD-01-Foundations.md`

- Always refer to Solutions Architecture to ensure current and future development always align with the original Solutions Architecture plan
`docs\architecture\00-architecture-overview.md`


## Behavior
### Responding
Keep responses focused, brief, and concise. Keep disclaimers and caveats short, and spend most of the response on the main answer. When asked to explain something, give a high-level summary unless an in-depth explanation is specifically requested.Keep responses focused, brief, and concise. Keep disclaimers and caveats short, and spend most of the response on the main answer. When asked to explain something, give a high-level summary unless an in-depth explanation is specifically requested.

### When taking action
Do not interpret questions as task - when asked simply respond to the question with the best of your knowledge, or if the question requires you do perform deep dive analysis on the codebase or researh, always return with the intetnt to answer the question and not make any changes unless explicitly asked by the user.

### Managing GIT
Git commit are safe at any point. Do not perform merge, or deploy without consent from the user. Even if previously asked to do so, always seek consent. Unless explicitly asked to do so by the user, a clear declaration from the user should be made before performing any Merge actions.

### Writing Docs
Match the length of written documents to what the task needs: cover the substance, but do not pad with filler sections, redundant summaries, or boilerplate.

### Scoping
Deliver what was asked, at the scope intended. Make routine judgment calls yourself, and check in only when different readings of the request would lead to materially different work. If the request seems mistaken or a better approach exists, say so in a sentence and continue with the task as asked rather than quietly narrowing, widening, or transforming it. Finish the whole task, and stop short of actions that are clearly beyond what was asked.

### When to delegate to Agents
Delegate to a subagent only for large tasks that are genuinely independent and parallelizable, such as a wide multi-file investigation. Do not delegate work you can finish yourself in a handful of tool calls, and do not use subagents to verify or double-check your own work. If one subagent can complete the task, use one rather than several, and keep spawn counts low.

### Live commentary of corrections
Only correct an earlier statement when the error would change the user's code, conclusions, or decisions. State corrections plainly and briefly, then continue the task. For slips that change nothing for the user, make the fix and move on without noting it.

### Tool calling issues
When you use a tool, you may say a brief sentence first. If no tool can express what the user asked for, say so instead of guessing. Do not include internal or system XML tags in your response.

## Graphify
This project has a knowledge graph at graphify-out/ with god nodes, community structure, and cross-file relationships.

**It is generated locally and not committed.** A fresh checkout has no graph until you run `graphify update .` — AST-only, no API cost, a few seconds. Every rule below is written to degrade gracefully when it is absent, so an ungenerated graph slows orientation rather than breaking anything. It is ignored rather than tracked for the same reason item sidecars and prompt snapshots are projections (R-DA-4): a committed copy of derived data is a second source of truth that can silently disagree with the code it describes.

Rules:
- For codebase questions, first run `graphify query "<question>"` when graphify-out/graph.json exists. Use `graphify path "<A>" "<B>"` for relationships and `graphify explain "<concept>"` for focused concepts. These return a scoped subgraph, usually much smaller than GRAPH_REPORT.md or raw grep output.
- If graphify-out/wiki/index.md exists, use it for broad navigation instead of raw source browsing.
- Read graphify-out/GRAPH_REPORT.md only for broad architecture review or when query/path/explain do not surface enough context.
- After modifying code, run `graphify update .` to keep the graph current (AST-only, no API cost).
