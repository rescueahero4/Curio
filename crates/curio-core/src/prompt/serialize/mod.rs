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

mod blocks;
mod chips;
mod footer;

#[cfg(test)]
mod fixtures;

use chips::cited_item_ids;
use footer::write_back_footer;

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
    use super::fixtures::*;
    use super::*;
    use crate::prompt::template;

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
}
