//! The MCP surface: how agents talk to the library.
//!
//! The inventory and the refusal shapes live here; [`server`] implements the handler and
//! [`library`] is the trait through which it reaches the library. This crate contains no
//! SQL and opens no database (R-MCP-13, D24).
//!
//! ## Seven tools, and why the number matters
//!
//! v1 exposes exactly the seven tools the previous implementation shipped, with their
//! names, schemas and behaviors preserved (R-MCP-1). Two semantic tools —
//! `library_semantic_search` and `library_related_items` — are designed but ship
//! **post-v1** with the vector layer, and if that layer's dependencies fail verification
//! they are **withheld from the tool list** rather than registered-but-erroring (R-MCP-2).
//!
//! Nine is a budget, not a floor to grow from. MCP clients inject every tool's schema into
//! model context on **every conversation**, so a verbose surface silently taxes every user
//! of every connected agent. A new tool has to argue it cannot be an argument on an
//! existing one (R-MCP-3).
//!
//! ## Two doors, one pipeline
//!
//! HTTP at `/mcp` inside the main server, and `curio --mcp-stdio` for Claude Desktop. The
//! stdio transport is a **thin proxy** that forwards frames to the live instance's `/mcp`
//! and never opens the database (D24) — which is what preserves the single-writer
//! invariant and the event fan-out by construction rather than by discipline. It also
//! means the `mcpEnabled` toggle gates both transports through one gate: the proxy
//! receives the same 503 and passes it through unchanged (R-MCP-6).
//!
//! ## Blast radius
//!
//! CVE-2026-42559's lesson is that a local MCP server's exposure equals whatever its tools
//! can do. So: every tool calls `curio-core` service functions — the same paths REST uses,
//! with the same validation, threshold logic, event emission and sidecar write-back — and
//! this crate contains no SQL and bypasses no invariant (R-MCP-13). Read tools keep
//! working while the app is paused; write tools refuse cleanly (R-MCP-10).

pub mod library;
pub mod proxy;
pub mod server;

pub use library::Library;
pub use server::Curio;

use serde::Serialize;

/// Whether a tool changes anything.
///
/// The distinction is enforced at **dispatch**, not in middleware: the paused middleware
/// passes `/mcp` through untouched because only the dispatcher can tell a read from a
/// write (R-MCP-10). Paused means paused for MCP writes — consultation of the library is
/// never interrupted by the pause toggle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Access {
    Read,
    Write,
}

/// One tool in the surface.
#[derive(Debug, Clone, Copy)]
pub struct Tool {
    pub name: &'static str,
    pub access: Access,
}

/// The v1 surface: exactly seven tools (R-MCP-1, D15).
pub const V1_TOOLS: &[Tool] = &[
    Tool {
        name: "library_search",
        access: Access::Read,
    },
    Tool {
        name: "library_get_item",
        access: Access::Read,
    },
    Tool {
        name: "library_list_vocabulary",
        access: Access::Read,
    },
    Tool {
        name: "library_create_item",
        access: Access::Write,
    },
    Tool {
        name: "library_update_item",
        access: Access::Write,
    },
    Tool {
        name: "prompt_get",
        access: Access::Read,
    },
    Tool {
        name: "project_register",
        access: Access::Write,
    },
];

/// The two semantic tools, listed for the record and **not registered in v1**.
///
/// They ship with the vector layer's activation release (R-MCP-2, D7 as amended). Both are
/// read-only by construction, so they remain available while paused.
pub const POST_V1_TOOLS: &[Tool] = &[
    Tool {
        name: "library_semantic_search",
        access: Access::Read,
    },
    Tool {
        name: "library_related_items",
        access: Access::Read,
    },
];

/// The JSON-RPC error code every refusal here uses.
pub const JSON_RPC_ERROR_CODE: i32 = -32000;

