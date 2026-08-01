//! Curio's domain: the types and rules that decide things.
//!
//! Everything a reviewer would call "a decision Curio makes" belongs here — which family
//! an assessment lands an item in, how a job's attempts are counted, what a serialized
//! prompt looks like. The layers above are deliberately thin: `curio-server` routes and
//! serializes, `curio-mcp` maps tool calls onto these same functions, and `curio-db`
//! stores what they produce. None of them may re-decide anything.
//!
//! That split is a contract, not a preference (R-DEL-2). This crate depends on no SQL
//! library, no HTTP framework, and no MCP SDK, and `cargo gate` asserts it against the
//! real dependency graph on every run. The point is to keep three seams cuttable: the
//! tray/service split, the storage trait, and the isolation of `curio-nmh`.
//!
//! ## What is here, and what is deliberately not
//!
//! At scaffold time this crate holds the vocabulary that is already fixed by contract —
//! the status enums the database's CHECK constraints spell out, the event names the SSE
//! stream promises, the config shape, ULID and timestamp generation. The rules that
//! decide things (threshold logic, prompt serialization, sidecar rendering) land with
//! their phases; each has a module here with the rule it must satisfy recorded in its
//! documentation, so the shape of the work is visible before the work exists.

pub mod ai;
pub mod assessment;
pub mod config;
pub mod domain;
pub mod error;
pub mod events;
pub mod ids;
pub mod nm;
pub mod paths;
pub mod ports;
pub mod prompt;
pub mod query;
pub mod sidecar;
pub mod time;
pub mod variants;

pub use error::{Error, Result};

/// The single stamped workspace version (R-DEL-12).
///
/// Three surfaces report a version — `curio --version`, `/health`, and `runtime.json` —
/// and all three read it from here. A mismatch between any two is a release-blocking bug,
/// which is only preventable if there is one place to read.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
