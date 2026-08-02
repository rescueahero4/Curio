//! `/api/projects` — the catalogue of folders the user's AI tools wrote (FR-17..FR-19).
//!
//! `GET` **repairs as it reads** (R-BE-22, Inventory §10.28). That is unusual for a read
//! route and deliberate: the watcher can miss a change while the app was closed, and the
//! moment a user opens the Projects page is exactly when a stale record is most visible and
//! least excusable. Reconciliation is cheap — one `exists()` per project — and it means the
//! page can never show a folder that is not there without saying so.

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use curio_core::domain::{Project, ProjectOrigin, ProjectStatus};
use curio_core::events::{Event, EventName};
use curio_db::projects;

use crate::routes::error::{ApiError, ApiResult};
use crate::state::AppState;

/// `GET /api/projects` — the catalogue, reconciled against the filesystem.
pub async fn list(State(state): State<AppState>) -> ApiResult<Json<Vec<Project>>> {
    let known = state.with_db(|db| projects::list(db.conn()))?;
    let mut reconciled = Vec::with_capacity(known.len());

    for project in known {
        let present = std::path::Path::new(&project.path).is_dir();
        let repaired = match (present, project.status) {
            (false, ProjectStatus::Present) => {
                state.with_db(|db| projects::mark_missing(db.conn(), &project.id))?;
                Project {
                    status: ProjectStatus::Missing,
                    ..project
                }
            }
            (true, ProjectStatus::Missing) => {
                // It came back — restored from a backup, or a drive that was unplugged.
                // Marking it present is the whole reason the record was kept (FR-19).
                state.with_db(|db| {
                    projects::register(
                        db.conn(),
                        &project.path,
                        &project.name,
                        project.origin,
                        project.fingerprint.as_deref(),
                        project.prompt_id.as_deref(),
                    )
                })?;
                Project {
                    status: ProjectStatus::Present,
                    ..project
                }
            }
            _ => project,
        };

        if repaired.status != known_status(&repaired) {
            publish(&state, EventName::ProjectUpdated, &repaired);
        }
        reconciled.push(repaired);
    }

    Ok(Json(reconciled))
}

#[derive(Debug, Deserialize)]
pub struct RegisterBody {
    pub path: String,
    pub name: Option<String>,
    pub prompt_id: Option<String>,
    /// Optional descriptions of the version folders inside `path`, written to
    /// `curio-variants.json`. Present so REST reaches exactly what MCP reaches and no less
    /// (R-MCP-13) — the MCP tool is a mapping onto this surface, not a wider one.
    #[serde(default)]
    pub variants: Option<Vec<curio_core::variants::ManifestEntry>>,
}

/// Write the variant manifest and say what happened, in a shape both callers report verbatim.
///
/// Never fails the registration it is part of. The project record is the thing the user asked
/// for; a manifest that could not be written is worth reporting and not worth undoing a
/// registration over.
pub fn write_variants(
    root: &std::path::Path,
    entries: Vec<curio_core::variants::ManifestEntry>,
) -> serde_json::Value {
    match curio_core::variants::write_manifest(root, entries) {
        Ok(report) => serde_json::json!({
            "written": report.written,
            "skipped": report.skipped,
        }),
        Err(err) => serde_json::json!({ "written": 0, "error": err.to_string() }),
    }
}

/// `POST /api/projects` — register a folder by hand (FR-17's manual path).
pub async fn register(
    State(state): State<AppState>,
    Json(body): Json<RegisterBody>,
) -> ApiResult<Json<Project>> {
    let path = std::path::Path::new(&body.path);
    if !path.is_dir() {
        return Err(ApiError(curio_core::Error::invalid(format!(
            "{} is not a folder on this machine",
            body.path
        ))));
    }

    let name = body.name.unwrap_or_else(|| {
        path.file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| body.path.clone())
    });

    let (project, is_new) = state.with_db(|db| {
        projects::register(
            db.conn(),
            &body.path,
            &name,
            ProjectOrigin::Manual,
            // No fingerprint: identity is minted on adoption by the watcher, never on
            // registration (Inventory §10.17), so a manually-added project is deliberately
            // not rename-followable.
            None,
            body.prompt_id.as_deref(),
        )
    })?;

    // Only when asked. A bare registration writes nothing into the folder, which is what
    // keeps identity-on-adoption honest (Inventory §10.17) — this file is content the caller
    // authored, not a marker Curio minted.
    if let Some(entries) = body.variants {
        let _ = write_variants(path, entries);
    }

    publish(
        &state,
        if is_new {
            EventName::ProjectDetected
        } else {
            EventName::ProjectUpdated
        },
        &project,
    );
    Ok(Json(project))
}

