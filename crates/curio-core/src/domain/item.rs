//! Items: the captured references the library is made of.

use serde::{Deserialize, Serialize};

/// Where an item is in its life.
///
/// The values are the `items.status` CHECK constraint, verbatim (Inventory §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ItemStatus {
    /// Captured, assessment not finished. Visible in the grid **immediately** — a capture
    /// appears as a card the instant it lands, rather than after the model replies (FR-3).
    Processing,
    /// Assessed and settled.
    Ready,
    /// Assessed, but a human has to decide something.
    ///
    /// Almost always the gray zone: the best family score fell between the two configured
    /// thresholds, so the model is neither confident enough to assign nor confident enough
    /// to reject (FR-6, FR-7).
    NeedsReview,
    /// Assessment failed after exhausting its attempts. The item and its screenshot are
    /// intact — only the enrichment is missing, and re-assess is one click.
    AssessmentFailed,
}

impl ItemStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            ItemStatus::Processing => "processing",
            ItemStatus::Ready => "ready",
            ItemStatus::NeedsReview => "needs_review",
            ItemStatus::AssessmentFailed => "assessment_failed",
        }
    }

    #[must_use]
    pub const fn all() -> [ItemStatus; 4] {
        [
            ItemStatus::Processing,
            ItemStatus::Ready,
            ItemStatus::NeedsReview,
            ItemStatus::AssessmentFailed,
        ]
    }
}

/// Who last touched an item's fields.
///
/// Load-bearing rather than informational: a re-assessment **must not overwrite a name the
/// user chose**. The stamping rules differ per path and are a documented invariant table
/// (Inventory §10.12) — a single PATCH stamps `User`, a bulk operation touches only
/// `updated_at`, an MCP update stamps `Ai`, and a failed assessment preserves whatever was
/// there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LastEditedBy {
    Ai,
    User,
}

impl LastEditedBy {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            LastEditedBy::Ai => "ai",
            LastEditedBy::User => "user",
        }
    }
}

/// One item's link to one aesthetic family.
///
/// Carries more than a join because the link itself holds state a user acts on: the score
/// the model gave, whether it is awaiting a decision, and whether the family exists only
/// because this item proposed it (FR-6, FR-7).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ItemFamily {
    pub id: String,
    pub name: String,
    pub score: f64,
    pub gray_zone: bool,
    pub ai_proposed: bool,
}

/// A library item, with its vocabulary joined in.
///
/// Serialized field-for-field as the API and the sidecar present it. The names are
/// `snake_case` because that is what the previous implementation published and what the
/// extension, the MCP tools, and every existing sidecar already read (Inventory §1, §2).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub name: String,
    pub short_description: String,
    pub source_url: Option<String>,
    pub image_recipe: Option<String>,
    /// **Relative to the data root**, never absolute. The root is configurable and the
    /// library is meant to survive being moved; an absolute path stored here would break
    /// every item the moment it did (R-DA-1).
    pub screenshot_path: String,
    pub thumbnail_path: Option<String>,
    pub status: ItemStatus,
    pub last_edited_by: LastEditedBy,
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
    pub design_types: Vec<String>,
    pub tags: Vec<String>,
    pub families: Vec<ItemFamily>,
}

impl Item {
    /// Whether the item is waiting on a human decision.
    ///
    /// Two conditions, not one: an item held at `needs_review` and an item carrying a
    /// gray-zone link. They usually coincide, but a threshold change can leave a
    /// `ready` item with a link now inside the gray band, and the badge has to follow the
    /// data rather than the status column.
    #[must_use]
    pub fn needs_review(&self) -> bool {
        self.status == ItemStatus::NeedsReview || self.families.iter().any(|link| link.gray_zone)
    }

    /// The family this item matches best, if any.
    #[must_use]
    pub fn nearest_family(&self) -> Option<&ItemFamily> {
        self.families
            .iter()
            .max_by(|a, b| a.score.total_cmp(&b.score))
    }

