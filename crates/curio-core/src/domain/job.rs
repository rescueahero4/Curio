//! Job kinds and states.

use serde::{Deserialize, Serialize};

/// What a queued job will do.
///
/// The three v1 kinds are the ones the previous implementation shipped. `embed_item` is
/// designed in but **not registered in v1**: vector and graph activate post-v1 (D7), and
/// when they do, the kind reuses this machinery unchanged — parking covers rate limits,
/// the missing-key refund covers offline queueing (R-BE-16, R-DA-13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobKind {
    /// One structured vision call describing one screenshot (FR-4).
    AssessItem,
    /// Re-tag a frozen set of items, serial or via the Batch API depending on size.
    BulkRetag,
    /// Ask a utility model which vocabulary entries are duplicates.
    ///
    /// The result is **stored and shown, never auto-applied** — merges are the user's call,
    /// applied per group (Inventory §10.8, R-FE-15a).
    VocabDedupe,
}

impl JobKind {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            JobKind::AssessItem => "assess_item",
            JobKind::BulkRetag => "bulk_retag",
            JobKind::VocabDedupe => "vocab_dedupe",
        }
    }

    #[must_use]
    pub const fn all() -> [JobKind; 3] {
        [
            JobKind::AssessItem,
            JobKind::BulkRetag,
            JobKind::VocabDedupe,
        ]
    }
}

/// Where a job is in the worker's state machine.
///
/// The values are the `jobs.status` CHECK constraint, verbatim (Inventory §4).
///
/// Parking is deliberately **not** a state here. A parked job is `Queued` with a
/// `not_before` timestamp, which is what lets one claim query — "oldest queued job whose
/// `not_before` is null or due" — serve both ordinary and parked work (R-BE-16).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus {
    Queued,
    Running,
    Done,
    Failed,
    Cancelled,
}

impl JobStatus {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            JobStatus::Queued => "queued",
            JobStatus::Running => "running",
            JobStatus::Done => "done",
            JobStatus::Failed => "failed",
            JobStatus::Cancelled => "cancelled",
        }
    }

    #[must_use]
    pub const fn all() -> [JobStatus; 5] {
        [
            JobStatus::Queued,
            JobStatus::Running,
            JobStatus::Done,
            JobStatus::Failed,
            JobStatus::Cancelled,
        ]
    }

    /// Whether the worker is finished with this job for good.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            JobStatus::Done | JobStatus::Failed | JobStatus::Cancelled
        )
    }
}

/// A row in the queue.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Job {
    pub id: String,
    pub kind: String,
    /// The job's input, frozen at enqueue.
    ///
    /// A bulk operation stores **ids**, never a filter (R-BE-18). Re-resolving the filter
    /// at run time would let a job silently change what it operates on between the moment
    /// the user pressed the button and the moment the worker got to it.
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub attempts: u32,
    pub error: Option<String>,
    /// Progress while running, summary when finished.
    pub result: Option<serde_json::Value>,
    /// Set while parked. A parked job is `Queued` with a timer, not a fourth state.
    pub not_before: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kind_strings_match_the_stored_values() {
        let actual: Vec<&str> = JobKind::all().iter().map(|k| k.as_str()).collect();
        assert_eq!(actual, ["assess_item", "bulk_retag", "vocab_dedupe"]);
    }

    #[test]
    fn status_strings_match_the_check_constraint() {
        // jobs.status CHECK (status IN ('queued','running','done','failed','cancelled'))
        let actual: Vec<&str> = JobStatus::all().iter().map(|s| s.as_str()).collect();
        assert_eq!(actual, ["queued", "running", "done", "failed", "cancelled"]);
    }

    #[test]
    fn only_finished_states_are_terminal() {
        // Startup reclaims orphaned `running` jobs back to `queued` (R-BE-19), so
        // "running" must never read as done.
        assert!(!JobStatus::Queued.is_terminal());
        assert!(!JobStatus::Running.is_terminal());
        assert!(JobStatus::Done.is_terminal());
        assert!(JobStatus::Failed.is_terminal());
        assert!(JobStatus::Cancelled.is_terminal());
    }

    #[test]
    fn embed_item_is_not_registered_in_v1() {
        // D7: the vector layer is designed in and deferred. If someone adds the kind
        // before the activation release, this test names the decision they are crossing.
        assert_eq!(JobKind::all().len(), 3);
        assert!(!JobKind::all().iter().any(|k| k.as_str() == "embed_item"));
    }
}
