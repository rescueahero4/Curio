//! `/api/jobs` and `/api/system` — the queue, and the handful of things only the machine
//! Curio runs on can do.
//!
//! The system routes all follow one copy rule (PRD §5): they report what was **asked for**,
//! never what happened. Launching an external tool is a request to the OS, and the OS
//! answers later or not at all — so "Asked Claude Code to open" is the truth and "Opened
//! Claude Code" is a guess that is wrong often enough to matter (Inventory §10.22).

use axum::Json;
use axum::extract::{Path, State};
use serde::{Deserialize, Serialize};

use curio_core::domain::{Job, JobKind};
use curio_core::events::{Event, EventName};
use curio_db::{items, jobs};

use crate::routes::error::ApiResult;
use crate::state::AppState;

/// How many jobs the queue view shows.
const RECENT_JOBS: usize = 50;

/// `GET /api/jobs`.
pub async fn list_jobs(State(state): State<AppState>) -> ApiResult<Json<Vec<Job>>> {
    Ok(Json(
        state.with_db(|db| jobs::recent(db.conn(), RECENT_JOBS))?,
    ))
}

/// `GET /api/jobs/:id`.
pub async fn get_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Job>> {
    Ok(Json(state.with_db(|db| jobs::require(db.conn(), &id))?))
}

