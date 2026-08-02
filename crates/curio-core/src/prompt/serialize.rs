//! Turning a TipTap document into the text a user pastes into their AI tool.
//!
//! This runs **server-side and is authoritative** (R-FE-18, Inventory §6). The dashboard
//! renders friendly chips and never learns what they expand to, which is what keeps one
//! definition of a serialized prompt rather than one per client: the MCP `prompt_get`
//! tool, the clipboard, and the `prompts/{id}.md` snapshot all come through here.
//!
//! ## Why chips expand to paths
//!
//! An `itemRef` chip becomes an absolute directory path plus an instruction to read what
//! is in it. That is the zero-integration handoff the product rests on (FR-14): an agent
//! with no MCP connection, no API, and no knowledge of Curio can still follow the prompt,
//! because the prompt names files on a disk it can already read.

use std::collections::HashMap;

/// What chips resolve against.
///
/// The server fills these from the database before serializing. Passing resolved data
/// rather than a database handle is what keeps this crate free of SQL (R-DEL-2) and makes
/// every rule below testable with a literal map.
#[derive(Debug, Default)]
pub struct ChipContext {
    /// Family id → (name, description).
    pub families: HashMap<String, (String, String)>,
    /// Item id → (name, **absolute** directory).
    pub items: HashMap<String, (String, String)>,
    /// Tag or design-type id → name.
    pub terms: HashMap<String, String>,
}

/// Serialize a TipTap document (Inventory §6, §10.22).
///
/// Unknown node types are traversed rather than dropped: a document written by a newer
/// editor must degrade to its text, not to a hole.
///
/// ## An empty section is dropped, heading and all
///
/// The template opens eight named sections and a user fills in three. The other five must
/// not reach the model: a heading with nothing under it is an instruction to interpret an
/// absence, and eight of them is a form the agent will try to answer.
///
/// The test is **what follows the heading**, not what the heading says. Section headings
/// used to be synthesized here from a `section` attribute, which meant only Curio's own
/// eight could be dropped; they are real, editable nodes now, so this rule applies equally
/// to a heading the user wrote and left empty (R-FE-18, FR-12).
#[must_use]
pub fn serialize(doc: &serde_json::Value, context: &ChipContext) -> String {
    let mut out = String::new();
    let blocks: Vec<&serde_json::Value> = doc["content"].as_array().into_iter().flatten().collect();

    for (index, block) in blocks.iter().enumerate() {
        if block["type"] == "heading" && !section_has_content(&blocks[index + 1..], context) {
            continue;
        }
        write_block(&mut out, block, context, 0);
    }

    collapse_blank_runs(out.trim())
}

/// Whether anything before the next heading writes text.
///
/// A horizontal rule counts as content on purpose — a user who put a divider under a heading
/// meant the section to be there.
fn section_has_content(rest: &[&serde_json::Value], context: &ChipContext) -> bool {
    rest.iter()
        .take_while(|block| block["type"] != "heading")
        .any(|block| {
            block["type"] == "horizontalRule" || {
                let mut text = String::new();
                write_block(&mut text, block, context, 0);
                !text.trim().is_empty()
            }
        })
}

fn write_block(out: &mut String, node: &serde_json::Value, context: &ChipContext, depth: usize) {
    match node["type"].as_str().unwrap_or_default() {
        "paragraph" => {
            let text = inline_text(node, context);
            // An untouched section contributes nothing, and the heading above it was already
            // skipped by `serialize`.
            if text.trim().is_empty() {
                return;
            }
            push_block(out, &text);
        }

        "heading" => {
            let level = node["attrs"]["level"].as_u64().unwrap_or(1).clamp(1, 6) as usize;
            let text = inline_text(node, context);
            if !text.trim().is_empty() {
                push_block(out, &format!("{} {text}", "#".repeat(level)));
            }
        }

        "bulletList" | "orderedList" => {
            let ordered = node["type"] == "orderedList";
            for (index, item) in node["content"].as_array().into_iter().flatten().enumerate() {
                let marker = if ordered {
                    format!("{}. ", index + 1)
                } else {
                    "- ".to_owned()
                };
                let body = list_item_text(item, context);
                if !body.trim().is_empty() {
                    out.push_str(&"  ".repeat(depth));
                    out.push_str(&marker);
                    out.push_str(&body);
                    out.push('\n');
                }
            }
            out.push('\n');
        }

        "blockquote" => {
            for child in node["content"].as_array().into_iter().flatten() {
                let text = inline_text(child, context);
                if !text.trim().is_empty() {
                    push_block(out, &format!("> {text}"));
                }
            }
        }

        "codeBlock" => {
            let text = inline_text(node, context);
            push_block(out, &format!("```\n{text}\n```"));
        }

        "horizontalRule" => push_block(out, "---"),

        // A node from a newer editor build. Walk into it: its text is the user's writing,
        // and dropping it silently would lose work rather than lose formatting.
        _ => {
            for child in node["content"].as_array().into_iter().flatten() {
                write_block(out, child, context, depth);
            }
        }
    }
}

