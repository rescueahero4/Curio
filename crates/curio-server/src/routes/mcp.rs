//! `/mcp` — the MCP surface, and the library behind it.
//!
//! Two halves. [`ServerLibrary`] implements `curio-mcp`'s trait over the same service
//! functions REST uses, so an agent reaches exactly the surface a user reaches and no wider
//! (R-MCP-13). And [`service`] mounts rmcp's Streamable HTTP transport at `/mcp`, above the
//! SPA catch-all — a disabled MCP surface must answer JSON-RPC, never the dashboard's HTML
//! (Inventory §10.6).
//!
//! ## The transport is stateless, and that is a decision that verified
//!
//! D0 row 7 asked whether rmcp still exposes the stateless / JSON-response configuration
//! R-MCP-4 assumes; D28 pinned major 3 and moved the question to P4, which is here. It
//! does: `legacy_session_mode: false` serves every request statelessly, and
//! `json_response: true` answers simple tool calls as `application/json` rather than an SSE
//! stream. Both matter because the stdio proxy (D24) forwards frames to this endpoint —
//! a session-bound transport would make the proxy hold state it has no way to recover after
//! a restart.

use std::sync::Arc;

use curio_core::domain::{ItemStatus, JobKind, VocabularyKind};
use curio_core::events::{Event, EventName};
use curio_core::query::ItemQuery;
use curio_db::{items, jobs, projects, prompts, vocabulary};
use serde_json::{Value, json};

use crate::state::AppState;

/// The library, as the MCP tools see it.
pub struct ServerLibrary {
    state: AppState,
}

impl ServerLibrary {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self { state }
    }

    fn root(&self) -> std::path::PathBuf {
        self.state.data_root().to_path_buf()
    }
}

/// Read a string argument, or refuse by name rather than by position.
fn required_str(arguments: &Value, field: &str) -> curio_core::Result<String> {
    arguments
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| curio_core::Error::invalid(format!("{field} is required")))
}

