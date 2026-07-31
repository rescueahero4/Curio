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

    let entry = resolve_entry(std::path::Path::new(&project.path));
    state.with_db(|db| projects::mark_opened(db.conn(), &project.id))?;

    Ok(Json(Opened {
        url: format!("/p/{id}/{entry}"),
        entry,
        path: project.path,
    }))
}

/// Where a project's front door is (R-BE-22).
///
/// Root `index.html`, else the **newest numeric subfolder** containing one, else a listing.
/// The numeric rule exists because AI tools commonly write `v1/`, `v2/`, `v3/` and the user
/// means the latest — opening `v1` because it sorts first would show them their oldest
/// attempt every time.
fn resolve_entry(root: &std::path::Path) -> String {
    if root.join("index.html").is_file() {
        return "index.html".to_owned();
    }

    let mut versions: Vec<(u64, String)> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.path().join("index.html").is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            // `v3` and `3` both count; anything else is a folder that happens to hold an
            // index.html, which is not the same as a version of the project.
            let digits: String = name.chars().filter(char::is_ascii_digit).collect();
            let looks_numeric = !digits.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_digit() || c.eq_ignore_ascii_case(&'v'));
            looks_numeric.then(|| digits.parse().ok().map(|number| (number, name)))?
        })
        .collect();

    versions.sort_unstable();
    versions
        .pop()
        .map_or_else(String::new, |(_, name)| format!("{name}/index.html"))
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

    fn tree(folders: &[&str], files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for folder in folders {
            std::fs::create_dir_all(dir.path().join(folder)).expect("mkdir");
        }
        for file in files {
            std::fs::write(dir.path().join(file), "<h1>hi</h1>").expect("write");
        }
        dir
    }

    #[test]
    fn a_root_index_is_the_front_door() {
        let dir = tree(&[], &["index.html"]);
        assert_eq!(resolve_entry(dir.path()), "index.html");
    }

    #[test]
    fn the_newest_numeric_folder_wins() {
        // AI tools write v1, v2, v3 and the user means the latest. Sorting as text would
        // put v10 before v2 and show them the wrong one as soon as they got to double
        // digits.
        let dir = tree(
            &["v1", "v2", "v10"],
            &["v1/index.html", "v2/index.html", "v10/index.html"],
        );
        assert_eq!(resolve_entry(dir.path()), "v10/index.html");
    }

    #[test]
    fn a_root_index_beats_a_versioned_one() {
        let dir = tree(&["v2"], &["index.html", "v2/index.html"]);
        assert_eq!(resolve_entry(dir.path()), "index.html");
    }

    #[test]
    fn a_non_numeric_folder_is_not_a_version() {
        // `docs/index.html` is documentation, not a version of the project.
        let dir = tree(&["docs"], &["docs/index.html"]);
        assert_eq!(resolve_entry(dir.path()), "");
    }

    #[test]
    fn a_project_with_nothing_to_open_says_so_rather_than_guessing() {
        let dir = tree(&["src"], &["src/main.rs"]);
        assert_eq!(resolve_entry(dir.path()), "");
    }
}