    /// The text an embedding would cover, and what the search index stores (R-DA-10).
    #[must_use]
    pub fn searchable_tags(&self) -> String {
        self.tags.join(" ")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item() -> Item {
        Item {
            id: "01J".to_owned(),
            name: "Stripe pricing".to_owned(),
            short_description: "A clean pricing table".to_owned(),
            source_url: None,
            image_recipe: None,
            screenshot_path: "items/01J/screenshot.png".to_owned(),
            thumbnail_path: None,
            status: ItemStatus::Ready,
            last_edited_by: LastEditedBy::Ai,
            error: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            design_types: vec!["pricing page".to_owned()],
            tags: vec!["saas".to_owned(), "minimal".to_owned()],
            families: Vec::new(),
        }
    }

    fn link(name: &str, score: f64, gray_zone: bool) -> ItemFamily {
        ItemFamily {
            id: format!("fam-{name}"),
            name: name.to_owned(),
            score,
            gray_zone,
            ai_proposed: false,
        }
    }

    #[test]
    fn a_ready_item_with_a_gray_link_still_needs_review() {
        // The case a status-only check gets wrong: lowering a threshold in Settings puts
        // an already-`ready` item's link back inside the gray band without touching the
        // status column, and the badge has to follow the data.
        let mut subject = item();
        subject.families = vec![link("Brutalist", 0.45, true)];

        assert!(subject.needs_review());
    }

    #[test]
    fn a_settled_item_needs_no_review() {
        let mut subject = item();
        subject.families = vec![link("Brutalist", 0.9, false)];

        assert!(!subject.needs_review());
    }

    #[test]
    fn the_nearest_family_is_the_highest_scoring_one() {
        let mut subject = item();
        subject.families = vec![link("Editorial", 0.6, false), link("Brutalist", 0.9, false)];

        assert_eq!(
            subject.nearest_family().map(|f| f.name.as_str()),
            Some("Brutalist")
        );
    }

    #[test]
    fn an_item_with_no_families_has_no_nearest_one() {
        assert!(item().nearest_family().is_none());
    }

    #[test]
    fn the_wire_shape_is_snake_case() {
        // The extension, the MCP tools, and every sidecar already on disk read these
        // names. Renaming one is a breaking change to three consumers at once.
        let json = serde_json::to_string(&item()).expect("serialize");

        assert!(json.contains("short_description"), "{json}");
        assert!(json.contains("screenshot_path"), "{json}");
        assert!(json.contains("last_edited_by"), "{json}");
    }

    #[test]
    fn the_screenshot_path_stays_relative() {
        // R-DA-1. The data root is configurable and libraries get moved; an absolute path
        // stored here breaks every item the moment one is.
        assert!(!item().screenshot_path.starts_with('/'));
        assert!(!item().screenshot_path.contains(':'));
    }

    #[test]
    fn status_strings_match_the_check_constraint() {
        // items.status CHECK (status IN ('processing','ready','needs_review','assessment_failed'))
        // A mismatch here is not a compile error — it is a constraint violation against a
        // real user's library at write time.
        let actual: Vec<&str> = ItemStatus::all().iter().map(|s| s.as_str()).collect();
        assert_eq!(
            actual,
            ["processing", "ready", "needs_review", "assessment_failed"]
        );
    }

    #[test]
    fn status_serializes_as_the_stored_string() {
        assert_eq!(
            serde_json::to_string(&ItemStatus::NeedsReview).unwrap(),
            "\"needs_review\""
        );
    }

    #[test]
    fn authorship_strings_match_the_check_constraint() {
        // items.last_edited_by CHECK (last_edited_by IN ('ai','user'))
        assert_eq!(LastEditedBy::Ai.as_str(), "ai");
        assert_eq!(LastEditedBy::User.as_str(), "user");
    }
}
