# The gold-standard prompt template

What every new prompt opens with (FR-12): seven sections, each a heading and a body. This
file is a **reference copy for reading and drafting wording**. Nothing loads it.

The source of truth is `SECTIONS` in
[`crates/curio-core/src/prompt/template.rs`](../crates/curio-core/src/prompt/template.rs).
Changing the template means changing that const and rebuilding; changing this file changes
nothing. Both are listed here so a wording pass can happen in prose and land in one edit —
if the two ever disagree, the Rust is right.

## The template is written in, not offered

A new prompt arrives with all of this already in the document. The user edits it down to
their own brief; the example is there to change rather than to retype, and anyone who wants
a blank page selects all and deletes.

That is a reversal. The bodies used to be ghost text only — visible but untouchable, so
working from one meant transcribing it. The cost of the change is that a prompt copied
without being touched hands the agent the ACME brief verbatim; the benefit is that the
common case, adapting the example, needs no typing at all.

The ghost text has not gone. Empty a section and its body reappears as a hint, so clearing
one says what belonged there instead of leaving a blank.

## How the two fields are used

A section's **heading** is written into the document as a real `h2` node. It is ordinary
content: rename it, reorder it, delete it. Whatever it says at serialization time is what
the agent receives.

A section's **body** is written in as paragraphs — one per line, because a `\n` inside a
ProseMirror text node is not a line break but a character that renders and serializes as a
space. Only the first paragraph carries the hidden `section` attribute, so an emptied
section ghosts its hint exactly once.

Neither is special-cased downstream. The serializer emits what it finds and drops any
heading with nothing under it, whether Curio wrote it or the user did.

## Naming

A prompt is named after its own first line, derived server-side on every save. Text the
template wrote is skipped — headings and bodies alike — or every new prompt in the list
would be called "Build a product landing page for ACME". Editing any line makes it eligible,
so adapting the Brief or retitling the first section both name the prompt without anyone
thinking about titles.

## The sections

### Brief

```text
Build a product landing page for "ACME" - a desktop application that writes notes using AI. Goal - start free trial. Primary CTA: Download App, "Free" and "Download" should be prominent hero and repeat at the end of the page.
```

### Intent

```text
Target audience are executives who understand limitations of traditional note taking app. An executive who struggles with keeping up with their schedule to to fragmented schedules from calendar, email, slack immediately feels this is their solution in 3 seconds.
```

### Guardrails — Always

```text
Always use /style-tag (add style tag) ,  /aesthetic-family -- add Aesthetic Family
```

### Guardrails — Never

```text
No gradients, glossy 3D saas blob. Do not use /design-type -- add design type or /aesthetic-family
```

### Design Direction

```text
Direction 1 - /aesthetic-family specially /item-reference (attach library item)

Direction 2 -  /aesthetic-family specially with  /style-tag (add style tag) /item-reference (attach library item)
```

### Important

```text
Do NOT generate or source any imagery, for each section where you need to put an image, add a placeholder with label with unique < Image#1 go here>. Fill it with a flat CSS stand-in that matches the direction's palette. Size all typography and negative space as if the described image were already there, so the real image drops in with zero layout changes.
```

### Output

```text
Create 3 versions of this page, each in its own folder (v1/ … v3/), one per direction in Design Direction. Same intent and guardrails for all three. Do NOT blend directions — each version commits fully to its own aesthetic. Name each direction, and record those names with their aesthetic family, design type and tags in curio-variants.json at the project root, so the versions can be told apart by something other than a folder number.
```

## The variant manifest

`v1/`, `v2/` and `v3/` tell a user nothing about what is inside them. When Curio serves a
project it overlays a bar for moving between the versions, and this file is where that bar
gets its labels — the same thing an agent usually writes into a README table, in a shape
Curio can read.

It is `curio-variants.json` at the **project root**, beside the version folders. Not a
dotfile: the names in it are authored content a user reads on screen and corrects when a
model gets one wrong, and a file that is hidden in Explorer is a file nobody can fix.

```json
{
  "version": 1,
  "variants": [
    {
      "folder": "v1",
      "name": "Print-tech",
      "summary": "Pale sage ground, muted coral accent, Archivo + IBM Plex Mono.",
      "family": "Editorial Print",
      "design_type": "Landing page",
      "tags": ["risograph", "monospace"]
    }
  ]
}
```

Only `folder` is required, and it must match the directory name exactly. `family`,
`design_type` and `tags` read best when they name terms the library already has — an agent
with an MCP connection should call `library_list_vocabulary` first; one without can take them
from the prompt's own chips.

Rules Curio applies when reading it, all of which favour showing something over showing
nothing:

- The manifest may only ever **enrich** what is on disk. A folder that exists is listed even
  when nothing describes it, and an entry naming a folder that does not exist cannot conjure
  one — a stale file must not offer a link to a 404.
- Unknown keys are ignored rather than rejected, so a helpful extra field costs nothing.
- A file that will not parse is reported in the bar and the versions are still listed. Losing
  navigation to a trailing comma would be worse than the trailing comma.

**Curio never writes this file itself.** An agent writes it directly, or asks Curio to by
passing `variants` to the `project_register` MCP tool — which writes exactly this shape.

