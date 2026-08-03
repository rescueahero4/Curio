//! What a chip expands to.
//!
//! The rules that make this more than a lookup are all about the document as a whole: a
//! family offers its exemplars once however many times it is cited, and an item the user
//! named explicitly is not offered back to them as a suggestion. That is why the expansion
//! hangs off [`Writer`] rather than being a free function over [`ChipContext`].

use super::*;

impl Writer<'_> {
    /// Expand one chip (Inventory §6).
    ///
    /// Every branch falls back to the chip's stored `label` when the referenced row is gone
    /// (R-FE-17). A prompt written six months ago that cites a since-deleted item must still
    /// read as a sentence — losing the path is a degradation; losing the word is a hole in
    /// the middle of the user's brief.
    pub(super) fn expand_chip(&mut self, chip: &serde_json::Value) -> String {
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
pub(super) fn cited_item_ids(doc: &serde_json::Value) -> HashSet<String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::serialize::fixtures::*;

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
}
