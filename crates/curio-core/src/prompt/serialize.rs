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
//!
//! A `familyChip` was the exception, and the gap showed. It expanded to a name and a
//! sentence — a label for a visual sense the library holds dozens of examples of, none of
//! which the model could reach, so it invented its own reading of the phrase. Family chips
//! now carry a few exemplar items: ids **and** absolute directories, because an id alone is
//! useless to an agent that has never heard of Curio, and a path alone is useless to
//! `library_get_item`.
//!
//! ## Why there is a footer
//!
//! The text used to end wherever the user's writing ended, which meant a correctly built
//! project could land anywhere on disk and Curio would never learn it existed. There are
//! only two ways a path reaches the library: the watcher, which sweeps one configured root
//! at depth 0 and does not recurse, and `project_register`, which no prompt ever asked
//! anyone to call. An agent working in its own directory — the normal case — could satisfy
//! the whole brief and still leave the Projects page empty and the prompt's claim
//! outstanding.
//!
//! The footer closes that loop without breaking FR-14. It is an instruction to a connected
//! agent, never a prerequisite: an agent with no MCP can follow every other line of the
//! prompt and say so in its reply.

use std::collections::{HashMap, HashSet};

/// How many exemplars a family chip carries.
///
/// Three. This lands in every prompt that cites a family and the length is paid on every
/// paste — enough to show a range, not enough to read as a list to work through.
///
/// Public because the caller that ranks the samples has to cap them with the same number.
/// Two constants that must agree is a bug waiting for someone to change one of them.
pub const MAX_SAMPLES: usize = 3;

/// One exemplar item offered alongside a family chip.
#[derive(Debug, Clone)]
pub struct FamilySample {
    pub id: String,
    pub name: String,
    /// **Absolute**, for the same reason an `itemRef`'s is (FR-14).
    pub directory: String,
}

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
    /// Family id → exemplars, best first and already ranked by the caller.
    ///
    /// Ranked *there* rather than here, and deterministically, because the serialized text
    /// is written to `prompts/{id}.md` on every save: re-serializing an unchanged prompt
    /// has to produce byte-identical output or every save rewrites the snapshot.
    pub family_samples: HashMap<String, Vec<FamilySample>>,
    /// The directory the watcher sweeps, or empty when none is configured.
    pub projects_root: String,
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
///
/// ## The footer is not part of the document
///
/// It is appended after everything the user wrote, and it survives a document the user
/// emptied completely — where the result goes is still true of a prompt whose sections have
/// all been deleted.
#[must_use]
pub fn serialize(doc: &serde_json::Value, context: &ChipContext) -> String {
    let mut writer = Writer {
        context,
        cited_items: cited_item_ids(doc),
        sampled_families: HashSet::new(),
        probing: false,
    };

    let mut out = String::new();
    let blocks: Vec<&serde_json::Value> = doc["content"].as_array().into_iter().flatten().collect();

    for (index, block) in blocks.iter().enumerate() {
        if block["type"] == "heading" && !writer.section_has_content(&blocks[index + 1..]) {
            continue;
        }
        writer.write_block(&mut out, block, 0);
    }

    let body = collapse_blank_runs(out.trim());
    let footer = write_back_footer(context);

    if body.is_empty() {
        footer
    } else {
        format!("{body}\n\n{footer}")
    }
}

/// One serialization pass, carrying the state that makes a document read as a whole.
///
/// Two decisions cannot be made chip by chip. A family cited three times should offer its
/// samples once, and an exemplar the user already cited deliberately as an `itemRef` should
/// not be handed back to them as a suggestion. Both need to know about the rest of the
/// document, so they live here rather than in a pure per-chip function.
struct Writer<'a> {
    context: &'a ChipContext,
    /// Items the document names explicitly, anywhere in it.
    cited_items: HashSet<String>,
    /// Families that have already spent their one sample block.
    sampled_families: HashSet<String>,
    /// Whether this pass is the emptiness probe rather than the real write.
    ///
    /// [`Writer::section_has_content`] runs the same expansion into a scratch buffer to find
    /// out whether a block produces any text. Unguarded it would spend a family's one sample
    /// block on a buffer that is thrown away, and the real write immediately afterwards would
    /// silently omit it.
    probing: bool,
}

