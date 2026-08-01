//! What a prompt is called, derived from what it says.
//!
//! There is no title field in the editor. A prompt is a document, and a document that makes
//! you name it before you have written it asks the one question you cannot answer yet — so
//! the name is the first thing you wrote, the way an untitled note in any notes app is.
//!
//! This lives here, next to [`serialize`](super::serialize), for the reason R-FE-18 gives:
//! the server owns every derivation from the document. A title computed in the SPA would be
//! a second implementation, and the two would disagree eventually — at which point the list
//! would show one name, the `prompts/{id}.md` snapshot another, and a project card a third.
//! They all read the stored `title`, and this function is what puts it there.
//!
//! Chips expand to their **stored label**, not to their full description. A title is a name
//! at a glance; the expansion belongs in the serialized body, and resolving it here would
//! make naming a prompt cost a database join on every keystroke.

/// The longest a derived title may be, in characters.
///
/// A brief opens with a sentence, not a heading, so the first line is routinely longer than
/// anything that fits in a list row. Cut rather than let the row wrap to three lines: this
/// is a name, and the whole text is one click away.
const MAX_CHARS: usize = 72;

/// Name a prompt from its document — the first block the *author* put there.
///
/// Falls back to [`UNTITLED`](super::UNTITLED) for a document that is still all template,
/// which is what every new prompt is now that the scaffold arrives written out in full. A
/// prompt keeps the placeholder name until the moment its author changes a line.
///
/// **Anything the template wrote is skipped** — its headings and its bodies alike. Without
/// that, every new prompt in the list would be called "Build a product landing page for
/// ACME", which is both wrong and identical for all of them. Only *exact* template text is
/// skipped, so the moment a user edits a line it becomes eligible: adapting the Brief, or
/// retitling the first section, are both perfectly good ways to name a prompt, and both are
/// things a user does without ever thinking about titles.
#[must_use]
pub fn title_from(doc: &serde_json::Value) -> String {
    doc["content"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|block| {
            let text = collapse(&block_text(block));
            if text.is_empty() || is_scaffold(block, &text) {
                return None;
            }
            Some(text)
        })
        .next()
        .map_or_else(|| super::UNTITLED.to_owned(), |text| truncate(&text))
}

/// Text the template wrote and the user has not touched.
fn is_scaffold(block: &serde_json::Value, text: &str) -> bool {
    if block["type"] == "heading" {
        return super::template::is_section_heading(text);
    }
    super::template::is_section_body(text)
}

/// Every scrap of text a block holds, chips included, in document order.
///
/// Walks unknown node types rather than skipping them, on the same reasoning the serializer
/// records: a node from a newer editor build still contains the user's writing, and a title
/// that silently ignored it would name the prompt after the second paragraph.
fn block_text(node: &serde_json::Value) -> String {
    if let Some(text) = node["text"].as_str() {
        return text.to_owned();
    }

    // A hard break carries no text but is a word boundary, exactly as it is in the
    // serializer. Without this, "Two words⏎and a third" collapses to "wordsand".
    if node["type"] == "hardBreak" {
        return " ".to_owned();
    }

    // A chip is a leaf: it has no `content`, and its label is the only human-readable thing
    // on it. Without this a document that opens with `/aesthetic:warm` would be untitled.
    if let Some(label) = node["attrs"]["label"].as_str() {
        return label.to_owned();
    }

    node["content"]
        .as_array()
        .into_iter()
        .flatten()
        .map(block_text)
        .collect::<Vec<_>>()
        .join("")
}

