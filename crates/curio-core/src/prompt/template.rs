//! The gold-standard prompt template (FR-12).
//!
//! Seven sections: a heading the user can edit, and a paragraph carrying a hidden `section`
//! attribute. The attribute drives one thing — which placeholder an empty line shows.
//!
//! Every section is **deletable**, and every heading is **editable**. The template is a
//! scaffold for a good brief, not a form to fill in: a user with nothing to say about
//! `Guardrails — Never` deletes it, and a user who would rather call `Brief` something else
//! renames it.
//!
//! ## The body is written in, and shown again if it is taken out
//!
//! Each `body` is a real fragment of a real brief — the ACME notes-app landing page the
//! product was designed against — rather than a description of what belongs there. "What are
//! we building? One or two sentences." tells a user the shape of the answer; the sentence
//! itself shows the altitude, the specificity and the length that actually produce good work
//! from a model, which is the thing that is hard to guess and easy to demonstrate.
//!
//! It does **both jobs**: [`starter_document`] writes it into a new prompt as real content,
//! and the editor shows the same string as ghost text on any section the user later empties.
//! So the example is there to edit rather than to retype, and clearing a section brings back
//! the hint about what belonged in it instead of leaving a blank.
//!
//! This reverses an earlier reading of R-FE-17, which took "a new prompt starts empty" to
//! mean the template could only ever be a placeholder. That kept the examples permanently
//! out of reach: to work from one you had to retype it. A user who wants a blank page
//! selects all and deletes, which is one gesture and needs no setting.

use serde::{Deserialize, Serialize};

/// One template section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// The value stored in the paragraph's `section` attribute.
    pub id: &'static str,
    /// The heading written above the section in a new prompt. The user may rename it.
    pub heading: &'static str,
    /// The section's starting text.
    ///
    /// Written into a new prompt as real content, and shown again as ghost text if the user
    /// empties the section. One string, both jobs — see the module note.
    pub body: &'static str,
}

/// The seven sections, in the order they appear in a new prompt.
///
/// The order is the argument: what is being built, who it is for, what must and must not
/// happen, how it should look, the one thing that must not be missed, and what to hand
/// back. A model reads it top to bottom and the constraints arrive before the aesthetics.
///
/// Guardrails is two sections rather than a parent and two children, exactly as FR-12 names
/// it. "Always" and "Never" are the two lists a brief actually contains; a third heading
/// above them labels a grouping the user then has to write nothing under.
pub const SECTIONS: [Section; 7] = [
    Section {
        id: "brief",
        heading: "Brief",
        body: "Build a product landing page for \"ACME\" - a desktop application that writes \
               notes using AI. Goal - start free trial. Primary CTA: Download App, \"Free\" and \
               \"Download\" should be prominent hero and repeat at the end of the page.",
    },
    Section {
        id: "intent",
        heading: "Intent",
        body: "Target audience are executives who understand limitations of traditional note \
               taking app. An executive who struggles with keeping up with their schedule to to \
               fragmented schedules from calendar, email, slack immediately feels this is their \
               solution in 3 seconds.",
    },
    Section {
        id: "always",
        heading: "Guardrails — Always",
        body: "Always use /style-tag (add style tag) ,  /aesthetic-family -- add Aesthetic Family",
    },
    Section {
        id: "never",
        heading: "Guardrails — Never",
        body: "No gradients, glossy 3D saas blob. Do not use /design-type -- add design type or \
               /aesthetic-family",
    },
    Section {
        id: "direction",
        heading: "Design Direction",
        /*
         * The one multi-line section, and the reason the ghost rule renders line breaks.
         *
         * A direction is a *shape* — the family, then what narrows it, then the reference it
         * hangs off — and one line cannot show a shape. Two of them also demonstrate the
         * thing a single example cannot: that the directions are meant to be genuinely
         * different from each other rather than two phrasings of one idea.
         */
        body: concat!(
            "Direction 1 - /aesthetic-family specially /item-reference (attach library item)\n\n",
            "Direction 2 -  /aesthetic-family specially with  /style-tag (add style tag) ",
            "/item-reference (attach library item)",
        ),
    },
    Section {
        id: "important",
        heading: "Important",
        body: "Do NOT generate or source any imagery, for each section where you need to put an \
               image, add a placeholder with label with unique < Image#1 go here>. Fill it with \
               a flat CSS stand-in that matches the direction's palette. Size all typography \
               and negative space as if the described image were already there, so the real \
               image drops in with zero layout changes.",
    },
    Section {
        id: "output",
        heading: "Output",
        body: "Create 3 versions of this page, each in its own folder (v1/ … v3/), one per \
               direction in Design Direction. Same intent and guardrails for all three. Do NOT \
               blend directions — each version commits fully to its own aesthetic. Name each \
               direction, and record those names with their aesthetic family, design type and \
               tags in curio-variants.json at the project root, so the versions can be told \
               apart by something other than a folder number.",
    },
];

