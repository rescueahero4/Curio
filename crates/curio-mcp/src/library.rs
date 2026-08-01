//! What the tools are allowed to do, as a trait rather than a database handle.
//!
//! This crate contains **no SQL and opens no database** (R-MCP-13, D24). Every tool calls
//! through here, and `curio-server` supplies the implementation over the same service
//! functions REST uses — same validation, same threshold logic, same event emission, same
//! sidecar write-back.
//!
//! That is not tidiness. CVE-2026-42559's lesson is that a local MCP server's exposure
//! equals whatever its tools can do, so the surface an agent can reach has to be exactly
//! the surface a user can reach and no wider. A trait makes that checkable: if a tool wants
//! something REST cannot do, it needs a method here, and adding one is a visible act.
//!
//! It also keeps the single-writer invariant by construction. There is one database
//! connection, in one process, behind one mutex — and the only way this crate could break
//! that is by opening its own, which it has no means to do.

use serde_json::Value;

/// Everything the seven tools need.
///
/// Synchronous because every implementation is a database call behind a mutex, and an async
/// wrapper around a blocking lock buys nothing but the illusion of concurrency.
pub trait Library: Send + Sync + 'static {
    /// Whether the MCP surface is switched on.
    ///
    /// Read **per request**, never cached at construction: a user who turns MCP off in
    /// Settings expects the next call to be refused, not the next restart (R-MCP-8).
    fn mcp_enabled(&self) -> bool;

    /// Whether mutations are currently refused (D25).
    fn is_paused(&self) -> bool;

    /// # Errors
    /// Propagates whatever the underlying service function refused or failed with.
    fn search(&self, arguments: &Value) -> curio_core::Result<Value>;

    /// # Errors
    /// See [`Library::search`].
    fn get_item(&self, arguments: &Value) -> curio_core::Result<Value>;

    /// # Errors
    /// See [`Library::search`].
    fn list_vocabulary(&self) -> curio_core::Result<Value>;

    /// # Errors
    /// See [`Library::search`].
    fn create_item(&self, arguments: &Value) -> curio_core::Result<Value>;

    /// # Errors
    /// See [`Library::search`].
    fn update_item(&self, arguments: &Value) -> curio_core::Result<Value>;

    /// # Errors
    /// See [`Library::search`].
    fn prompt_get(&self, arguments: &Value) -> curio_core::Result<Value>;

    /// # Errors
    /// See [`Library::search`].
    fn project_register(&self, arguments: &Value) -> curio_core::Result<Value>;
}