fn list_item_text(item: &serde_json::Value, context: &ChipContext) -> String {
    item["content"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|child| inline_text(child, context))
        .collect::<Vec<_>>()
        .join(" ")
        .trim()
        .to_owned()
}

fn inline_text(node: &serde_json::Value, context: &ChipContext) -> String {
    let mut out = String::new();
    for child in node["content"].as_array().into_iter().flatten() {
        match child["type"].as_str().unwrap_or_default() {
            "text" => out.push_str(&marked_text(child)),
            "hardBreak" => out.push('\n'),
            "familyChip" | "tagChip" | "typeChip" | "itemRef" => {
                out.push_str(&expand_chip(child, context));
            }
            _ => out.push_str(&inline_text(child, context)),
        }
    }
    out
}

/// Apply the marks a run of text carries.
///
/// Only the marks that survive a round trip through plain text are rendered. A prompt is
/// pasted into a terminal or a chat box as often as into a markdown renderer, so a mark
/// with no textual meaning is noise in the majority case.
fn marked_text(node: &serde_json::Value) -> String {
    let text = node["text"].as_str().unwrap_or_default();
    let mut rendered = text.to_owned();
    for mark in node["marks"].as_array().into_iter().flatten() {
        rendered = match mark["type"].as_str().unwrap_or_default() {
            "bold" => format!("**{rendered}**"),
            "italic" => format!("*{rendered}*"),
            "code" => format!("`{rendered}`"),
            _ => rendered,
        };
    }
    rendered
}

/// Expand one chip (Inventory §6).
///
/// Every branch falls back to the chip's stored `label` when the referenced row is gone
/// (R-FE-17). A prompt written six months ago that cites a since-deleted item must still
/// read as a sentence — losing the path is a degradation; losing the word is a hole in the
/// middle of the user's brief.
fn expand_chip(chip: &serde_json::Value, context: &ChipContext) -> String {
    let id = chip["attrs"]["id"].as_str().unwrap_or_default();
    let label = chip["attrs"]["label"].as_str().unwrap_or_default();

    match chip["type"].as_str().unwrap_or_default() {
        "familyChip" => match context.families.get(id) {
            Some((name, description)) => format!("Aesthetic: {name} — {description}"),
            None if label.is_empty() => String::new(),
            None => format!("Aesthetic: {label}"),
        },

        "itemRef" => match context.items.get(id) {
            Some((name, directory)) => format!(
                "Reference: {name} — {directory} — read screenshot.png and item.md before \
                 designing; match feel, not content."
            ),
            None if label.is_empty() => String::new(),
            None => format!("Reference: {label}"),
        },

        // Tags and design types carry no extra payload — the word is the whole point.
        _ => context
            .terms
            .get(id)
            .map_or_else(|| label.to_owned(), Clone::clone),
    }
}

fn push_block(out: &mut String, block: &str) {
    out.push_str(block);
    out.push_str("\n\n");
}

