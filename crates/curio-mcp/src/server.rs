//! The `ServerHandler`: seven tools, two gates, one dispatch.
//!
//! ## Both gates are checked per call, not per connection
//!
//! `mcpEnabled` is read on **every request** (R-MCP-8) — a user who turns MCP off in
//! Settings expects the next call refused, not the next restart. And the paused check
//! happens at **dispatch**, not in middleware, because only the dispatcher can tell a read
//! from a write: the pause toggle stops writes and leaves consultation of the library
//! working (R-MCP-10, D25). A middleware that 503'd the whole `/mcp` path would take
//! `library_search` down with `library_create_item`, which is precisely the wrong half.
//!
//! ## The schemas are hand-written
//!
//! Seven small objects, written out rather than derived. MCP clients inject every tool's
//! schema into model context on **every conversation**, so the wording is a product surface
//! that a `#[derive]` would hide behind field names (R-MCP-3).

use std::borrow::Cow;
use std::sync::Arc;

use rmcp::ErrorData as McpError;
use rmcp::handler::server::ServerHandler;
use rmcp::model::{
    CallToolRequestParams, CallToolResult, ContentBlock, Implementation, ListToolsResult,
    PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool,
};
use rmcp::service::RequestContext;
use rmcp::{RoleServer, model::CallToolResponse};

use crate::library::Library;
use crate::{Access, JSON_RPC_ERROR_CODE, Refusal, V1_TOOLS};

/// The MCP server, over whatever supplies the library.
pub struct Curio<L: Library> {
    library: Arc<L>,
}

impl<L: Library> Curio<L> {
    pub fn new(library: Arc<L>) -> Self {
        Self { library }
    }
}

/// The description an agent reads before deciding whether to call a tool.
///
/// Longer than a name and shorter than documentation, because it is injected into context
/// on every conversation and is the only thing standing between a useful call and a wasted
/// one.
fn describe(name: &str) -> &'static str {
    match name {
        "library_search" => {
            "Search the user's design-inspiration library. Filter by design type, aesthetic \
             family, or tag, and optionally by free text. Returns matching items with their \
             vocabulary and family scores."
        }
        "library_get_item" => {
            "Get one library item in full, including absolute paths to its screenshot and \
             its Markdown sidecar."
        }
        "library_list_vocabulary" => {
            "List the library's aesthetic families (with descriptions and item counts), \
             design types, and tags. Call this before searching so filters use terms the \
             library actually has."
        }
        "library_create_item" => {
            "Add a screenshot already on disk to the library. The file is copied in and an \
             assessment is queued; the item appears immediately as `processing`."
        }
        "library_update_item" => {
            "Change an item's name, description, tags, or design types. Edits are recorded \
             as made by AI."
        }
        "prompt_get" => {
            "Get a saved prompt, serialized with its referenced items, families, and tags \
             expanded into full context."
        }
        "project_register" => {
            "Register a directory as a Curio project so it appears in the Projects list and \
             can be served and linked to a prompt."
        }
        _ => "",
    }
}

/// The input schema for a tool, as the object rmcp wants.
fn schema_for(name: &str) -> serde_json::Map<String, serde_json::Value> {
    let schema = match name {
        "library_search" => serde_json::json!({
            "type": "object",
            "properties": {
                "query": { "type": "string", "description": "Free-text search over names, descriptions, and tags." },
                "design_types": { "type": "array", "items": { "type": "string" }, "description": "Design-type names. Unknown names are ignored rather than erroring." },
                "families": { "type": "array", "items": { "type": "string" }, "description": "Aesthetic family names." },
                "tags": { "type": "array", "items": { "type": "string" } },
                "limit": { "type": "integer", "description": "Maximum items to return, 1-50. Defaults to 20." }
            },
            "additionalProperties": false
        }),
        "library_get_item" => serde_json::json!({
            "type": "object",
            "properties": { "item_id": { "type": "string" } },
            "required": ["item_id"],
            "additionalProperties": false
        }),
        "library_list_vocabulary" => serde_json::json!({
            "type": "object", "properties": {}, "additionalProperties": false
        }),
        "library_create_item" => serde_json::json!({
            "type": "object",
            "properties": {
                "screenshot_path": { "type": "string", "description": "Absolute path to an image file that already exists." },
                "source_url": { "type": "string" },
                "name": { "type": "string", "description": "Optional. The assessment will suggest one if omitted." }
            },
            "required": ["screenshot_path"],
            "additionalProperties": false
        }),
        "library_update_item" => serde_json::json!({
            "type": "object",
            "properties": {
                "item_id": { "type": "string" },
                "name": { "type": "string" },
                "short_description": { "type": "string" },
                "tags": { "type": "array", "items": { "type": "string" } },
                "design_types": { "type": "array", "items": { "type": "string" } }
            },
            "required": ["item_id"],
            "additionalProperties": false
        }),
        "prompt_get" => serde_json::json!({
            "type": "object",
            "properties": { "prompt_id": { "type": "string" } },
            "required": ["prompt_id"],
            "additionalProperties": false
        }),
        "project_register" => serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "Absolute path to an existing directory." },
                "name": { "type": "string" }
            },
            "required": ["path"],
            "additionalProperties": false
        }),
        _ => serde_json::json!({ "type": "object", "properties": {} }),
    };

    schema
        .as_object()
        .cloned()
        .unwrap_or_else(serde_json::Map::new)
}