#[derive(Debug, Deserialize)]
pub struct UpdateBody {
    /// `null` unlinks. Absent is not accepted — a PATCH that changes nothing is a client
    /// bug worth surfacing rather than a no-op worth hiding.
    pub prompt_id: Option<String>,
}

/// `PATCH /api/projects/:id` — link or unlink the prompt that produced it.
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(body): Json<UpdateBody>,
) -> ApiResult<Json<Project>> {
    let project =
        state.with_db(|db| projects::set_prompt(db.conn(), &id, body.prompt_id.as_deref()))?;
    publish(&state, EventName::ProjectUpdated, &project);
    Ok(Json(project))
}

/// `DELETE /api/projects/:id` — forget a project record. The folder is never touched.
///
/// FR-19 keeps a `missing` folder's record on its own, because the record carries the prompt
/// link and nothing on disk can rebuild that. This route is the user overriding that: a
/// folder they deleted on purpose should not have to stay in the list forever. Reconciliation
/// only ever *marks* missing (never deletes), so removal stays a decision the user makes.
pub async fn delete(State(state): State<AppState>, Path(id): Path<String>) -> ApiResult<Json<()>> {
    // `require` first, so removing something already gone answers 404 rather than 200 —
    // a silent success would look like it worked on a stale id.
    state.with_db(|db| projects::require(db.conn(), &id))?;
    state.with_db(|db| projects::forget(db.conn(), &id))?;
    state.publish(Event::project_removed(&id));
    Ok(Json(()))
}

#[derive(Debug, Serialize)]
pub struct Opened {
    /// The local URL to open. Served through the jail, never as a `file://` path — a
    /// `file://` origin cannot run the fetches most built projects make.
    pub url: String,
    pub entry: String,
    pub path: String,
}