/// Collapse three or more consecutive newlines into two (Inventory §6).
///
/// Deleted ghost sections and empty paragraphs leave gaps behind. Left alone they
/// accumulate into pages of whitespace inside a prompt whose whole job is to be pasted
/// somewhere and read.
fn collapse_blank_runs(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut newlines = 0usize;
    for character in text.chars() {
        if character == '\n' {
            newlines += 1;
            if newlines > 2 {
                continue;
            }
        } else {
            newlines = 0;
        }
        out.push(character);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::template;
    use super::*;

    fn context() -> ChipContext {
        let mut context = ChipContext::default();
        context.families.insert(
            "fam1".to_owned(),
            (
                "Warm Editorial".to_owned(),
                "Serif headlines on paper-warm neutrals".to_owned(),
            ),
        );
        context.items.insert(
            "item1".to_owned(),
            (
                "Stripe pricing".to_owned(),
                "C:\\Users\\me\\Curio\\items\\01J".to_owned(),
            ),
        );
        context
            .terms
            .insert("tag1".to_owned(), "brutalist".to_owned());
        context
    }

    fn paragraph(section: &str, content: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "type": "paragraph", "attrs": { "section": section }, "content": content })
    }

    fn text(value: &str) -> serde_json::Value {
        serde_json::json!({ "type": "text", "text": value })
    }

    fn doc(content: Vec<serde_json::Value>) -> serde_json::Value {
        serde_json::json!({ "type": "doc", "content": content })
    }

    fn heading(level: u64, value: &str) -> serde_json::Value {
        serde_json::json!({
            "type": "heading",
            "attrs": { "level": level },
            "content": [text(value)],
        })
    }

    #[test]
    fn a_section_with_content_keeps_its_heading() {
        // The heading is a node in the document now, not something synthesized here from a
        // `section` attribute — so what comes out is what the user can see and edit.
        let output = serialize(
            &doc(vec![
                heading(2, "Brief"),
                paragraph("brief", serde_json::json!([text("A pricing page.")])),
            ]),
            &context(),
        );

        assert_eq!(output, "## Brief\n\nA pricing page.");
    }

    #[test]
    fn a_renamed_heading_is_carried_verbatim() {
        // The point of making the scaffold real: a user who retitles a section gets their
        // own word in the prompt, not Curio's.
        let output = serialize(
            &doc(vec![
                heading(2, "Context for the agent"),
                paragraph("brief", serde_json::json!([text("A pricing page.")])),
            ]),
            &context(),
        );

        assert_eq!(output, "## Context for the agent\n\nA pricing page.");
    }

    #[test]
    fn an_untouched_section_disappears_heading_and_all() {
        // FR-12's sections are deletable, and an untouched one is functionally deleted. A
        // heading with nothing under it is an empty instruction for the model to interpret,
        // and a fresh template holds eight of them.
        let output = serialize(
            &doc(vec![
                heading(2, "Brief"),
                paragraph("brief", serde_json::json!([text("A pricing page.")])),
                heading(2, "Never"),
                paragraph("never", serde_json::json!([])),
            ]),
            &context(),
        );

        assert_eq!(output, "## Brief\n\nA pricing page.");
    }

    #[test]
    fn an_empty_heading_the_user_wrote_is_dropped_too() {
        // The rule is about what follows a heading, not about whether Curio wrote it. A
        // section the user added and left blank is as empty as one of ours.
        let output = serialize(
            &doc(vec![
                heading(3, "My own empty section"),
                heading(2, "Brief"),
                paragraph("brief", serde_json::json!([text("A pricing page.")])),
            ]),
            &context(),
        );

        assert_eq!(output, "## Brief\n\nA pricing page.");
    }

    #[test]
    fn a_heading_followed_by_a_list_survives() {
        // "Content" is not only paragraphs. A section whose whole body is a list was being
        // read as empty by an earlier draft of this rule.
        let output = serialize(
            &doc(vec![
                heading(2, "Always"),
                serde_json::json!({
                    "type": "bulletList",
                    "content": [{
                        "type": "listItem",
                        "content": [paragraph("", serde_json::json!([text("Ship it")]))],
                    }],
                }),
            ]),
            &context(),
        );

        assert!(output.contains("## Always"), "{output}");
        assert!(output.contains("- Ship it"), "{output}");
    }

    #[test]
    fn a_fresh_template_serializes_to_the_whole_worked_brief() {
        // The consequence of the template being content rather than ghost text, stated so it
        // is a decision rather than a surprise: copying an untouched new prompt hands the
        // agent the ACME brief in full. That is the trade — the example is there to edit
        // instead of retype, and a user who wants none of it selects all and deletes.
        let text = serialize(&template::starter_document(), &context());

        for section in template::SECTIONS {
            assert!(
                text.contains(&format!("## {}", section.heading)),
                "{} lost its heading",
                section.id
            );
        }
        assert!(
            text.contains("Build a product landing page for \"ACME\""),
            "{text}"
        );
        // Each direction on its own line, not run together by a stray `\n` in a text node.
        assert!(text.contains("Direction 1 - "), "{text}");
        assert!(text.contains("Direction 2 - "), "{text}");
    }

    #[test]
    fn a_section_the_user_emptied_still_drops() {
        // The rule that used to be exercised by the fresh template, which no longer has any
        // empty sections. Clearing a section is now how one becomes empty, and it must still
        // take its heading with it rather than leave a bare instruction.
        let output = serialize(
            &doc(vec![
                heading(2, "Brief"),
                paragraph("brief", serde_json::json!([text("A pricing page.")])),
                heading(2, "Guardrails — Never"),
                paragraph("never", serde_json::json!([])),
            ]),
            &context(),
        );

        assert_eq!(output, "## Brief\n\nA pricing page.");
    }

    #[test]
    fn a_family_chip_carries_its_description() {
        // FR-14: the whole value of a family chip is that the model receives the library's
        // own definition of the look, not just its name.
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([{ "type": "familyChip", "attrs": { "id": "fam1", "label": "Warm Editorial" } }]),
            )]),
            &context(),
        );

        assert!(
            output.contains("Aesthetic: Warm Editorial — Serif headlines"),
            "{output}"
        );
    }

    #[test]
    fn an_item_chip_becomes_an_absolute_path_with_reading_instructions() {
        // The zero-integration handoff (FR-14): an agent that has never heard of Curio can
        // still follow this, because it names files on a disk it can already read.
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([{ "type": "itemRef", "attrs": { "id": "item1", "label": "Stripe pricing" } }]),
            )]),
            &context(),
        );

        assert!(
            output.contains("C:\\Users\\me\\Curio\\items\\01J"),
            "{output}"
        );
        assert!(
            output.contains("read screenshot.png and item.md"),
            "{output}"
        );
        assert!(output.contains("match feel, not content"), "{output}");
    }

    #[test]
    fn a_chip_whose_row_is_gone_falls_back_to_its_label() {
        // R-FE-17. A prompt citing a since-deleted item must still read as a sentence.
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([{ "type": "itemRef", "attrs": { "id": "deleted", "label": "Old capture" } }]),
            )]),
            &context(),
        );

        assert!(output.contains("Reference: Old capture"), "{output}");
    }

    #[test]
    fn tag_and_type_chips_serialize_to_their_name_alone() {
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([
                    text("Feels "),
                    { "type": "tagChip", "attrs": { "id": "tag1", "label": "brutalist" } },
                    text(".")
                ]),
            )]),
            &context(),
        );

        assert!(output.contains("Feels brutalist."), "{output}");
    }

    #[test]
    fn runs_of_blank_lines_collapse() {
        let output = serialize(
            &doc(vec![
                paragraph("brief", serde_json::json!([text("One.")])),
                serde_json::json!({ "type": "paragraph", "content": [] }),
                serde_json::json!({ "type": "paragraph", "content": [] }),
                paragraph("output", serde_json::json!([text("Two.")])),
            ]),
            &context(),
        );

        assert!(!output.contains("\n\n\n"), "{output:?}");
    }

    #[test]
    fn lists_survive_as_lists() {
        let output = serialize(
            &doc(vec![serde_json::json!({
                "type": "bulletList",
                "content": [
                    { "type": "listItem", "content": [{ "type": "paragraph", "content": [text("Keyboard reachable")] }] },
                    { "type": "listItem", "content": [{ "type": "paragraph", "content": [text("No modal traps")] }] }
                ]
            })]),
            &context(),
        );

        assert!(output.contains("- Keyboard reachable"), "{output}");
        assert!(output.contains("- No modal traps"), "{output}");
    }

    #[test]
    fn marks_that_mean_something_in_plain_text_survive() {
        let output = serialize(
            &doc(vec![paragraph(
                "important",
                serde_json::json!([{ "type": "text", "text": "never", "marks": [{ "type": "bold" }] }]),
            )]),
            &context(),
        );

        assert!(output.contains("**never**"), "{output}");
    }

    #[test]
    fn an_unknown_node_keeps_its_text() {
        // A document written by a newer editor build must degrade to its words, not to a
        // hole in the middle of the user's brief.
        let output = serialize(
            &doc(vec![serde_json::json!({
                "type": "somethingNew",
                "content": [{ "type": "paragraph", "content": [text("Still mine.")] }]
            })]),
            &context(),
        );

        assert!(output.contains("Still mine."), "{output}");
    }

    #[test]
    fn an_empty_document_serializes_to_nothing() {
        // A genuinely empty document — what a user has after select-all-delete. This used to
        // pass the starter template, which no longer is one.
        assert_eq!(serialize(&doc(vec![]), &context()), "");
    }

    #[test]
    fn the_output_has_no_leading_or_trailing_whitespace() {
        // It goes straight onto the clipboard. A leading blank line is the first thing the
        // user pastes.
        let output = serialize(
            &doc(vec![paragraph(
                "brief",
                serde_json::json!([text("A pricing page.")]),
            )]),
            &context(),
        );

        assert_eq!(output, output.trim());
    }
}