/// The heading for a section id, if it is one of ours.
#[must_use]
pub fn heading_for(section_id: &str) -> Option<&'static str> {
    SECTIONS
        .iter()
        .find(|section| section.id == section_id)
        .map(|section| section.heading)
}

/// The heading level the scaffold's section names are written at.
///
/// Level 2, so a section serializes to `## Brief` — which is what the old attribute-driven
/// synthesis emitted, so upgrading a library does not change the text an agent receives.
pub const SECTION_LEVEL: u64 = 2;

/// A new prompt, written out in full: seven named sections, each with its example in place.
///
/// Returned by `GET /api/prompts/template` and used as the `doc_json` of every new prompt,
/// so the server owns the template rather than each client re-deriving it.
///
/// **The bodies are content, not placeholders.** A new prompt opens on a worked brief the
/// user edits down to their own — the example is there to change rather than to retype, and
/// select-all-delete is the one gesture that gets a blank page. What it costs is that a
/// prompt copied without being touched hands the agent the ACME brief; what it buys is that
/// the common case, adapting the example, needs no transcription at all.
///
/// The headings are **real nodes** too, for the same reason: every one can be renamed,
/// reordered or deleted, and a prompt that ends up with nothing in common with this template
/// is a prompt the user wrote rather than one they fought. Nothing downstream depends on a
/// section still being present — the serializer emits what it finds, and an empty one is
/// dropped whether or not it was ever ours.
///
/// A body with line breaks becomes several paragraphs rather than one paragraph holding
/// newlines, because that is what the editor would have produced had the user typed it: a
/// `\n` inside a ProseMirror text node is not a line break, it is a character that renders as
/// a space and serializes as one. Only the **first** carries the `section` attribute, so
/// emptying the section shows its hint exactly once.
#[must_use]
pub fn starter_document() -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": SECTIONS
            .iter()
            .flat_map(|section| {
                let heading = serde_json::json!({
                    "type": "heading",
                    "attrs": { "level": SECTION_LEVEL },
                    "content": [{ "type": "text", "text": section.heading }],
                });

                let body = section.body.split('\n').enumerate().map(|(index, line)| {
                    let attrs = if index == 0 {
                        serde_json::json!({ "section": section.id })
                    } else {
                        serde_json::json!({})
                    };
                    // A blank line between two directions is a real empty paragraph; giving
                    // it a `content` array of one empty text node is invalid in TipTap's
                    // schema and the editor drops the whole document rather than the node.
                    if line.is_empty() {
                        serde_json::json!({ "type": "paragraph", "attrs": attrs })
                    } else {
                        serde_json::json!({
                            "type": "paragraph",
                            "attrs": attrs,
                            "content": [{ "type": "text", "text": line }],
                        })
                    }
                });

                std::iter::once(heading).chain(body)
            })
            .collect::<Vec<_>>(),
    })
}

/// Give a document written before the headings were real ones a heading per section.
///
/// Until now the scaffold was eight bare paragraphs carrying a `section` attribute, and the
/// serializer synthesized `## Brief` from that attribute. Both halves of that arrangement
/// are gone: the heading is a node the user can edit, and the serializer emits only what it
/// finds. A document still in the old shape would therefore lose every section heading from
/// its output — silently, and in the text the user pastes into an agent.
///
/// Returns `None` when there is nothing to do, so the caller can leave the row untouched.
/// The check is per paragraph, not per document: a half-migrated document — one a user
/// edited between builds — gets exactly the headings it is missing.
#[must_use]
pub fn upgrade_document(doc: &serde_json::Value) -> Option<serde_json::Value> {
    let blocks = doc["content"].as_array()?;
    let mut upgraded: Vec<serde_json::Value> = Vec::with_capacity(blocks.len());
    let mut changed = false;

    for (index, block) in blocks.iter().enumerate() {
        let heading = block["attrs"]["section"]
            .as_str()
            .filter(|_| block["type"] == "paragraph")
            .and_then(heading_for);

        // Already introduced by a heading — whatever it says. A user who renamed a section
        // before upgrading must not have Curio's word inserted above their own.
        let introduced = index > 0 && blocks[index - 1]["type"] == "heading";

        if let Some(heading) = heading
            && !introduced
        {
            upgraded.push(serde_json::json!({
                "type": "heading",
                "attrs": { "level": SECTION_LEVEL },
                "content": [{ "type": "text", "text": heading }],
            }));
            changed = true;
        }

        upgraded.push(block.clone());
    }

    changed.then(|| serde_json::json!({ "type": "doc", "content": upgraded }))
}