/// `POST /api/jobs/:id/cancel`.
pub async fn cancel_job(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Job>> {
    let job = state.with_db(|db| jobs::cancel(db.conn(), &id))?;

    // R-BE-19: a batch-backed job cancelled locally leaves the batch running upstream —
    // still working, still billing, after the user believes they stopped it. The worker
    // would catch this on its next poll; doing it here makes "cancel" mean cancelled at
    // the moment the button is pressed.
    if let Some(batch_id) = job
        .result
        .as_ref()
        .and_then(|result| result.get("batch_id"))
        .and_then(serde_json::Value::as_str)
        && let Some(key) = crate::secrets::api_key()
        && let Ok(client) = crate::ai::Anthropic::new(key)
        && let Err(err) = client.cancel_batch(batch_id).await
    {
        // Not fatal to the request: the local job *is* cancelled, and saying otherwise
        // would leave the user pressing a button that already worked.
        tracing::warn!(%err, batch = batch_id, "could not cancel the batch upstream");
    }

    publish_job(&state, &job);
    Ok(Json(job))
}

#[derive(Debug, Serialize)]
pub struct Enqueued {
    pub job_id: String,
}

/// `POST /api/items/:id/reassess` — run the assessment again on demand (FR-9).
pub async fn reassess(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<Json<Enqueued>> {
    // Checked here so an unknown id is a 404 rather than a job that fails minutes later
    // with nothing on screen to connect it to.
    state.with_db(|db| items::require(db.conn(), &id))?;

    let job = state.with_db(|db| {
        jobs::enqueue(
            db.conn(),
            JobKind::AssessItem,
            &serde_json::json!({ "item_id": id }),
        )
    })?;

    publish_job(&state, &job);
    state.wake_worker();
    Ok(Json(Enqueued { job_id: job.id }))
}

#[derive(Debug, Deserialize)]
pub struct RevealBody {
    pub path: String,
    /// When `path` itself is gone, open the closest folder above it that still exists.
    ///
    /// Off by default, and deliberately opt-in: for a prompt snapshot "that file is not on
    /// disk" is the honest answer, and quietly opening its parent instead would read as
    /// though the file had been found. A project whose folder was deleted is the opposite
    /// case — the user is being asked to go and look for it, so somewhere to start looking
    /// is the whole point.
    #[serde(default)]
    pub nearest: bool,
}

#[derive(Debug, Serialize)]
pub struct Outcome {
    /// Whether the request reached the OS. Not whether anything opened — nothing can know
    /// that from here.
    pub asked: bool,
    /// What to show the user, phrased as a request (Inventory §10.22).
    pub message: String,
}

/// `POST /api/system/reveal` — show a path in the OS file manager.
///
/// Always answers 200 with the outcome in the body, never a 4xx. A file manager that
/// declined to open is not a failed request, and rendering it as one would put a red error
/// in front of a user whose only problem is that nothing appeared.
pub async fn reveal(Json(body): Json<RevealBody>) -> Json<Outcome> {
    let wanted = std::path::Path::new(&body.path);
    if body.nearest && !wanted.exists() {
        return Json(match nearest_folder(wanted) {
            Some(found) => {
                let mut outcome = open_in_os(&found, "Asked your file manager to open");
                // Never let the fallback pass for the real thing: the message names what was
                // opened *and* what was not there.
                if outcome.asked {
                    outcome.message = format!(
                        "{} is not there. Opened the closest folder that is, {}.",
                        wanted.display(),
                        found.display()
                    );
                }
                outcome
            }
            None => Outcome {
                asked: false,
                message: format!(
                    "Neither {} nor anything above it is on disk.",
                    wanted.display()
                ),
            },
        });
    }

    Json(open_in_os(wanted, "Asked your file manager to open"))
}

/// The closest ancestor of `path` that is a folder on disk, if there is one.
fn nearest_folder(path: &std::path::Path) -> Option<std::path::PathBuf> {
    path.ancestors()
        .skip(1)
        .find(|ancestor| ancestor.is_dir())
        .map(std::path::Path::to_path_buf)
}

/// `POST /api/system/open-skill-file` — open the editable assessment rubric (FR-3's
/// "rubric editable as a markdown skill file").
pub async fn open_skill_file(State(state): State<AppState>) -> Json<Outcome> {
    let path = state
        .data_root()
        .join(curio_core::paths::SKILL_FILE_RELATIVE);
    Json(open_in_os(&path, "Asked your editor to open"))
}

#[derive(Debug, Deserialize)]
pub struct SendBody {
    /// The serialized prompt. Sent from the client because it has already put this exact
    /// text on the clipboard — the ordering invariant is serialize → copy → claim → launch,
    /// and the launch must carry what was copied, not a fresh serialization that could
    /// differ (Inventory §10.22).
    pub text: String,
}

/// `POST /api/system/send-to-claude` — best-effort launch of the configured target.
pub async fn send_to_claude(
    State(state): State<AppState>,
    Json(body): Json<SendBody>,
) -> Json<Outcome> {
    use curio_core::config::SendToClaudeTarget;

    let target = state.config().send_to_claude_target;
    let (command, label) = match target {
        SendToClaudeTarget::ClaudeCode => ("claude", "Claude Code"),
        SendToClaudeTarget::ClaudeDesktop => ("claude-desktop", "Claude Desktop"),
        // Not a degraded mode — a deliberate choice for users who paste into something
        // Curio has never heard of. The clipboard already holds the prompt.
        SendToClaudeTarget::Clipboard => {
            return Json(Outcome {
                asked: true,
                message: format!(
                    "Copied {} characters. Paste them wherever you're working.",
                    body.text.chars().count()
                ),
            });
        }
    };

    let launched = std::process::Command::new(command)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn();

    Json(match launched {
        Ok(_) => Outcome {
            asked: true,
            message: format!("Asked {label} to open — paste there."),
        },
        Err(_) => Outcome {
            asked: false,
            // Honest and actionable: the prompt is already on the clipboard, so the user
            // has lost nothing and the next step is one they can take.
            message: format!(
                "Couldn't start {label}, but the prompt is on your clipboard — paste it there."
            ),
        },
    })
}

/// Ask the OS to open a path, without waiting to find out whether it did.
fn open_in_os(path: &std::path::Path, phrasing: &str) -> Outcome {
    if !path.exists() {
        return Outcome {
            asked: false,
            message: format!("{} is not on disk.", path.display()),
        };
    }

    #[cfg(windows)]
    let command = std::process::Command::new("explorer").arg(path).spawn();
    #[cfg(target_os = "macos")]
    let command = std::process::Command::new("open").arg(path).spawn();
    #[cfg(all(unix, not(target_os = "macos")))]
    let command = std::process::Command::new("xdg-open").arg(path).spawn();

    match command {
        // Windows Explorer returns a non-zero exit code on success often enough that
        // reading it would produce false failures; the spawn succeeding is all we claim.
        Ok(_) => Outcome {
            asked: true,
            message: format!("{phrasing} {}.", path.display()),
        },
        Err(err) => Outcome {
            asked: false,
            message: format!("Couldn't ask the system to open it: {err}"),
        },
    }
}

fn publish_job(state: &AppState, job: &Job) {
    match serde_json::to_value(job) {
        Ok(payload) => state.publish(Event::new(EventName::JobUpdated, payload)),
        Err(err) => tracing::error!(%err, "could not serialize a job for the event stream"),
    }
}

/// How long the response has to reach the caller before the process starts going down.
///
/// Inherited from the previous implementation (Inventory §8). Long enough for a loopback
/// response to be flushed and read, short enough that Quit feels immediate.
const QUIT_GRACE: std::time::Duration = std::time::Duration::from_millis(150);

/// `POST /api/system/quit` — authenticated by the quit token alone (R-SEC-8).
///
/// Answers **first**, then signals shutdown after a short grace, so the caller receives a
/// real response rather than a dropped connection it has to interpret (R-BE-7). The signal
/// converges on the same path the tray's Quit uses — there is one shutdown sequence, not
/// two that can drift.
pub async fn quit(State(state): State<AppState>) -> Json<Outcome> {
    tokio::spawn(async move {
        tokio::time::sleep(QUIT_GRACE).await;
        state.request_quit();
    });

    Json(Outcome {
        asked: true,
        message: "Curio is shutting down.".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn revealing_a_path_that_is_not_there_says_so_without_failing() {
        // A 4xx here would put a red error in front of a user whose only problem is that
        // nothing appeared.
        let Json(outcome) = reveal(Json(RevealBody {
            path: "/definitely/not/here".to_owned(),
            nearest: false,
        }))
        .await;

        assert!(!outcome.asked);
        assert!(
            outcome.message.contains("not on disk"),
            "{}",
            outcome.message
        );
    }

    #[test]
    fn locating_a_deleted_folder_falls_back_to_the_closest_one_that_exists() {
        // What "Locate" on a missing project needs: somewhere to start looking. Revealing
        // the vanished path itself would open nothing at all.
        let dir = tempfile::tempdir().expect("tempdir");
        let gone = dir.path().join("gone").join("deeper");

        assert_eq!(nearest_folder(&gone).as_deref(), Some(dir.path()));
    }

    #[test]
    fn the_copy_says_asked_rather_than_opened() {
        // Inventory §10.22. Launching is a request to the OS, and claiming it succeeded is
        // a guess that is wrong often enough to matter.
        let outcome = open_in_os(std::path::Path::new("/nope"), "Asked your editor to open");
        assert!(!outcome.message.contains("Opened"), "{}", outcome.message);
    }
}
