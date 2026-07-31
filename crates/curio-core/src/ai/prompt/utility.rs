//! The two utility-model calls: vocabulary dedupe and bulk re-tag.
//!
//! Separated from the vision call because they share a constraint the vision call does not:
//! **no `effort` parameter**. The cheap utility model rejects it outright, so every request
//! built here leaves it absent rather than null (R-BE-24, Inventory §10.7). Keeping them in
//! one file means the rule has one place to be forgotten in, and one test guarding it.
//!
//! Both also use a single cached system block rather than two. There is no rubric here —
//! the shared prefix is the instruction plus the vocabulary, and splitting a short block in
//! half buys nothing.

use super::{UTILITY_MAX_TOKENS, Vocabulary};
use crate::ai::wire::{Content, Message, MessagesRequest, OutputConfig, OutputFormat, SystemBlock};

/// One utility call over a list of names (R-BE-24).
///
/// A **single** cached system block and **no `effort`** — the utility model rejects the
/// parameter (Inventory §10.7).
#[must_use]
pub fn dedupe(model: &str, kind_plural: &str, names: &[String]) -> MessagesRequest {
    let instructions = format!(
        "You are tidying a design library's {kind_plural}.\n\n\
         Group names that mean the same thing, so the user can merge them. For each group, \
         say why they are the same, pick the one to keep, and list the others.\n\n\
         Rules:\n\
         - Use names **verbatim** from the supplied list. A name you invent cannot be merged \
           and will be discarded.\n\
         - A group must never fold a name into itself.\n\
         - Near-synonyms that a designer would filter on separately are not duplicates. \
           'minimal' and 'minimalist' are; 'dark' and 'dark mode' are; 'blue' and 'navy' \
           are not.\n\
         - If you reconsider a group, return it with an empty merge list rather than \
           inventing a weaker one. An empty list withdraws it.\n\
         - Returning no groups at all is a good answer when the vocabulary is already clean."
    );

    MessagesRequest {
        model: model.to_owned(),
        max_tokens: UTILITY_MAX_TOKENS,
        system: vec![SystemBlock::cached(instructions)],
        messages: vec![Message::user(vec![Content::text(format!(
            "Here are the {kind_plural}:\n\n{}",
            names.join("\n")
        ))])],
        output_config: Some(OutputConfig {
            // R-BE-24. Absent, not null.
            effort: None,
            format: Some(OutputFormat {
                kind: "json_schema",
                schema: crate::ai::schema::raw(crate::ai::schema::DEDUPE),
            }),
        }),
    }
}

/// What a re-tag knows about one item.
#[derive(Debug, Clone, Default)]
pub struct ItemSummary {
    pub name: String,
    pub short_description: String,
    pub design_types: Vec<String>,
    pub tags: Vec<String>,
}

/// One item's re-tag call (R-BE-18).
///
/// ## D33: text, not vision
///
/// ARCH-01 R-BE-18 does not say whether a re-tag re-reads the screenshot. It is written as
/// a text call over the item's existing description, for two reasons. Batching 500 images
/// would approach the Batch API's whole-request size limit and cost image tokens per item
/// for information already extracted; and the short description was itself written from
/// the screenshot by the assessment call, so the visual evidence is present in words. A
/// user who wants the image re-read has re-assess, which does exactly that.
///
/// The system block is shared across every item in a bulk run and cached once — with the
/// per-item detail in the user turn, below the breakpoint, where it belongs.
#[must_use]
pub fn retag(
    model: &str,
    vocabulary: &Vocabulary,
    instruction: &str,
    item: &ItemSummary,
) -> MessagesRequest {
    let mut system = String::from(
        "You are re-tagging items in a designer's reference library.\n\n\
         Return the tags and design types this item should carry. Prefer terms the library \
         already uses — a near-duplicate you invent is a filter the user scrolls past \
         forever. Do not rename the item and do not describe it; only its vocabulary is \
         being changed.\n\n",
    );
    if !instruction.trim().is_empty() {
        system.push_str(&format!(
            "The user asked for this specifically:\n\n{}\n\n",
            instruction.trim()
        ));
    }
    system.push_str(&vocabulary.render());

    let mut detail = format!("Item: {}\n", item.name);
    if !item.short_description.trim().is_empty() {
        detail.push_str(&format!("Description: {}\n", item.short_description.trim()));
    }
    if !item.design_types.is_empty() {
        detail.push_str(&format!(
            "Current design types: {}\n",
            item.design_types.join(", ")
        ));
    }
    if !item.tags.is_empty() {
        detail.push_str(&format!("Current tags: {}\n", item.tags.join(", ")));
    }

    MessagesRequest {
        model: model.to_owned(),
        max_tokens: UTILITY_MAX_TOKENS,
        system: vec![SystemBlock::cached(system)],
        messages: vec![Message::user(vec![Content::text(detail)])],
        output_config: Some(OutputConfig {
            // R-BE-24: the utility model rejects it.
            effort: None,
            format: Some(OutputFormat {
                kind: "json_schema",
                schema: crate::ai::schema::raw(crate::ai::schema::RETAG),
            }),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vocabulary() -> Vocabulary {
        Vocabulary {
            families: vec![("Brutalist".to_owned(), "Heavy type, raw edges".to_owned())],
            design_types: vec!["pricing page".to_owned()],
            tags: vec!["saas".to_owned()],
        }
    }

    #[test]
    fn a_utility_call_sends_no_effort_parameter_at_all() {
        // Inventory §10.7: the utility model rejects it, so this is a 400 rather than a
        // stylistic difference.
        let json =
            serde_json::to_value(dedupe("claude-haiku-4-5", "tags", &["a".to_owned()])).expect("s");

        assert!(json["output_config"].get("effort").is_none());
        assert_eq!(json["max_tokens"], UTILITY_MAX_TOKENS);
        assert_eq!(json["system"].as_array().expect("system").len(), 1);
    }

    #[test]
    fn a_retag_shares_one_cached_block_across_the_whole_run() {
        // A 500-item bulk retag sends the same instruction and vocabulary 500 times. One
        // breakpoint over them turns 499 of those into cache reads; per-item detail sits
        // in the user turn, below it.
        let built = retag(
            "claude-haiku-4-5",
            &vocabulary(),
            "Use British spelling",
            &ItemSummary {
                name: "Stripe pricing".to_owned(),
                short_description: "A clean table".to_owned(),
                tags: vec!["saas".to_owned()],
                ..ItemSummary::default()
            },
        );
        let json = serde_json::to_value(&built).expect("serialize");

        let system = json["system"].as_array().expect("system");
        assert_eq!(system.len(), 1);
        assert!(system[0]["cache_control"].is_object());

        let cached = system[0]["text"].as_str().expect("text");
        assert!(cached.contains("British spelling"), "shared across the run");
        assert!(
            !cached.contains("Stripe"),
            "per-item detail leaked into cache"
        );
    }

    #[test]
    fn a_retag_cannot_return_a_name_or_a_family() {
        // Inventory §10.12: a bulk edit does not stamp `last_edited_by`, so a rename here
        // would rewrite a curated item with nothing recording who did it.
        let json = serde_json::to_value(retag("m", &vocabulary(), "", &ItemSummary::default()))
            .expect("serialize");
        let schema = json["output_config"]["format"]["schema"].to_string();

        assert!(!schema.contains("name_suggestion"));
        assert!(!schema.contains("family_scores"));
    }

    #[test]
    fn a_retag_with_no_instruction_omits_the_heading() {
        let built = retag("m", &vocabulary(), "   ", &ItemSummary::default());
        assert!(!built.system[0].text.contains("asked for this specifically"));
    }
}