/// Whether `text` is one of the template's own section names.
///
/// Used to tell a heading the scaffold wrote from one the user did. The comparison is on the
/// trimmed, case-folded text: a user who retypes "brief" has not renamed anything, and a
/// user who writes "Brief for the agent" has.
#[must_use]
pub fn is_section_heading(text: &str) -> bool {
    let needle = text.trim().to_lowercase();
    SECTIONS
        .iter()
        .any(|section| section.heading.to_lowercase() == needle)
}

/// Whether `text` is a line the template wrote, untouched.
///
/// The counterpart to [`is_section_heading`], and it exists for the same one reason: naming a
/// prompt after its first line must not name every new prompt after the template's Brief.
/// Matching per line rather than per section because a multi-line body becomes one paragraph
/// per line, and the title is derived from a single block.
/// Runs of whitespace collapse before comparing, because the caller has already collapsed
/// its side: a title is one line, and the body it is being matched against contains double
/// spaces the author typed. Comparing raw against collapsed never matches, which showed up as
/// a new prompt titling itself after the first section whose text happened to be short.
#[must_use]
pub fn is_section_body(text: &str) -> bool {
    let needle = squash(text);
    !needle.is_empty()
        && SECTIONS
            .iter()
            .flat_map(|section| section.body.split('\n'))
            .any(|line| squash(line) == needle)
}

