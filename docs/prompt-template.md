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
Create 3 versions of this page, each in its own folder (v1/ … v3/), one per direction in Design Direction. Same intent and guardrails for all three. Do NOT blend directions — each version commits fully to its own aesthetic.
```