/// `POST /api/projects/:id/open` — resolve an entry point and hand back a URL (FR-18).
pub async fn open(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Opened>> {
    let project = state.with_db(|db| projects::require(db.conn(), &id))?;

    if !std::path::Path::new(&project.path).is_dir() {
        state.with_db(|db| projects::mark_missing(db.conn(), &project.id))?;
        return Err(ApiError(curio_core::Error::invalid(
            "that folder is no longer on disk",
        )));
    }

    let entry = curio_core::variants::front_door(std::path::Path::new(&project.path));
    state.with_db(|db| projects::mark_opened(db.conn(), &project.id))?;

    Ok(Json(Opened {
        url: format!("/p/{id}/{entry}"),
        entry,
        path: project.path,
    }))
}

/// One version of a project, as the switcher needs it.
#[derive(Debug, Serialize)]
pub struct VariantView {
    /// The folder name; empty for a project whose `index.html` is at the root.
    pub slug: String,
    pub entry: String,
    /// Where to send the browser. Through the jail, for the reason [`Opened::url`] gives.
    pub url: String,
    /// The manifest's name for this direction, or the folder name when nobody said.
    pub name: String,
    /// Whether that name came from the manifest or is just the folder repeated back. The
    /// switcher uses this to decide whether it is showing labels or filenames.
    pub described: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub design_type: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct Variants {
    pub variants: Vec<VariantView>,
    /// `ok`, `absent`, or `malformed` — the user has to be able to tell a missing manifest
    /// from one with a typo in it, because only the second is theirs to fix.
    pub manifest_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manifest_error: Option<String>,
    /// Folders the manifest describes that are not on disk. Reported rather than dropped: a
    /// silently ignored entry looks like Curio lost the label.
    pub unknown_folders: Vec<String>,
}

/// `GET /api/projects/:id/variants` — every version of a project, oldest first.
///
/// The order is the opposite end from [`open`], which takes the newest. Both are deliberate
/// and both are asserted in `curio_core::variants`, where the two rules sit together.
///
/// The manifest may only ever *enrich* what is on disk. A folder that exists is listed even
/// when nothing describes it, and a manifest entry naming a folder that does not exist cannot
/// conjure one — otherwise a stale file would offer the user a link to a 404.
pub async fn variants(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Variants>> {
    let project = state.with_db(|db| projects::require(db.conn(), &id))?;
    let root = std::path::Path::new(&project.path);

    // The same answer `open` gives, for the same reason. Two routes that disagree about
    // whether a project is on disk is a bug the user would experience as a phantom.
    if !root.is_dir() {
        state.with_db(|db| projects::mark_missing(db.conn(), &project.id))?;
        return Err(ApiError(curio_core::Error::invalid(
            "that folder is no longer on disk",
        )));
    }

    let found = curio_core::variants::scan(root);
    let (manifest, manifest_status, manifest_error) =
        match curio_core::variants::read_manifest(root) {
            curio_core::variants::ManifestOutcome::Ok(manifest) => (Some(manifest), "ok", None),
            curio_core::variants::ManifestOutcome::Absent => (None, "absent", None),
            // Still answer with every folder. A switcher that disappeared over a trailing comma
            // would be a worse bug than the trailing comma.
            curio_core::variants::ManifestOutcome::Malformed(reason) => {
                (None, "malformed", Some(reason))
            }
        };

    let described = manifest
        .map(|manifest| manifest.variants)
        .unwrap_or_default();
    let unknown_folders = described
        .iter()
        .filter(|entry| !found.iter().any(|variant| variant.slug == entry.folder))
        .map(|entry| entry.folder.clone())
        .collect();

    let variants = found
        .into_iter()
        .map(|variant| {
            let entry = described
                .iter()
                .find(|candidate| candidate.folder == variant.slug);
            let name = entry
                .and_then(|entry| entry.name.clone())
                .filter(|name| !name.trim().is_empty());

            VariantView {
                url: format!("/p/{}/{}", project.id, variant.entry),
                name: name
                    .clone()
                    .unwrap_or_else(|| fallback_name(&variant.slug, &project.name)),
                described: name.is_some(),
                summary: entry.and_then(|entry| entry.summary.clone()),
                family: entry.and_then(|entry| entry.family.clone()),
                design_type: entry.and_then(|entry| entry.design_type.clone()),
                tags: entry.map(|entry| entry.tags.clone()).unwrap_or_default(),
                slug: variant.slug,
                entry: variant.entry,
            }
        })
        .collect();

    Ok(Json(Variants {
        variants,
        manifest_status,
        manifest_error,
        unknown_folders,
    }))
}

/// What to call a version nobody named: the folder, or the project itself when the folder is
/// the project — a root `index.html` has no slug to show, and "" is not a label.
fn fallback_name(slug: &str, project_name: &str) -> String {
    if slug.is_empty() {
        project_name.to_owned()
    } else {
        slug.to_owned()
    }
}

fn known_status(project: &Project) -> ProjectStatus {
    project.status
}

fn publish(state: &AppState, name: EventName, project: &Project) {
    match serde_json::to_value(project) {
        Ok(payload) => state.publish(Event::new(name, payload)),
        Err(err) => tracing::error!(%err, "could not serialize a project for the event stream"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // The front-door rules moved to `curio_core::variants` with their tests, so that one
    // module owns both orderings. What is left here is the naming this route adds on top.

    #[test]
    fn an_unnamed_version_falls_back_to_its_folder() {
        assert_eq!(fallback_name("v2", "curio-test-flow"), "v2");
    }

    #[test]
    fn a_root_index_borrows_the_project_name_because_it_has_no_folder() {
        // Its slug is "", and an empty string is not a label a user can click.
        assert_eq!(fallback_name("", "curio-test-flow"), "curio-test-flow");
    }

    #[test]
    fn a_registration_only_writes_the_manifest_when_it_was_asked_to() {
        // The assertion that protects "Curio does not write into a folder it did not adopt".
        // Both callers guard on `variants` being present; this is the shape they report when
        // it is, and the proof that the folder is otherwise left alone.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join("v1")).expect("mkdir");
        std::fs::write(dir.path().join("v1").join("index.html"), "<h1>hi</h1>").expect("write");

        let manifest = dir.path().join(curio_core::variants::MANIFEST_FILE_NAME);
        assert!(!manifest.exists(), "nothing has asked for a manifest yet");

        let report = write_variants(
            dir.path(),
            vec![
                curio_core::variants::ManifestEntry {
                    folder: "v1".to_owned(),
                    name: Some("Print-tech".to_owned()),
                    ..curio_core::variants::ManifestEntry::default()
                },
                curio_core::variants::ManifestEntry {
                    folder: "v9".to_owned(),
                    ..curio_core::variants::ManifestEntry::default()
                },
            ],
        );

        assert!(manifest.exists());
        assert_eq!(report["written"], 1);
        assert_eq!(report["skipped"][0], "v9");
    }
}