fn squash(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// The title a prompt starts with, before the user names it.
pub const UNTITLED: &str = "Untitled prompt";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_carries_seven_sections() {
        // FR-12 names them; the count is asserted so adding an eighth is a deliberate act
        // that updates the requirement rather than a quiet drift.
        //
        // It was eight until now, and that was the drift this test exists to catch: a
        // standalone "Guardrails" heading had been added above Always and Never, where FR-12
        // names "Guardrails Always/Never" as two sections and no parent.
        assert_eq!(SECTIONS.len(), 7);
    }

    #[test]
    fn every_section_body_is_written_prose_rather_than_an_instruction_to_write_some() {
        // The template's whole teaching value. "What are we building? One or two sentences"
        // describes the answer; the body *is* the answer, so a user adapts it instead of
        // starting from a description of what they were meant to say.
        //
        // Asserted by shape rather than by length: two of these are one-line rules, and an
        // earlier version of this test demanded 120 characters and would have rejected them.
        for section in SECTIONS {
            let body = section.body;
            assert!(!body.trim().is_empty(), "{}", section.id);
            assert!(
                !body.ends_with('?'),
                "{} asks the user a question instead of showing them an answer: {body}",
                section.id
            );
            assert!(
                body.split_whitespace().count() >= 8,
                "{} is too short to be an example: {body}",
                section.id
            );
        }
    }

    #[test]
    fn the_design_direction_body_keeps_its_line_breaks() {
        // A direction is a shape — the family, then what narrows it, then the reference — and
        // the breaks are what show it. `starter_document` turns each line into its own
        // paragraph, and the editor's ghost rule renders the breaks with `white-space:
        // pre-line` for when the section is emptied. If this text ever collapses to one line,
        // both of those went with it.
        let direction = SECTIONS
            .iter()
            .find(|section| section.id == "direction")
            .expect("the direction section");

        assert!(direction.body.contains('\n'));
        assert!(direction.body.contains("Direction 1"));
        assert!(direction.body.contains("Direction 2"));
        // A blank line between directions, so two examples read as two.
        assert!(direction.body.contains("\n\n"));
    }

    #[test]
    fn section_ids_are_unique() {
        let ids: std::collections::HashSet<&str> = SECTIONS.iter().map(|s| s.id).collect();
        assert_eq!(ids.len(), SECTIONS.len());
    }

    #[test]
    fn the_documented_order_is_the_stored_order() {
        // "Brief → Intent → Guardrails Always/Never → Design Direction → Important →
        // Output" (FR-12). Constraints arrive before aesthetics, and a model reads the
        // document top to bottom.
        let ids: Vec<&str> = SECTIONS.iter().map(|s| s.id).collect();
        assert_eq!(
            ids,
            [
                "brief",
                "intent",
                "always",
                "never",
                "direction",
                "important",
                "output"
            ]
        );

        // The two guardrail sections carry the grouping in their own names, which is what
        // lets them stand alone in the document without a parent heading over them.
        let headings: Vec<&str> = SECTIONS.iter().map(|s| s.heading).collect();
        assert!(headings.contains(&"Guardrails — Always"));
        assert!(headings.contains(&"Guardrails — Never"));
    }

    #[test]
    fn every_section_has_a_heading_and_a_ghost() {
        for section in SECTIONS {
            assert!(!section.heading.is_empty(), "{}", section.id);
            assert!(!section.body.is_empty(), "{}", section.id);
        }
    }

    #[test]
    fn an_unknown_section_has_no_heading() {
        assert_eq!(heading_for("brief"), Some("Brief"));
        assert!(heading_for("epilogue").is_none());
    }

    #[test]
    fn a_new_document_opens_with_the_template_already_written_in() {
        // The change this test exists for: a new prompt used to be seven headings over seven
        // *empty* lines, with the examples visible only as ghost text you could not edit. The
        // body is content now — the user adapts what is there instead of retyping it.
        let doc = starter_document();

        assert_eq!(doc["type"], "doc");
        let content = doc["content"].as_array().expect("content");

        assert_eq!(content[0]["type"], "heading");
        assert_eq!(content[0]["attrs"]["level"], SECTION_LEVEL);
        assert_eq!(content[0]["content"][0]["text"], "Brief");

        assert_eq!(content[1]["type"], "paragraph");
        assert_eq!(content[1]["attrs"]["section"], "brief");
        assert_eq!(content[1]["content"][0]["text"], SECTIONS[0].body);
    }

    #[test]
    fn a_multi_line_body_becomes_one_paragraph_per_line() {
        // A `\n` inside a ProseMirror text node is not a line break — it renders as a space
        // and serializes as one — so the two directions would run together in the editor and
        // in the prompt. Only the first paragraph carries the `section` attribute, or an
        // emptied section would ghost its hint three times.
        let doc = starter_document();
        let content = doc["content"].as_array().expect("content");

        let start = content
            .iter()
            .position(|node| node["attrs"]["section"] == "direction")
            .expect("the direction section");
        let lines: Vec<&str> = SECTIONS
            .iter()
            .find(|section| section.id == "direction")
            .expect("direction")
            .body
            .split('\n')
            .collect();

        assert_eq!(lines.len(), 3, "two directions and the blank line between");
        for (offset, line) in lines.iter().enumerate() {
            let node = &content[start + offset];
            assert_eq!(node["type"], "paragraph");
            if line.is_empty() {
                // A blank line is an empty paragraph, not a paragraph holding "".
                assert!(node["content"].is_null(), "{node}");
            } else {
                assert_eq!(node["content"][0]["text"], *line);
            }
        }

        assert_eq!(content[start]["attrs"]["section"], "direction");
        assert!(content[start + 1]["attrs"]["section"].is_null());
        assert!(content[start + 2]["attrs"]["section"].is_null());
    }

    #[test]
    fn the_section_names_are_real_editable_content() {
        // The distinction this test exists to protect: the headings used to be drawn over
        // the document by the editor, which meant the user could not put the caret in one.
        // They are nodes now, so every one of them can be renamed or deleted.
        let doc = starter_document();
        let content = doc["content"].as_array().expect("content");

        let headings: Vec<&str> = content
            .iter()
            .filter(|node| node["type"] == "heading")
            .map(|node| node["content"][0]["text"].as_str().unwrap_or_default())
            .collect();

        assert_eq!(
            headings,
            SECTIONS.iter().map(|s| s.heading).collect::<Vec<_>>()
        );
    }

    #[test]
    fn an_old_document_gains_the_headings_its_sections_lost() {
        let old = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "paragraph", "attrs": { "section": "brief" },
                  "content": [{ "type": "text", "text": "A pricing page" }] },
                { "type": "paragraph", "attrs": { "section": "output" } },
            ],
        });

        let upgraded = upgrade_document(&old).expect("an old document needs upgrading");
        let content = upgraded["content"].as_array().expect("content");

        assert_eq!(content.len(), 4);
        assert_eq!(content[0]["type"], "heading");
        assert_eq!(content[0]["content"][0]["text"], "Brief");
        assert_eq!(content[1]["content"][0]["text"], "A pricing page");
        assert_eq!(content[2]["content"][0]["text"], "Output");
    }

    #[test]
    fn a_current_document_is_left_alone() {
        // Idempotence. The migration runs once, but `None` here is what guarantees a re-run
        // against a hand-patched library cannot double every heading.
        assert!(upgrade_document(&starter_document()).is_none());
    }

    #[test]
    fn a_section_the_user_already_renamed_keeps_their_name() {
        // The failure this prevents: inserting "Brief" above a heading that says "Context",
        // leaving the user with two headings where they wrote one.
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 },
                  "content": [{ "type": "text", "text": "Context" }] },
                { "type": "paragraph", "attrs": { "section": "brief" },
                  "content": [{ "type": "text", "text": "A pricing page" }] },
            ],
        });

        assert!(upgrade_document(&doc).is_none());
    }

    #[test]
    fn a_half_migrated_document_gets_only_what_it_is_missing() {
        // A user who edited between builds: one section already headed, one not.
        let doc = serde_json::json!({
            "type": "doc",
            "content": [
                { "type": "heading", "attrs": { "level": 2 },
                  "content": [{ "type": "text", "text": "Brief" }] },
                { "type": "paragraph", "attrs": { "section": "brief" },
                  "content": [{ "type": "text", "text": "A pricing page" }] },
                { "type": "paragraph", "attrs": { "section": "never" },
                  "content": [{ "type": "text", "text": "No gradients" }] },
            ],
        });

        let content = upgrade_document(&doc).expect("needs upgrading");
        let blocks = content["content"].as_array().expect("content");

        // One insertion, not two: "Brief" already had its heading and must not gain a second.
        assert_eq!(blocks.len(), 4);
        assert_eq!(blocks[0]["content"][0]["text"], "Brief");
        assert_eq!(blocks[1]["content"][0]["text"], "A pricing page");
        assert_eq!(blocks[2]["type"], "heading");
        assert_eq!(blocks[2]["content"][0]["text"], "Guardrails — Never");
        assert_eq!(blocks[3]["content"][0]["text"], "No gradients");
    }

    #[test]
    fn a_paragraph_with_no_section_is_not_given_a_heading() {
        // Ordinary prose the user typed. Only the scaffold's own sections are restored.
        let doc = serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "content": [{ "type": "text", "text": "Free" }] }],
        });

        assert!(upgrade_document(&doc).is_none());
    }

    #[test]
    fn an_upgraded_document_serializes_the_same_as_it_used_to() {
        // The whole point of the migration. The old serializer turned a sectioned paragraph
        // into "## Brief" plus its text; the new one emits the heading node the migration
        // inserts. A user's prompt must read identically across the upgrade.
        let old = serde_json::json!({
            "type": "doc",
            "content": [{ "type": "paragraph", "attrs": { "section": "brief" },
                          "content": [{ "type": "text", "text": "A pricing page." }] }],
        });

        let upgraded = upgrade_document(&old).expect("needs upgrading");
        let text = super::super::serialize(&upgraded, &super::super::ChipContext::default());

        // The document half, not the whole output: every prompt now ends with the write-back
        // footer, which is appended after the user's content and is not what this migration
        // is about.
        assert!(
            text.starts_with("## Brief\n\nA pricing page."),
            "the upgrade changed how a prompt reads: {text}"
        );
    }

    #[test]
    fn a_template_name_is_recognised_whatever_its_case_or_padding() {
        assert!(is_section_heading("Brief"));
        assert!(is_section_heading("  design direction  "));
        assert!(!is_section_heading("Context for the agent"));
        assert!(!is_section_heading(""));
    }

    #[test]
    fn every_body_is_in_the_document() {
        // The inverse of what this asserted until now. The bodies used to be kept *out* of
        // the document on the reasoning that storing them would put Curio's words in the
        // user's brief — true, and now the point: a new prompt is a worked brief to edit
        // down, and a user who wants none of it selects all and deletes.
        let doc = starter_document();
        let text = serde_json::to_string(&doc).expect("serialize");

        for section in SECTIONS {
            for line in section.body.split('\n').filter(|line| !line.is_empty()) {
                let escaped = serde_json::to_string(line).expect("escape");
                let quoted = escaped.trim_matches('"');
                assert!(text.contains(quoted), "{} is missing: {line}", section.id);
            }
        }
    }

    #[test]
    fn a_body_line_is_recognised_as_the_templates_own() {
        // What keeps a new prompt out of the list titled "Build a product landing page…".
        assert!(is_section_body(SECTIONS[0].body));
        // Whitespace is not an edit: the guardrail bodies carry double spaces the author
        // typed, and the caller compares an already-collapsed single line.
        assert!(is_section_body(
            "Always use /style-tag (add style tag) , /aesthetic-family -- add Aesthetic Family"
        ));
        // One line of a multi-line body counts, because each is its own paragraph.
        assert!(is_section_body(
            "Direction 1 - /aesthetic-family specially /item-reference (attach library item)"
        ));
        // And anything the user wrote does not.
        assert!(!is_section_body("Build a landing page for my own product"));
        assert!(!is_section_body(""));
    }
}
