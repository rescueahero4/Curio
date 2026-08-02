//! The library's nouns.
//!
//! At scaffold time this module holds the vocabulary that is **already fixed by
//! contract** — the enums the database spells out as CHECK constraints (Inventory §4),
//! which an existing `library.db` will be validated against the moment it opens. Getting
//! one of these strings wrong is not a compile error; it is a constraint violation on a
//! real user's library, so they are written once here and reused everywhere.
//!
//! The record structs sit beside their enums: what the API publishes, what the sidecar
//! renders, and what the repositories read are one shape, so a column added in one place
//! cannot quietly fail to appear in the other two.

mod item;
mod job;
mod project;
mod prompt;
mod vocabulary;

pub use item::{Item, ItemFamily, ItemStatus, LastEditedBy};
pub use job::{Job, JobKind, JobStatus};
pub use project::{
    MARKER_FILE_NAME, MARKER_FINGERPRINT_PREFIX, Project, ProjectMarker, ProjectOrigin,
    ProjectStatus,
};
pub use prompt::{Prompt, SENT_CLAIM_WINDOW_SECONDS};
pub use vocabulary::{CreatedBy, Family, MAX_NAME_LEN, Term, VocabularyKind};

/// The largest number of items one bulk operation may touch.
///
/// Over-cap is a **named refusal** that reports what matched and what the limit is —
/// never a silent trim to the first 500 (R-BE-18, R-FE-11, Inventory §10.11). Trimming
/// would give the user a confident wrong answer about what they just changed.
pub const BULK_ITEM_CAP: usize = 500;
