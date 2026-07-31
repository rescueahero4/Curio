//! Item status and authorship.

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

#[cfg(test)]
mod tests {
    use super::*;

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
