//! Prompts: the composed briefs a user hands to their AI tool.

use serde::{Deserialize, Serialize};

/// How long a sent prompt's claim on the next project folder stays open.
///
/// The watcher only ever sees a directory appear; nothing in it names a prompt. "Send to
/// Claude" is the one moment both are known, so it stakes a claim the watcher spends on
/// the next folder it sees (R-BE-21, Inventory §8).
///
/// Six hours is long enough for a session that starts before lunch and produces a folder
/// after it, and short enough that a prompt sent yesterday does not adopt a project the
/// user made by hand this morning.
pub const SENT_CLAIM_WINDOW_SECONDS: i64 = 6 * 60 * 60;

/// A saved prompt.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Prompt {
    pub id: String,
    pub title: String,
    /// The TipTap document.
    ///
    /// Kept in TipTap's own shape rather than a normalized one because every existing
    /// vault already holds documents in it (D16). The editor round-trips this verbatim;
    /// nothing else parses it except the server-side serializer.
    ///
    /// A **value, not a string**, even though the column is TEXT. Publishing it as a
    /// JSON-encoded string would make every client parse a string out of a parsed
    /// document — and a client that forgot would hand TipTap a string, which it renders
    /// as literal text rather than as a document. `GET /api/prompts/template` returns a
    /// real object; these two had to agree.
    pub doc_json: serde_json::Value,
    /// The last serialization, produced **server-side and authoritatively** (R-FE-18).
    ///
    /// Stored so the prompts list and the MCP `prompt_get` tool can answer without
    /// re-running the serializer, and so a prompt whose referenced items were since
    /// deleted still reads as it did when it was sent.
    pub serialized_text: String,
    pub created_at: String,
    pub updated_at: String,
    /// When this prompt was last sent — millisecond precision, because it is an ordering
    /// key rather than a display value (R-DA-6, Inventory §10.15).
    pub sent_at: Option<String>,
}

impl Prompt {
    /// Whether this prompt still has an outstanding claim on the next project folder.
    ///
    /// `now` and `sent_at` are both ISO-8601 UTC and fixed-width, so the comparison the
    /// caller does upstream is textual; this function takes the already-parsed distance in
    /// seconds so the rule stays testable without a clock.
    #[must_use]
    pub fn claim_is_open(age_seconds: i64) -> bool {
        (0..SENT_CLAIM_WINDOW_SECONDS).contains(&age_seconds)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_claim_is_open() {
        assert!(Prompt::claim_is_open(0));
        assert!(Prompt::claim_is_open(60 * 60));
    }

    #[test]
    fn a_claim_expires_after_six_hours() {
        assert!(Prompt::claim_is_open(SENT_CLAIM_WINDOW_SECONDS - 1));
        assert!(!Prompt::claim_is_open(SENT_CLAIM_WINDOW_SECONDS));
    }

    #[test]
    fn a_claim_from_the_future_is_not_open() {
        // Clock skew, or a restored backup. Treating a negative age as "very fresh" would
        // let a prompt sent tomorrow adopt today's folder.
        assert!(!Prompt::claim_is_open(-1));
    }

    #[test]
    fn the_wire_shape_keeps_the_stored_column_names() {
        let prompt = Prompt {
            id: "01J".to_owned(),
            title: "Untitled prompt".to_owned(),
            doc_json: serde_json::json!({}),
            serialized_text: String::new(),
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            sent_at: None,
        };
        let json = serde_json::to_string(&prompt).expect("serialize");

        assert!(json.contains("doc_json"), "{json}");
        assert!(json.contains("serialized_text"), "{json}");
        assert!(json.contains("sent_at"), "{json}");
    }
}
