//! The library's nouns.
//!
//! At scaffold time this module holds the vocabulary that is **already fixed by
//! contract** — the enums the database spells out as CHECK constraints (Inventory §4),
//! which an existing `library.db` will be validated against the moment it opens. Getting
//! one of these strings wrong is not a compile error; it is a constraint violation on a
//! real user's library, so they are written once here and reused everywhere.
//!
//! The record structs (`Item`, `Prompt`, `Project`, `Job`) land with P2, when the
//! repositories that read them exist. Defining them now would mean guessing at
//! serialization boundaries the API surface has not yet had to satisfy.

mod item;
mod job;
mod project;
mod vocabulary;

pub use item::{ItemStatus, LastEditedBy};
pub use job::{JobKind, JobStatus};
pub use project::{MARKER_FILE_NAME, MARKER_FINGERPRINT_PREFIX, ProjectOrigin, ProjectStatus};
pub use vocabulary::{CreatedBy, MAX_NAME_LEN, VocabularyKind};

/// The largest number of items one bulk operation may touch.
///
/// Over-cap is a **named refusal** that reports what matched and what the limit is —
/// never a silent trim to the first 500 (R-BE-18, R-FE-11, Inventory §10.11). Trimming
/// would give the user a confident wrong answer about what they just changed.
pub const BULK_ITEM_CAP: usize = 500;

/// The number of items at which a bulk retag switches from serial calls to the Batch API.
///
/// Below it, serial work is resumable through `progress.done` and finishes sooner. At or
/// above it, batching is dramatically cheaper (R-BE-18).
pub const BULK_BATCH_THRESHOLD: usize = 8;

/// How many times a job may fail before it is abandoned.
///
/// Counted only against real failures: a missing API key requeues without consuming an
/// attempt, and parking refunds one (R-BE-17, Inventory §10.10).
pub const JOB_MAX_ATTEMPTS: u32 = 3;
