//! The three vocabularies items are described with.

use serde::{Deserialize, Serialize};

/// Which vocabulary a name belongs to.
///
/// All three behave alike for CRUD and merging, but they are stored in separate tables
/// with different link semantics, and only families carry a score. This enum exists so the
/// shared code paths — merge, prune, the FTS rebuild — can be written once rather than
/// three times with two of them subtly wrong.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VocabularyKind {
    /// An aesthetic family: the library's own named look, with a description in a fixed
    /// format the vision model is shown as part of its rubric.
    ///
    /// Unlike tags and types, a family link carries a **score** and may sit in the gray
    /// zone awaiting a human decision (FR-6, FR-7).
    Family,
    /// What kind of thing the screenshot is — a landing page, a dashboard, a pricing table.
    DesignType,
    /// A free descriptor. Stored `COLLATE NOCASE`, so "Brutalist" and "brutalist" are one
    /// tag rather than two — a distinction no user ever intends to draw.
    Tag,
}

impl VocabularyKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            VocabularyKind::Family => "family",
            VocabularyKind::DesignType => "design_type",
            VocabularyKind::Tag => "tag",
        }
    }

    /// The table this vocabulary lives in.
    #[must_use]
    pub fn table(self) -> &'static str {
        match self {
            VocabularyKind::Family => "aesthetic_families",
            VocabularyKind::DesignType => "design_types",
            VocabularyKind::Tag => "tags",
        }
    }

    /// The link table joining this vocabulary to items.
    #[must_use]
    pub fn link_table(self) -> &'static str {
        match self {
            VocabularyKind::Family => "item_families",
            VocabularyKind::DesignType => "item_types",
            VocabularyKind::Tag => "item_tags",
        }
    }

    #[must_use]
    pub const fn all() -> [VocabularyKind; 3] {
        [
            VocabularyKind::Family,
            VocabularyKind::DesignType,
            VocabularyKind::Tag,
        ]
    }
}

/// Whether a vocabulary entry was proposed by the model or created by the user.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CreatedBy {
    Ai,
    User,
}

impl CreatedBy {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            CreatedBy::Ai => "ai",
            CreatedBy::User => "user",
        }
    }
}

/// The longest a cleaned vocabulary name may be.
///
/// Model output is trimmed and length-capped before it becomes a row, because a model
/// occasionally answers a request for a tag with a sentence, and a sentence in the tag
/// filter is permanent clutter (Inventory §9, `cleanVocabularyNames`).
pub const MAX_NAME_LEN: usize = 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_match_the_shipped_schema() {
        // These names are compared against a real library.db in the migration round-trip
        // test; getting one wrong surfaces as "no such table" at query time, not here.
        assert_eq!(VocabularyKind::Family.table(), "aesthetic_families");
        assert_eq!(VocabularyKind::DesignType.table(), "design_types");
        assert_eq!(VocabularyKind::Tag.table(), "tags");
    }

    #[test]
    fn link_tables_match_the_shipped_schema() {
        assert_eq!(VocabularyKind::Family.link_table(), "item_families");
        assert_eq!(VocabularyKind::DesignType.link_table(), "item_types");
        assert_eq!(VocabularyKind::Tag.link_table(), "item_tags");
    }

    #[test]
    fn every_kind_has_a_distinct_pair_of_tables() {
        let tables: std::collections::HashSet<&str> =
            VocabularyKind::all().iter().map(|k| k.table()).collect();
        let links: std::collections::HashSet<&str> = VocabularyKind::all()
            .iter()
            .map(|k| k.link_table())
            .collect();

        assert_eq!(tables.len(), 3);
        assert_eq!(links.len(), 3);
    }
}
