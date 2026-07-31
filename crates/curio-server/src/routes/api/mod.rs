//! `/api/*` — the surface the dashboard, the extension, and any scripted client talk to.
//!
//! Every module here is **thin by contract** (R-DEL-2): parse, call `curio-core` or
//! `curio-db`, serialize, announce. Nothing in this directory decides anything. When a
//! handler looks like it is about to — a threshold applied, a name resolved, a merge
//! reconciled — that logic belongs one layer down, where it is tested without a router and
//! shared with the MCP tools that expose the same operations.

pub mod ingest;
pub mod items;
pub mod projects;
pub mod prompts;
pub mod settings;
pub mod system;
pub mod vocabulary;