/// Why a request was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    /// The Settings toggle is off. Read **per request**, never cached at router
    /// construction, so flipping it takes effect immediately (R-MCP-8).
    McpDisabled,
    /// The app is paused and this is a write tool (R-MCP-10).
    Paused,
    /// The stdio proxy found no live instance to forward to.
    ///
    /// A clean error rather than a hang, and never a direct database open — a second
    /// process opening the library would fork the single-writer invariant (D24, R-MCP-5).
    NotRunning,
}

impl Refusal {
    /// The machine-readable reason, as it appears in the error's `data`.
    #[must_use]
    pub fn reason(self) -> &'static str {
        match self {
            Refusal::McpDisabled => "mcp_disabled",
            Refusal::Paused => "paused",
            Refusal::NotRunning => "not_running",
        }
    }

    /// The message a user reads.
    #[must_use]
    pub fn message(self) -> &'static str {
        match self {
            Refusal::McpDisabled => {
                "Curio's MCP server is turned off. Enable it in Curio's Settings."
            }
            Refusal::Paused => {
                "Curio is paused, so it is not accepting changes. Reading the library still works."
            }
            Refusal::NotRunning => "Curio isn't running. Start Curio and try again.",
        }
    }

    /// The JSON-RPC error body.
    #[must_use]
    pub fn to_json(self) -> serde_json::Value {
        serde_json::json!({
            "code": JSON_RPC_ERROR_CODE,
            "message": self.message(),
            "data": { "reason": self.reason() },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_exposes_exactly_seven_tools() {
        // R-MCP-1 / D15. Every registered tool's schema is injected into model context on
        // every conversation with every connected agent — the count is a budget with a
        // real cost, not a coincidence.
        assert_eq!(V1_TOOLS.len(), 7);
    }

    #[test]
    fn the_v1_tool_names_are_the_shipped_ones() {
        // Inventory §2. These names are what existing agent configurations call.
        let names: Vec<&str> = V1_TOOLS.iter().map(|tool| tool.name).collect();
        assert_eq!(
            names,
            [
                "library_search",
                "library_get_item",
                "library_list_vocabulary",
                "library_create_item",
                "library_update_item",
                "prompt_get",
                "project_register",
            ]
        );
    }

    #[test]
    fn the_semantic_pair_is_not_in_the_v1_surface() {
        // R-MCP-2: withheld, not registered-and-erroring. An agent should not see a tool
        // it cannot use.
        for tool in POST_V1_TOOLS {
            assert!(
                !V1_TOOLS.iter().any(|v1| v1.name == tool.name),
                "{} must not ship in v1",
                tool.name
            );
        }
    }

    #[test]
    fn the_query_tools_are_read_only_by_construction() {
        // R-SEC-13: the blast radius of the MCP surface is whatever its tools can do.
        for name in [
            "library_search",
            "library_get_item",
            "library_list_vocabulary",
            "prompt_get",
        ] {
            let tool = V1_TOOLS.iter().find(|t| t.name == name).expect(name);
            assert_eq!(tool.access, Access::Read, "{name}");
        }
    }

    #[test]
    fn every_post_v1_tool_is_read_only() {
        // R-MCP-2 requires it, which is also why they stay available while paused.
        for tool in POST_V1_TOOLS {
            assert_eq!(tool.access, Access::Read, "{}", tool.name);
        }
    }

    #[test]
    fn refusals_carry_a_machine_readable_reason() {
        // Clients branch on `data.reason`; the message is for the human reading the
        // agent's output.
        let disabled = Refusal::McpDisabled.to_json();
        assert_eq!(disabled["code"], JSON_RPC_ERROR_CODE);
        assert_eq!(disabled["data"]["reason"], "mcp_disabled");
        assert_eq!(Refusal::Paused.to_json()["data"]["reason"], "paused");
    }

    #[test]
    fn the_paused_message_says_reading_still_works() {
        // D25 is a user-visible promise, and this string is where the user meets it.
        assert!(Refusal::Paused.message().contains("Reading"));
    }
}
