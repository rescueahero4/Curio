//! The gold-standard prompt template (FR-12).
//!
//! Eight sections, each a paragraph carrying a hidden `section` attribute. The attribute
//! is what drives the editor's ghost text and what the serializer turns into a heading —
//! one fact, two uses, so a section cannot render a prompt for input it will not label on
//! the way out.
//!
//! Every section is **deletable**. The template is a scaffold for a good brief, not a form
//! to fill in: a user who has nothing to say about `Never` should be able to delete the
//! section rather than leave an empty heading in the output.

use serde::{Deserialize, Serialize};

/// One template section.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// The value stored in the paragraph's `section` attribute.
    pub id: &'static str,
    /// The heading the serializer emits when the section has content.
    pub heading: &'static str,
    /// Placeholder text shown in the empty paragraph. Never serialized — it is a prompt to
    /// the author, and emitting it would put Curio's words in the user's brief.
    pub ghost: &'static str,
}

/// The eight sections, in the order they appear in a new prompt.
///
/// The order is the argument: what is being built, who it is for, what must and must not
/// happen, how it should look, the one thing that must not be missed, and what to hand
/// back. A model reads it top to bottom and the constraints arrive before the aesthetics.
pub const SECTIONS: [Section; 8] = [
    Section {
        id: "brief",
        heading: "Brief",
        ghost: "What are we building? One or two sentences.",
    },
    Section {
        id: "intent",
        heading: "Intent",
        ghost: "Who is it for, and what should it make them feel?",
    },
    Section {
        id: "guardrails",
        heading: "Guardrails",
        ghost: "The rules that hold whatever else changes.",
    },
    Section {
        id: "always",
        heading: "Always",
        ghost: "Non-negotiables. Accessibility, platform conventions, brand rules.",
    },
    Section {
        id: "never",
        heading: "Never",
        ghost: "What would make this wrong even if it looked good.",
    },
    Section {
        id: "direction",
        heading: "Design Direction",
        ghost: "Type / to pull an aesthetic, style, type, or item from your library.",
    },
    Section {
        id: "important",
        heading: "Important",
        ghost: "The one thing that must not be missed.",
    },
    Section {
        id: "output",
        heading: "Output",
        ghost: "What should the tool hand back — files, a route, a component?",
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

/// A fresh, empty TipTap document carrying the eight ghost sections.
///
/// Returned by `GET /api/prompts/template` and used as the `doc_json` of every new prompt,
/// so the server owns the template rather than each client re-deriving it.
#[must_use]
pub fn empty_document() -> serde_json::Value {
    serde_json::json!({
        "type": "doc",
        "content": SECTIONS
            .iter()
            .map(|section| serde_json::json!({
                "type": "paragraph",
                "attrs": { "section": section.id },
            }))
            .collect::<Vec<_>>(),
    })
}

/// The title a prompt starts with, before the user names it.
pub const UNTITLED: &str = "Untitled prompt";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_template_carries_eight_sections() {
        // FR-12 names them; the count is asserted so adding a ninth is a deliberate act
        // that updates the requirement rather than a quiet drift.
        assert_eq!(SECTIONS.len(), 8);
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
                "guardrails",
                "always",
                "never",
                "direction",
                "important",
                "output"
            ]
        );
    }

    #[test]
    fn every_section_has_a_heading_and_a_ghost() {
        for section in SECTIONS {
            assert!(!section.heading.is_empty(), "{}", section.id);
            assert!(!section.ghost.is_empty(), "{}", section.id);
        }
    }

    #[test]
    fn an_unknown_section_has_no_heading() {
        assert_eq!(heading_for("brief"), Some("Brief"));
        assert!(heading_for("epilogue").is_none());
    }

    #[test]
    fn a_new_document_is_a_tiptap_doc_with_one_paragraph_per_section() {
        let doc = empty_document();

        assert_eq!(doc["type"], "doc");
        let content = doc["content"].as_array().expect("content");
        assert_eq!(content.len(), 8);
        assert_eq!(content[0]["attrs"]["section"], "brief");
        assert_eq!(content[0]["type"], "paragraph");
    }

    #[test]
    fn ghost_text_is_not_in_the_document() {
        // The placeholder is a prompt to the author. Storing it as content would put
        // Curio's words in the user's brief and serialize them into the output.
        let doc = serde_json::to_string(&empty_document()).expect("serialize");
        for section in SECTIONS {
            assert!(!doc.contains(section.ghost), "{}", section.id);
        }
    }
}