impl Writer<'_> {
    /// Whether anything before the next heading writes text.
    ///
    /// A horizontal rule counts as content on purpose — a user who put a divider under a
    /// heading meant the section to be there.
    fn section_has_content(&mut self, rest: &[&serde_json::Value]) -> bool {
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

    fn write_block(&mut self, out: &mut String, node: &serde_json::Value, depth: usize) {
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

    /// Expand one chip (Inventory §6).
    ///
    /// Every branch falls back to the chip's stored `label` when the referenced row is gone
    /// (R-FE-17). A prompt written six months ago that cites a since-deleted item must still
    /// read as a sentence — losing the path is a degradation; losing the word is a hole in
    /// the middle of the user's brief.
    fn expand_chip(&mut self, chip: &serde_json::Value) -> String {
        let id = chip["attrs"]["id"].as_str().unwrap_or_default();
        let label = chip["attrs"]["label"].as_str().unwrap_or_default();

        match chip["type"].as_str().unwrap_or_default() {
            "familyChip" => {
                let head = match self.context.families.get(id) {
                    Some((name, description)) => format!("Aesthetic: {name} — {description}"),
                    None if label.is_empty() => return String::new(),
                    None => return format!("Aesthetic: {label}"),
                };
                self.with_samples(id, head)
            }

            "itemRef" => match self.context.items.get(id) {
                Some((name, directory)) => format!(
                    "Reference: {name} — {directory} — read screenshot.png and item.md before \
                     designing; match feel, not content."
                ),
                None if label.is_empty() => String::new(),
                None => format!("Reference: {label}"),
            },

            // Tags and design types carry no extra payload — the word is the whole point.
            _ => self
                .context
                .terms
                .get(id)
                .map_or_else(|| label.to_owned(), Clone::clone),
        }
    }

    /// Offer a family's exemplars, at most once per document.
    ///
    /// The wording is doing a specific job. A single path in a prompt reads as a thing to
    /// reproduce, so the sentence says outright that a family is a sense and not a template
    /// — the same distinction `itemRef` draws with "match feel, not content", phrased to
    /// stay consistent with it.
    fn with_samples(&mut self, id: &str, head: String) -> String {
        // The probe must not spend the one chance: the real pass runs immediately after and
        // needs the samples still owed.
        if self.probing || !self.sampled_families.insert(id.to_owned()) {
            return head;
        }

        let list: Vec<String> = self
            .context
            .family_samples
            .get(id)
            .into_iter()
            .flatten()
            // An item the user cited deliberately elsewhere is not a suggestion worth making
            // back to them; they already chose it.
            .filter(|sample| !self.cited_items.contains(&sample.id))
            .take(MAX_SAMPLES)
            .map(|sample| format!("{} ({}) {}", sample.name, sample.id, sample.directory))
            .collect();

        // R-FE-17's rule at family scale: a family with nothing eligible degrades to the
        // sentence it has always been, never to an empty "Samples:" tail.
        if list.is_empty() {
            return head;
        }

        format!(
            "{head}. A visual sense, not a template: open one or two of these to calibrate, \
             and again to check your work — match the feel, not the content. Samples: {}. \
             Each folder holds screenshot.png and item.md.",
            list.join("; ")
        )
    }
}

/// Every item the document cites explicitly, collected before serializing.
///
/// Before rather than during, because the first family chip has to know about an `itemRef`
/// that may not appear until further down the document.
fn cited_item_ids(doc: &serde_json::Value) -> HashSet<String> {
    fn walk(node: &serde_json::Value, ids: &mut HashSet<String>) {
        if node["type"] == "itemRef"
            && let Some(id) = node["attrs"]["id"].as_str()
        {
            ids.insert(id.to_owned());
        }
        for child in node["content"].as_array().into_iter().flatten() {
            walk(child, ids);
        }
    }

    let mut ids = HashSet::new();
    walk(doc, &mut ids);
    ids
}

/// The write-back instruction every serialized prompt ends with.
///
/// Two halves, in this order because they are not equally cheap. Building under the watched
/// root costs the agent nothing further — the watcher adopts the folder within a few
/// seconds, mints `.curio-project`, and spends the prompt's claim. Registering is the
/// fallback for what is actually the normal case: a person working in their own directory.
///
/// **Once** is stated rather than implied. A folder Curio already knows is never re-adopted
/// and cannot spend a second claim, so a repeated call is harmless but pointless, and an
/// agent iterating on one project across several turns should not make it every turn.
///
/// It steers toward `project_register`'s `variants` argument rather than the agent
/// hand-writing `curio-variants.json`. The argument validates every entry against what is on
/// disk and writes the shape Curio actually reads; a hand-written file parses either way, so
/// a wrong one fails silently into an unlabelled switcher.
fn write_back_footer(context: &ChipContext) -> String {
    let mut footer = String::from("## Where this lands\n\n");

    if !context.projects_root.trim().is_empty() {
        footer.push_str(&format!(
            "Curio is watching `{}`. A project built directly inside it is picked up within a \
             few seconds, and nothing further is needed.\n\n",
            context.projects_root.trim()
        ));
    }

    footer.push_str(
        "If the project lands anywhere else, call the `project_register` MCP tool **once**, \
         after the folder is finished, with its absolute path. Pass its `variants` argument if \
         you produced several versions, so Curio writes `curio-variants.json` itself instead of \
         you hand-writing it. If you have no Curio MCP connection, say so in your reply and it \
         can be registered by hand.",
    );

    footer
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

    /// The document half, with the write-back footer removed.
    ///
    /// The body rules and the footer are independent. A body test that asserted the whole
    /// output would fail every time the footer's wording was improved, which would make the
    /// footer expensive to touch for no gain in what those tests actually check.
    fn body(output: &str) -> String {
        match output.find("## Where this lands") {
            Some(at) => output[..at].trim_end().to_owned(),
            None => output.trim_end().to_owned(),
        }
    }

    fn sample(id: &str, name: &str) -> FamilySample {
        FamilySample {
            id: id.to_owned(),
            name: name.to_owned(),
            directory: format!("C:\\Users\\me\\Curio\\items\\{id}"),
        }
    }

    fn context() -> ChipContext {
        let mut context = ChipContext::default();
        context.families.insert(
            "fam1".to_owned(),
            (
                "Warm Editorial".to_owned(),
                "Serif headlines on paper-warm neutrals".to_owned(),
            ),
        );
        // A family the library holds nothing eligible for — every item in it is gray-zone,
        // or it has none at all.
        context.families.insert(
            "fam2".to_owned(),
            ("Cold Brutalist".to_owned(), "Concrete and grids".to_owned()),
        );
        context.family_samples.insert(
            "fam1".to_owned(),
            vec![
                sample("01JAAA", "Stripe pricing"),
                sample("01JBBB", "Monzo blog"),
            ],
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
        context.projects_root = "C:\\Users\\me\\Curio\\projects".to_owned();
        context
    }

    /// A chip node, for the tests that build documents by hand.
    fn chip(kind: &str, id: &str, label: &str) -> serde_json::Value {
        serde_json::json!({ "type": kind, "attrs": { "id": id, "label": label } })
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

        assert_eq!(body(&output), "## Brief\n\nA pricing page.");
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
    fn an_emptied_document_still_carries_the_write_back_footer() {
        // What a user has after select-all-delete. The body is genuinely nothing, but where
        // the result goes is still true of a prompt with no sections left — so the footer is
        // appended to the document rather than being part of it.
        let output = serialize(&doc(vec![]), &context());

        assert_eq!(body(&output), "");
        assert!(output.starts_with("## Where this lands"), "{output}");
        assert!(output.contains("project_register"), "{output}");
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

    /* -- family exemplars ------------------------------------------------------------ */

    #[test]
    fn a_family_chip_carries_exemplars_with_ids_and_absolute_paths() {
        // The gap this closes: a description is a label for a visual sense the library holds
        // dozens of examples of, and the model could reach none of them. Both halves are
        // emitted because they serve different readers — the path is for an agent that has
        // never heard of Curio, the id is what `library_get_item` takes.
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
            )]),
            &context(),
        );

        assert!(output.contains("Aesthetic: Warm Editorial"), "{output}");
        assert!(output.contains("01JAAA"), "{output}");
        assert!(
            output.contains("C:\\Users\\me\\Curio\\items\\01JAAA"),
            "{output}"
        );
        assert!(output.contains("Stripe pricing"), "{output}");
    }

    #[test]
    fn the_exemplars_say_the_family_is_a_sense_and_not_a_template() {
        // A single path in a prompt reads as a thing to reproduce. Said outright, and phrased
        // to stay consistent with the itemRef wording it sits beside.
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
            )]),
            &context(),
        );

        assert!(output.contains("not a template"), "{output}");
        assert!(
            output.contains("match the feel, not the content"),
            "{output}"
        );
    }

    #[test]
    fn no_more_than_three_exemplars_reach_the_prompt() {
        // The length is paid on every paste of every prompt that cites a family.
        let mut context = context();
        context.family_samples.insert(
            "fam1".to_owned(),
            (0..6).map(|n| sample(&format!("0{n}"), "Item")).collect(),
        );

        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
            )]),
            &context,
        );

        assert_eq!(output.matches("Curio\\items\\0").count(), MAX_SAMPLES);
    }

    #[test]
    fn a_family_cited_twice_offers_its_samples_once() {
        // Otherwise a prompt that mentions one aesthetic in three sections carries the same
        // three paths three times.
        let output = serialize(
            &doc(vec![
                paragraph(
                    "direction",
                    serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
                ),
                paragraph(
                    "important",
                    serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
                ),
            ]),
            &context(),
        );

        assert_eq!(output.matches("Aesthetic: Warm Editorial").count(), 2);
        assert_eq!(output.matches("Samples:").count(), 1);
    }

    #[test]
    fn an_exemplar_the_user_already_cited_is_not_offered_back() {
        // They chose it deliberately. Repeating it as a suggestion spends length on something
        // already in the prompt, and reads as though Curio did not notice.
        let mut context = context();
        context.items.insert(
            "01JAAA".to_owned(),
            (
                "Stripe pricing".to_owned(),
                "C:\\Users\\me\\Curio\\items\\01JAAA".to_owned(),
            ),
        );

        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([
                    chip("familyChip", "fam1", "Warm Editorial"),
                    chip("itemRef", "01JAAA", "Stripe pricing"),
                ]),
            )]),
            &context,
        );

        // Bounded to the sample sentence itself: the explicit reference expands further down
        // and legitimately names the same item, which is the whole point.
        let from = output.find("Samples:").expect("samples");
        let to = output[from..].find("Each folder").expect("terminator") + from;
        let samples = &output[from..to];

        assert!(output.contains("Reference: Stripe pricing"), "{output}");
        assert!(!samples.contains("01JAAA"), "{samples}");
        assert!(samples.contains("01JBBB"), "{samples}");
    }

    #[test]
    fn a_family_with_nothing_eligible_serializes_exactly_as_it_used_to() {
        // R-FE-17 at family scale: a missing referent degrades the detail, never the
        // sentence. An empty "Samples:" tail would be worse than no tail.
        let output = serialize(
            &doc(vec![paragraph(
                "direction",
                serde_json::json!([chip("familyChip", "fam2", "Cold Brutalist")]),
            )]),
            &context(),
        );

        assert_eq!(
            body(&output),
            "Aesthetic: Cold Brutalist — Concrete and grids"
        );
    }

    #[test]
    fn a_family_chip_inside_a_section_still_gets_its_samples() {
        // The subtle one. `section_has_content` re-runs the whole expansion into a scratch
        // buffer to decide whether the heading survives — so without a guard the probe spends
        // the family's one sample block on a buffer that is thrown away, and the real write
        // silently emits the bare sentence. Every chip in a real prompt sits under a heading.
        let output = serialize(
            &doc(vec![
                heading(2, "Direction"),
                paragraph(
                    "direction",
                    serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
                ),
            ]),
            &context(),
        );

        assert!(output.contains("## Direction"), "{output}");
        assert!(output.contains("Samples:"), "{output}");
    }

    #[test]
    fn re_serializing_an_unchanged_prompt_is_byte_identical() {
        // The snapshot at `prompts/{id}.md` is rewritten on every save. Sample order that
        // wandered between runs would rewrite the file on every keystroke and make the
        // history worthless.
        let document = doc(vec![paragraph(
            "direction",
            serde_json::json!([chip("familyChip", "fam1", "Warm Editorial")]),
        )]);

        assert_eq!(
            serialize(&document, &context()),
            serialize(&document, &context())
        );
    }

    /* -- the write-back footer -------------------------------------------------------- */

    #[test]
    fn the_footer_names_the_root_curio_is_watching() {
        // Half the instruction is "build here and do nothing else", which is only usable if
        // the prompt says where here is.
        let output = serialize(
            &doc(vec![paragraph("brief", serde_json::json!([text("Hi.")]))]),
            &context(),
        );

        assert!(
            output.contains("C:\\Users\\me\\Curio\\projects"),
            "{output}"
        );
        assert!(output.contains("project_register"), "{output}");
    }

    #[test]
    fn the_register_instruction_says_to_call_it_once() {
        // A folder Curio already knows cannot spend a second claim, so a repeated call is
        // harmless but pointless — and an agent iterating across turns will otherwise make it
        // every turn.
        let output = serialize(&doc(vec![]), &context());
        assert!(output.contains("**once**"), "{output}");
    }

    #[test]
    fn the_footer_prefers_the_variants_argument_over_hand_written_json() {
        // The live failure this came from: an agent hand-wrote curio-variants.json in a shape
        // Curio parses but cannot read — trailing slash on the folder, wrong key names, no
        // version. `project_register` writes it correctly by construction.
        let output = serialize(&doc(vec![]), &context());

        assert!(output.contains("variants"), "{output}");
        assert!(output.contains("curio-variants.json"), "{output}");
    }

    #[test]
    fn without_a_configured_root_only_the_register_half_survives() {
        // Emitting a path that is not there would be worse than saying nothing: an agent
        // would build into a directory nothing is watching and believe it was done.
        let mut context = context();
        context.projects_root = String::new();

        let output = serialize(&doc(vec![]), &context);

        assert!(!output.contains("is watching"), "{output}");
        assert!(output.contains("project_register"), "{output}");
    }

    #[test]
    fn the_footer_never_makes_mcp_a_prerequisite() {
        // FR-14 is the whole product bet: an agent that has never heard of Curio must be able
        // to follow the prompt. The footer is an instruction to a connected agent and has to
        // give an unconnected one somewhere to go.
        let output = serialize(&doc(vec![]), &context());

        assert!(
            output.contains("no Curio MCP connection"),
            "the footer must tell an unconnected agent what to do: {output}"
        );
    }
}