fn string_list(arguments: &Value, field: &str) -> Vec<String> {
    arguments
        .get(field)
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

impl curio_mcp::Library for ServerLibrary {
    /// Read per request, never cached (R-MCP-8).
    fn mcp_enabled(&self) -> bool {
        self.state.config().mcp_enabled
    }

    fn is_paused(&self) -> bool {
        self.state.is_paused()
    }

    fn search(&self, arguments: &Value) -> curio_core::Result<Value> {
        // Names in, ids out. An agent has no way to know an id, and refusing a name the
        // library does not have would make one bad filter fail an otherwise good search —
        // so unknown names are dropped silently (Inventory §2).
        let query = self.state.with_db(|db| -> curio_core::Result<ItemQuery> {
            Ok(ItemQuery {
                types: resolve(
                    db,
                    VocabularyKind::DesignType,
                    &string_list(arguments, "design_types"),
                )?,
                families: resolve(
                    db,
                    VocabularyKind::Family,
                    &string_list(arguments, "families"),
                )?,
                tags: resolve(db, VocabularyKind::Tag, &string_list(arguments, "tags"))?,
                statuses: Vec::new(),
                search: arguments
                    .get("query")
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .filter(|text| !text.trim().is_empty()),
                needs_review: false,
                limit: arguments
                    .get("limit")
                    .and_then(Value::as_u64)
                    .map_or(20, |limit| {
                        usize::try_from(limit).unwrap_or(20).clamp(1, 50)
                    }),
                cursor: None,
            })
        })?;

        let page = self.state.with_db(|db| items::list(db.conn(), &query))?;
        let root = self.root();

        Ok(json!({
            "total": page.items.len(),
            "items": page.items.iter().map(|item| json!({
                "id": item.id,
                "name": item.name,
                "short_description": item.short_description,
                "source_url": item.source_url,
                "status": item.status.as_str(),
                // Absolute, because an agent's working directory is not ours.
                "directory": curio_core::paths::item_dir(&root, &item.id).to_string_lossy(),
                "design_types": item.design_types,
                "tags": item.tags,
                "families": item.families.iter().map(|family| json!({
                    "name": family.name, "score": family.score
                })).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        }))
    }

    fn get_item(&self, arguments: &Value) -> curio_core::Result<Value> {
        let id = required_str(arguments, "item_id")?;
        let item = self.state.with_db(|db| items::require(db.conn(), &id))?;
        let root = self.root();
        let directory = curio_core::paths::item_dir(&root, &item.id);

        let mut value = serde_json::to_value(&item)?;
        if let Some(object) = value.as_object_mut() {
            // Absolute paths: the agent is a different process with a different cwd, and a
            // relative path here is a file it cannot open.
            object.insert(
                "screenshot_absolute_path".to_owned(),
                json!(root.join(&item.screenshot_path).to_string_lossy()),
            );
            object.insert(
                "sidecar_absolute_path".to_owned(),
                json!(directory.join("item.md").to_string_lossy()),
            );
            object.insert("directory".to_owned(), json!(directory.to_string_lossy()));
        }
        Ok(value)
    }

    fn list_vocabulary(&self) -> curio_core::Result<Value> {
        self.state.with_db(|db| {
            Ok(json!({
                // Families carry descriptions because that is what makes them scoreable —
                // a bare name tells an agent nothing about what it would be filtering for.
                "families": vocabulary::list_families(db.conn())?
                    .iter()
                    .map(|family| json!({
                        "id": family.id,
                        "name": family.name,
                        "description": family.description,
                        "item_count": family.item_count,
                    }))
                    .collect::<Vec<_>>(),
                "design_types": vocabulary::list_terms(db.conn(), VocabularyKind::DesignType)?
                    .into_iter().map(|term| term.name).collect::<Vec<_>>(),
                "tags": vocabulary::list_terms(db.conn(), VocabularyKind::Tag)?
                    .into_iter().map(|term| term.name).collect::<Vec<_>>(),
            }))
        })
    }

    fn create_item(&self, arguments: &Value) -> curio_core::Result<Value> {
        let source = std::path::PathBuf::from(required_str(arguments, "screenshot_path")?);
        if !source.is_file() {
            return Err(curio_core::Error::invalid(format!(
                "there is no file at {}",
                source.display()
            )));
        }

        let root = self.root();
        let name = arguments
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                source
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Untitled".to_owned())
            });

        let item = self.state.with_db_mut(|db| {
            items::create(
                db.conn_mut(),
                Some(&root),
                &items::NewItem {
                    name,
                    source_url: arguments
                        .get("source_url")
                        .and_then(Value::as_str)
                        .map(str::to_owned),
                    screenshot_path: String::new(),
                    thumbnail_path: None,
                },
            )
        })?;

        // Copied in, not referenced. The library owns its media, and a path the agent
        // supplied could be deleted the moment after this returns (R-DA-1).
        let directory = curio_core::paths::item_dir(&root, &item.id);
        std::fs::create_dir_all(&directory)?;
        std::fs::copy(&source, directory.join("screenshot.png"))?;

        let relative = format!("items/{}/screenshot.png", item.id);
        let item = self
            .state
            .with_db(|db| items::set_media(db.conn(), Some(&root), &item.id, &relative, None))?;

        let job = self.state.with_db(|db| {
            jobs::enqueue(
                db.conn(),
                JobKind::AssessItem,
                &json!({ "item_id": item.id }),
            )
        })?;

        if let Ok(payload) = serde_json::to_value(&item) {
            self.state
                .publish(Event::new(EventName::ItemCreated, payload));
        }
        self.state.wake_worker();

        Ok(json!({ "item_id": item.id, "job_id": job.id, "status": item.status.as_str() }))
    }

    fn update_item(&self, arguments: &Value) -> curio_core::Result<Value> {
        let id = required_str(arguments, "item_id")?;
        let root = self.root();

        let patch = items::ItemPatch {
            name: arguments
                .get("name")
                .and_then(Value::as_str)
                .map(str::to_owned),
            short_description: arguments
                .get("short_description")
                .and_then(Value::as_str)
                .map(str::to_owned),
            source_url: None,
            image_recipe: None,
            tags: arguments
                .get("tags")
                .map(|_| string_list(arguments, "tags")),
            design_types: arguments
                .get("design_types")
                .map(|_| string_list(arguments, "design_types")),
            family_ids: None,
            status: None,
        };

        if patch.is_empty() {
            return Err(curio_core::Error::invalid(
                "that update would change nothing",
            ));
        }

        // Stamped `ai`, not `user` (Inventory §10.12). A later assessment must know it may
        // rename this item; a `user` stamp here would freeze a name the user never chose.
        let item = self.state.with_db_mut(|db| {
            items::patch(
                db.conn_mut(),
                Some(&root),
                &id,
                &patch,
                curio_core::domain::LastEditedBy::Ai,
            )
        })?;

        if let Ok(payload) = serde_json::to_value(&item) {
            self.state
                .publish(Event::new(EventName::ItemUpdated, payload));
        }
        Ok(serde_json::to_value(&item)?)
    }

    fn prompt_get(&self, arguments: &Value) -> curio_core::Result<Value> {
        let id = required_str(arguments, "prompt_id")?;
        let prompt = self.state.with_db(|db| prompts::require(db.conn(), &id))?;

        // Serialized, not raw: a prompt's value is its chips expanded into the families,
        // items and tags they point at, which is the whole reason the tool exists (FR-14).
        // The same context builder the REST route uses, so `prompt_get` and
        // `POST /api/prompts/:id/serialize` cannot render the same prompt differently.
        let root = self.root();
        let text = self.state.with_db(|db| {
            let context = super::api::prompts::chip_context(db, &root)?;
            Ok::<_, curio_db::Error>(curio_core::prompt::serialize(&prompt.doc_json, &context))
        })?;

        Ok(json!({
            "id": prompt.id,
            "title": prompt.title,
            "text": text,
            "updated_at": prompt.updated_at,
        }))
    }

    fn project_register(&self, arguments: &Value) -> curio_core::Result<Value> {
        let path = std::path::PathBuf::from(required_str(arguments, "path")?);
        if !path.is_dir() {
            return Err(curio_core::Error::invalid(format!(
                "there is no directory at {}",
                path.display()
            )));
        }

        let name = arguments
            .get("name")
            .and_then(Value::as_str)
            .map(str::to_owned)
            .unwrap_or_else(|| {
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "Project".to_owned())
            });

        // `Manual` origin, no fingerprint: an agent registering a directory is the same
        // act as a user pressing Register, and neither mints a marker file. Only adoption
        // does (R-BE-21).
        let (project, _created) = self.state.with_db(|db| {
            projects::register(
                db.conn(),
                &path.to_string_lossy(),
                &name,
                curio_core::domain::ProjectOrigin::Manual,
                None,
                None,
            )
        })?;

        if let Ok(payload) = serde_json::to_value(&project) {
            self.state
                .publish(Event::new(EventName::ProjectUpdated, payload));
        }

        let mut answer = serde_json::to_value(&project)?;
        // Absent `variants` writes nothing. That is what keeps "Curio does not write into a
        // folder it did not adopt" true: the file appears only when the agent that made the
        // folder explicitly asks for it, and it carries no identity — just the names it
        // chose for its own directions.
        if let Some(entries) = manifest_entries(arguments) {
            let report = crate::routes::api::projects::write_variants(&path, entries);
            if let Ok(object) = answer.as_object_mut().ok_or(()) {
                object.insert("variants".to_owned(), report);
            }
        }
        Ok(answer)
    }
}

/// The `variants` argument, if the agent sent one.
///
/// A malformed array is dropped rather than refused: the registration is the point, and
/// failing it over a mistyped label would cost the agent the project as well as the names.
fn manifest_entries(arguments: &Value) -> Option<Vec<curio_core::variants::ManifestEntry>> {
    let raw = arguments.get("variants")?;
    serde_json::from_value(raw.clone()).ok()
}

/// Names to ids, dropping what the library does not have.
fn resolve(
    db: &curio_db::Db,
    kind: VocabularyKind,
    names: &[String],
) -> curio_core::Result<Vec<String>> {
    if names.is_empty() {
        return Ok(Vec::new());
    }
    let known = if kind == VocabularyKind::Family {
        vocabulary::list_families(db.conn())?
            .into_iter()
            .map(|family| (family.name, family.id))
            .collect::<Vec<_>>()
    } else {
        vocabulary::list_terms(db.conn(), kind)?
            .into_iter()
            .map(|term| (term.name, term.id))
            .collect::<Vec<_>>()
    };

    Ok(names
        .iter()
        .filter_map(|wanted| {
            known
                .iter()
                .find(|(name, _)| name.eq_ignore_ascii_case(wanted))
                .map(|(_, id)| id.clone())
        })
        .collect())
}

/// Silence the unused-import warning for a type only the trait signature mentions.
const _: Option<ItemStatus> = None;

/// The rmcp Streamable HTTP service, ready to mount at `/mcp`.
///
/// The factory runs **per request** (R-MCP-4): it clones handles rather than rebuilding
/// pools, which is what lets `mcpEnabled` be read per call rather than frozen at router
/// construction.
#[must_use]
pub fn service(
    state: AppState,
) -> rmcp::transport::streamable_http_server::StreamableHttpService<
    curio_mcp::Curio<ServerLibrary>,
    rmcp::transport::streamable_http_server::session::local::LocalSessionManager,
> {
    use rmcp::transport::streamable_http_server::{
        StreamableHttpService, session::local::LocalSessionManager,
    };

    // D0 row 7, answered here: rmcp 3.x does still expose both knobs R-MCP-4 assumes.
    let mut config = rmcp::transport::streamable_http_server::StreamableHttpServerConfig::default();
    // Stateless. The stdio proxy forwards frames here and cannot recover a session across a
    // restart, so binding one would be state with no owner (D24).
    config.legacy_session_mode = false;
    // Plain JSON for simple tool calls; the transport still falls back to SSE if a handler
    // emits anything before its final response.
    config.json_response = true;

    StreamableHttpService::new(
        move || {
            Ok(curio_mcp::Curio::new(Arc::new(ServerLibrary::new(
                state.clone(),
            ))))
        },
        Arc::new(LocalSessionManager::default()),
        config,
    )
}