/// One line, one space between words. A title is a row in a list, not a paragraph.
fn collapse(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Cut on a character boundary, and prefer a word boundary near the limit.
fn truncate(text: &str) -> String {
    if text.chars().count() <= MAX_CHARS {
        return text.to_owned();
    }

    let cut: String = text.chars().take(MAX_CHARS).collect();
    // Trailing partial word: drop it, unless doing so would leave almost nothing — a title
    // cut to "The" because the next word is long is worse than one cut mid-word.
    let trimmed = match cut.rfind(' ') {
        Some(space) if space >= MAX_CHARS / 2 => &cut[..space],
        _ => cut.trim_end(),
    };

    format!("{trimmed}…")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn doc(blocks: serde_json::Value) -> serde_json::Value {
        json!({ "type": "doc", "content": blocks })
    }

    fn paragraph(text: &str) -> serde_json::Value {
        json!({ "type": "paragraph", "content": [{ "type": "text", "text": text }] })
    }

    #[test]
    fn a_fresh_template_is_untitled() {
        // Every new prompt is exactly this: eight empty, section-attributed paragraphs. If
        // this returned anything else, a list of new prompts would be a list of blanks.
        assert_eq!(
            title_from(&super::super::starter_document()),
            super::super::UNTITLED
        );
    }

    #[test]
    fn the_first_written_line_names_the_prompt() {
        let document = doc(json!([
            { "type": "paragraph", "attrs": { "section": "brief" } },
            paragraph("A pricing page for a logistics tool"),
            paragraph("Something written later"),
        ]));

        assert_eq!(title_from(&document), "A pricing page for a logistics tool");
    }

    #[test]
    fn an_empty_block_is_skipped_rather_than_named() {
        // Whitespace-only is empty. A paragraph holding a space is what a user leaves behind
        // when they type and delete, and naming the prompt " " would look like a bug.
        let document = doc(json!([paragraph("   "), paragraph("The real first line")]));

        assert_eq!(title_from(&document), "The real first line");
    }

    #[test]
    fn a_heading_can_be_the_title() {
        let document = doc(json!([
            { "type": "heading", "attrs": { "level": 2 },
              "content": [{ "type": "text", "text": "Checkout flow" }] },
        ]));

        assert_eq!(title_from(&document), "Checkout flow");
    }

    #[test]
    fn the_scaffolds_own_headings_do_not_name_the_prompt() {
        // The whole template is headings now. Without the skip, every new prompt in the list
        // would be called "Brief" — including ones the user has written nothing in.
        let document = doc(json!([
            { "type": "heading", "attrs": { "level": 2 },
              "content": [{ "type": "text", "text": "Brief" }] },
            { "type": "paragraph", "attrs": { "section": "brief" },
              "content": [{ "type": "text", "text": "A pricing page" }] },
        ]));

        assert_eq!(title_from(&document), "A pricing page");
    }

    #[test]
    fn a_renamed_section_heading_does_name_the_prompt() {
        // The counterpart, and the reason the skip matches on the template's exact names
        // rather than on "is a heading": renaming the first section is a natural way to name
        // a prompt, and a rule that ignored all headings would silently refuse it.
        let document = doc(json!([
            { "type": "heading", "attrs": { "level": 2 },
              "content": [{ "type": "text", "text": "Checkout flow" }] },
            { "type": "paragraph", "attrs": { "section": "brief" } },
        ]));

        assert_eq!(title_from(&document), "Checkout flow");
    }

    #[test]
    fn a_retyped_section_name_is_still_the_scaffolds() {
        // Case and surrounding space are not a rename.
        let document = doc(json!([
            { "type": "heading", "attrs": { "level": 2 },
              "content": [{ "type": "text", "text": "  brief  " }] },
            { "type": "paragraph", "content": [{ "type": "text", "text": "The real line" }] },
        ]));

        assert_eq!(title_from(&document), "The real line");
    }

    #[test]
    fn a_list_can_be_the_title() {
        // Nested two levels deep — list → item → paragraph → text. The walk has to reach it.
        let document = doc(json!([{
            "type": "bulletList",
            "content": [{ "type": "listItem", "content": [paragraph("First bullet")] }],
        }]));

        assert_eq!(title_from(&document), "First bullet");
    }

    #[test]
    fn a_chip_contributes_its_label_not_its_expansion() {
        // The serializer would turn this into a paragraph of description and an absolute
        // path. A title is a name: the label is what the user sees in the editor, so it is
        // what they should see in the list.
        let document = doc(json!([{
            "type": "paragraph",
            "content": [
                { "type": "text", "text": "Like " },
                { "type": "familyChip", "attrs": { "id": "fam_1", "label": "Warm brutalist" } },
            ],
        }]));

        assert_eq!(title_from(&document), "Like Warm brutalist");
    }

    #[test]
    fn a_document_that_opens_with_a_chip_is_still_named() {
        let document = doc(json!([{
            "type": "paragraph",
            "content": [{ "type": "itemRef", "attrs": { "id": "itm_1", "label": "Stripe pricing" } }],
        }]));

        assert_eq!(title_from(&document), "Stripe pricing");
    }

    #[test]
    fn line_breaks_and_runs_of_space_become_single_spaces() {
        let document = doc(json!([{
            "type": "paragraph",
            "content": [
                { "type": "text", "text": "Two   words" },
                { "type": "hardBreak" },
                { "type": "text", "text": "and a third" },
            ],
        }]));

        assert_eq!(title_from(&document), "Two words and a third");
    }

    #[test]
    fn a_long_first_line_is_cut_at_a_word() {
        let document = doc(json!([paragraph(
            "A landing page for a supply-chain intelligence platform aimed at mid-size manufacturers"
        )]));

        let title = title_from(&document);

        assert!(title.ends_with('…'), "{title}");
        assert!(title.chars().count() <= MAX_CHARS + 1, "{title}");
        assert!(!title.contains("manufacturers"), "{title}");
        // Cut between words, never mid-word. The 72nd character lands inside "mid-size", so
        // the whole partial word goes rather than being shown as "mid-siz".
        assert_eq!(
            title,
            "A landing page for a supply-chain intelligence platform aimed at…"
        );
    }

    #[test]
    fn one_very_long_word_is_cut_mid_word_rather_than_to_nothing() {
        // A pasted URL is the real case. Falling back to the last space would leave "See",
        // which names nothing.
        let document = doc(json!([paragraph(&format!("See {}", "x".repeat(200)))]));

        let title = title_from(&document);

        assert!(title.chars().count() <= MAX_CHARS + 1, "{title}");
        assert!(title.starts_with("See xxx"), "{title}");
    }

    #[test]
    fn an_unknown_node_type_is_walked_into() {
        // A node from a newer editor build. Its text is the user's writing.
        let document = doc(json!([{
            "type": "callout",
            "content": [paragraph("Inside something new")],
        }]));

        assert_eq!(title_from(&document), "Inside something new");
    }

    #[test]
    fn a_document_with_no_content_at_all_is_untitled() {
        assert_eq!(
            title_from(&json!({ "type": "doc" })),
            super::super::UNTITLED
        );
    }
}