/// Turn a domain error into a JSON-RPC refusal an agent can read.
fn refuse(reason: &str) -> McpError {
    McpError::new(
        rmcp::model::ErrorCode(JSON_RPC_ERROR_CODE),
        reason.to_owned(),
        None,
    )
}

impl<L: Library> ServerHandler for Curio<L> {
    fn get_info(&self) -> ServerInfo {
        let mut identity = Implementation::from_build_env();
        identity.name = "curio".into();
        identity.version = curio_core::VERSION.into();

        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(identity)
            // Read by the agent once, at connect. Naming the ordering here saves a wasted
            // search against vocabulary the library does not have.
            .with_instructions(
                "Curio is the user's personal design-inspiration library. Call \
                 library_list_vocabulary first so filters use terms the library actually \
                 has, then library_search to find references.",
            )
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        // Read per request (R-MCP-8). A disabled surface lists nothing rather than listing
        // tools that would then refuse — an agent that sees a tool will eventually call it.
        if !self.library.mcp_enabled() {
            return Err(refuse(Refusal::McpDisabled.message()));
        }

        Ok(ListToolsResult {
            tools: V1_TOOLS
                .iter()
                .map(|tool| {
                    Tool::new(
                        Cow::Borrowed(tool.name),
                        Cow::Borrowed(describe(tool.name)),
                        Arc::new(schema_for(tool.name)),
                    )
                })
                .collect(),
            ..ListToolsResult::default()
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResponse, McpError> {
        if !self.library.mcp_enabled() {
            return Err(refuse(Refusal::McpDisabled.message()));
        }

        let name = request.name.as_ref();
        let Some(tool) = V1_TOOLS.iter().find(|candidate| candidate.name == name) else {
            return Err(refuse(&format!("Curio has no tool called {name}")));
        };

        // The gate that must live here rather than in middleware: only dispatch knows
        // whether this particular call writes (R-MCP-10).
        if tool.access == Access::Write && self.library.is_paused() {
            return Err(refuse(Refusal::Paused.message()));
        }

        let arguments = serde_json::Value::Object(request.arguments.unwrap_or_default());

        let outcome = match name {
            "library_search" => self.library.search(&arguments),
            "library_get_item" => self.library.get_item(&arguments),
            "library_list_vocabulary" => self.library.list_vocabulary(),
            "library_create_item" => self.library.create_item(&arguments),
            "library_update_item" => self.library.update_item(&arguments),
            "prompt_get" => self.library.prompt_get(&arguments),
            "project_register" => self.library.project_register(&arguments),
            _ => unreachable!("the tool was found in V1_TOOLS above"),
        };

        match outcome {
            Ok(value) => {
                let text = serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{}".to_owned());
                Ok(CallToolResult::success(vec![ContentBlock::text(text)]).into())
            }
            // A refusal the user could act on — a missing item, a path outside the jail —
            // is returned as an error rather than an empty success, so the agent stops
            // rather than reporting that it did something.
            Err(err) => Err(refuse(&err.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_v1_tool_has_a_description_and_a_schema() {
        // Both are injected into model context on every conversation. A blank description
        // is a tool an agent will not call; a missing schema is one it will call wrongly.
        for tool in V1_TOOLS {
            assert!(!describe(tool.name).is_empty(), "{}", tool.name);

            let schema = schema_for(tool.name);
            assert_eq!(
                schema.get("type").and_then(serde_json::Value::as_str),
                Some("object"),
                "{}",
                tool.name
            );
        }
    }

    #[test]
    fn every_schema_that_takes_arguments_names_its_required_ones() {
        // A tool whose only argument is optional is a tool that gets called with nothing.
        for name in [
            "library_get_item",
            "library_create_item",
            "library_update_item",
            "prompt_get",
            "project_register",
        ] {
            let schema = schema_for(name);
            let required = schema
                .get("required")
                .and_then(serde_json::Value::as_array)
                .map(Vec::len)
                .unwrap_or_default();
            assert!(required > 0, "{name} requires nothing");
        }
    }

    #[test]
    fn search_requires_nothing_because_an_empty_search_is_a_browse() {
        assert!(schema_for("library_search").get("required").is_none());
    }
}
