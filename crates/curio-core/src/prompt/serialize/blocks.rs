//! Walking the document: blocks, lists, and the runs of text inside them.
//!
//! Everything here is shape rather than meaning. What a chip *means* is [`super::chips`];
//! this half only decides what survives, in what order, and with which markdown around it.
//!
//! The one rule that is not obvious lives in [`Writer::section_has_content`], which runs the
//! whole expansion into a buffer it throws away — see the note on `Writer::probing` for why
//! that has to be told apart from a real write.

use super::*;

impl Writer<'_> {
    /// Whether anything before the next heading writes text.
    ///
    /// A horizontal rule counts as content on purpose — a user who put a divider under a
    /// heading meant the section to be there.
    pub(super) fn section_has_content(&mut self, rest: &[&serde_json::Value]) -> bool {
        let was_probing = std::mem::replace(&mut self.probing, true);

        let mut found = false;
        for block in rest.iter().take_while(|block| block["type"] != "heading") {
            if block["type"] == "horizontalRule" {
                found = true;
                break;
            }
            let mut text = String::new();
            self.write_block(&mut text, block, 0);
            if !text.trim().is_empty() {
                found = true;
                break;
            }
        }

        self.probing = was_probing;
        found
    }

    pub(super) fn write_block(&mut self, out: &mut String, node: &serde_json::Value, depth: usize) {
        match node["type"].as_str().unwrap_or_default() {
            "paragraph" => {
                let text = self.inline_text(node);
                // An untouched section contributes nothing, and the heading above it was
                // already skipped by `serialize`.
                if text.trim().is_empty() {
                    return;
                }
                push_block(out, &text);
            }

            "heading" => {
                let level = node["attrs"]["level"].as_u64().unwrap_or(1).clamp(1, 6) as usize;
                let text = self.inline_text(node);
                if !text.trim().is_empty() {
                    push_block(out, &format!("{} {text}", "#".repeat(level)));
                }
            }

            "bulletList" | "orderedList" => {
                let ordered = node["type"] == "orderedList";
                let items: Vec<&serde_json::Value> =
                    node["content"].as_array().into_iter().flatten().collect();

                for (index, item) in items.into_iter().enumerate() {
                    let marker = if ordered {
                        format!("{}. ", index + 1)
                    } else {
                        "- ".to_owned()
                    };
                    let body = self.list_item_text(item);
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
                let children: Vec<&serde_json::Value> =
                    node["content"].as_array().into_iter().flatten().collect();

                for child in children {
                    let text = self.inline_text(child);
                    if !text.trim().is_empty() {
                        push_block(out, &format!("> {text}"));
                    }
                }
            }

            "codeBlock" => {
                let text = self.inline_text(node);
                push_block(out, &format!("```\n{text}\n```"));
            }

            "horizontalRule" => push_block(out, "---"),

            // A node from a newer editor build. Walk into it: its text is the user's
            // writing, and dropping it silently would lose work rather than lose formatting.
            _ => {
                let children: Vec<&serde_json::Value> =
                    node["content"].as_array().into_iter().flatten().collect();

                for child in children {
                    self.write_block(out, child, depth);
                }
            }
        }
    }

    fn list_item_text(&mut self, item: &serde_json::Value) -> String {
        let children: Vec<&serde_json::Value> =
            item["content"].as_array().into_iter().flatten().collect();

        children
            .into_iter()
            .map(|child| self.inline_text(child))
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_owned()
    }

    fn inline_text(&mut self, node: &serde_json::Value) -> String {
        let children: Vec<&serde_json::Value> =
            node["content"].as_array().into_iter().flatten().collect();

        let mut out = String::new();
        for child in children {
            match child["type"].as_str().unwrap_or_default() {
                "text" => out.push_str(&marked_text(child)),
                "hardBreak" => out.push('\n'),
                "familyChip" | "tagChip" | "typeChip" | "itemRef" => {
                    out.push_str(&self.expand_chip(child));
                }
                _ => out.push_str(&self.inline_text(child)),
            }
        }
        out
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::serialize::fixtures::*;

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

        assert_eq!(body(&output), "## Brief\n\nA pricing page.");
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

        assert_eq!(body(&output), "## Context for the agent\n\nA pricing page.");
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

        assert_eq!(body(&output), "## Brief\n\nA pricing page.");
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

        assert_eq!(body(&output), "## Brief\n\nA pricing page.");
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

        assert_eq!(body(&output), "## Brief\n\nA pricing page.");
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
}
