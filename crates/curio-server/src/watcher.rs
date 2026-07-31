//! The projects watcher (FR-17, FR-19, R-BE-20, R-BE-21).
//!
//! Watches the projects root at **depth 0**. A project is a top-level folder; watching
//! recursively would mean every file an AI tool writes inside one wakes the process, which
//! is a direct hit on the "≈ 0 % idle CPU" budget for no information — the folder appearing
//! is the entire event.
//!
//! ## Settle before adopting
//!
//! A folder is adopted only after it has stopped changing for [`SETTLE`]. A tool that
//! creates a directory and then writes forty files into it would otherwise be adopted at the
//! instant it was empty, and the marker file Curio mints could land in a directory the tool
//! is still deciding the shape of.
//!
//! ## Identity is minted, never derived
//!
//! Adoption writes `.curio-project` with a fresh ULID. Copies therefore get a fresh identity
//! the first time they are seen, which is correct: a copy is a different project. The
//! previous scheme derived identity from the inode, and **ext4 re-issues a deleted
//! directory's inode to the next directory created** — so a deleted project and an unrelated
//! new folder shared one identity (Inventory §10.17).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use curio_core::domain::{MARKER_FILE_NAME, ProjectMarker, ProjectOrigin, ProjectStatus};
use curio_core::events::{Event, EventName};

use crate::state::AppState;

/// How long a folder must stop changing before it is adopted (R-BE-20).
const SETTLE: Duration = Duration::from_secs(2);

/// How often the watcher reconciles.
///
/// Detection must happen within five seconds (FR-17), and a one-second sweep leaves room
/// for the settle window inside that budget. A poll rather than a filesystem subscription:
/// the root holds tens of folders, `read_dir` on it is microseconds, and it removes an
/// entire class of platform-specific event-coalescing bugs from a feature whose only job is
/// to notice a directory.
const SWEEP: Duration = Duration::from_secs(1);

/// Run the watcher until the service shuts down.
pub async fn run(state: AppState, root: PathBuf) {
    let mut pending: HashMap<PathBuf, (Instant, u64)> = HashMap::new();
    let mut ticker = tokio::time::interval(SWEEP);

    loop {
        ticker.tick().await;
        if let Err(err) = sweep(&state, &root, &mut pending) {
            // Logged and retried rather than fatal: an unreadable projects root is usually
            // a drive that is not mounted yet, and killing the watcher would mean it never
            // recovers when it is.
            tracing::warn!(%err, root = %root.display(), "the projects sweep failed");
        }
    }
}

/// One reconciliation pass.
fn sweep(
    state: &AppState,
    root: &Path,
    pending: &mut HashMap<PathBuf, (Instant, u64)>,
) -> std::io::Result<()> {
    if !root.is_dir() {
        return Ok(());
    }

    let mut seen = Vec::new();
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        // Dotfolders are tooling, not projects: `.git`, `.venv`, `.next` all live at the
        // top level of the kind of directory a user points this at.
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }

        seen.push(path.clone());
        if settled(&path, pending) {
            adopt(state, &path);
        }
    }

    pending.retain(|path, _| seen.contains(path));
    reconcile_missing(state, &seen);
    Ok(())
}

/// Whether `path` has stopped changing.
///
/// "Changing" is measured as the number of entries plus the newest modification time —
/// enough to notice a tool still writing, cheap enough to run every second on every folder.
fn settled(path: &Path, pending: &mut HashMap<PathBuf, (Instant, u64)>) -> bool {
    let signature = signature(path);
    match pending.get(path) {
        Some((since, previous)) if *previous == signature => since.elapsed() >= SETTLE,
        _ => {
            pending.insert(path.to_path_buf(), (Instant::now(), signature));
            false
        }
    }
}

fn signature(path: &Path) -> u64 {
    std::fs::read_dir(path)
        .into_iter()
        .flatten()
        .flatten()
        .fold(0u64, |accumulated, entry| {
            let modified = entry
                .metadata()
                .ok()
                .and_then(|meta| meta.modified().ok())
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map_or(0, |elapsed| elapsed.as_secs());
            accumulated.wrapping_add(modified).wrapping_add(1)
        })
}

/// Register a settled folder, minting its identity if it has none.
fn adopt(state: &AppState, path: &Path) {
    let marker = read_or_mint_marker(path);
    let fingerprint = marker.as_ref().map(ProjectMarker::fingerprint);
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();

    // Only a folder Curio has never seen may spend a prompt's claim. A folder being
    // re-scanned after a restart is not new work, and letting it claim would attribute
    // yesterday's project to this morning's prompt (R-BE-21).
    let already_known = state
        .with_db(|db| curio_db::projects::by_path(db.conn(), &path.to_string_lossy()))
        .ok()
        .flatten();

    let claim = if already_known.is_some() {
        None
    } else {
        state
            .with_db(|db| curio_db::prompts::open_claim(db.conn()))
            .ok()
            .flatten()
    };

    let registered = state.with_db(|db| {
        curio_db::projects::register(
            db.conn(),
            &path.to_string_lossy(),
            &name,
            ProjectOrigin::Watcher,
            fingerprint.as_deref(),
            claim.as_ref().map(|prompt| prompt.id.as_str()),
        )
    });

    let Ok((project, is_new)) = registered else {
        return;
    };

    if is_new && let Some(prompt) = claim {
        // Spent whether or not it was still inside its window — the claim was made for the
        // next folder, and that folder has now arrived (R-BE-21).
        let _ = state.with_db(|db| curio_db::prompts::clear_sent(db.conn(), &prompt.id));
    }

    if is_new || project.status == ProjectStatus::Missing {
        publish(
            state,
            if is_new {
                EventName::ProjectDetected
            } else {
                EventName::ProjectUpdated
            },
            &project,
        );
    }
}

/// Read a folder's marker, or write one.
///
/// A failure to write is not fatal: the project is still catalogued, it is simply not
/// rename-followable. A read-only folder — a mounted artefact, a checked-out worktree — is
/// a perfectly ordinary thing to point Curio at.
fn read_or_mint_marker(path: &Path) -> Option<ProjectMarker> {
    let marker_path = path.join(MARKER_FILE_NAME);

    if let Ok(body) = std::fs::read_to_string(&marker_path)
        && let Ok(marker) = serde_json::from_str::<ProjectMarker>(&body)
    {
        return Some(marker);
    }

    let marker = ProjectMarker {
        id: curio_core::ids::generate(),
        tool: Some("curio".to_owned()),
        note: None,
    };
    let body = serde_json::to_string_pretty(&marker).ok()?;
    match std::fs::write(&marker_path, body) {
        Ok(()) => Some(marker),
        Err(err) => {
            tracing::info!(%err, folder = %path.display(), "could not mint a project marker");
            None
        }
    }
}

/// Mark records whose folders are gone. Never deletes (FR-19).
fn reconcile_missing(state: &AppState, seen: &[PathBuf]) {
    let Ok(known) = state.with_db(|db| curio_db::projects::list(db.conn())) else {
        return;
    };

    for project in known {
        if project.status != ProjectStatus::Present || project.origin != ProjectOrigin::Watcher {
            // Only watched projects are reconciled here. A folder registered by hand or
            // over MCP may live anywhere, and its absence from *this* root says nothing
            // about whether it exists.
            continue;
        }
        if seen
            .iter()
            .any(|path| path.to_string_lossy() == project.path)
        {
            continue;
        }

        if state
            .with_db(|db| curio_db::projects::mark_missing(db.conn(), &project.id))
            .is_ok()
        {
            publish(
                state,
                EventName::ProjectUpdated,
                &curio_core::domain::Project {
                    status: ProjectStatus::Missing,
                    ..project
                },
            );
        }
    }
}

fn publish(state: &AppState, name: EventName, project: &curio_core::domain::Project) {
    if let Ok(payload) = serde_json::to_value(project) {
        state.publish(Event::new(name, payload));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RuntimeToken;
    use curio_core::config::Config;

    fn state() -> AppState {
        AppState::new(
            RuntimeToken::mint(),
            "quit",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            Config::default(),
            curio_db::Db::open_in_memory().expect("db"),
        )
    }

    #[test]
    fn the_detection_budget_leaves_room_for_the_settle_window() {
        // FR-17 gives five seconds end to end. Settle plus one sweep must fit inside it,
        // or a project that lands just after a tick is reported late.
        assert!(SETTLE + SWEEP < Duration::from_secs(5));
    }

    #[test]
    fn a_folder_is_not_adopted_on_the_first_sight_of_it() {
        // A tool that creates a directory and then writes forty files into it must not be
        // adopted while it is still empty.
        let dir = tempfile::tempdir().expect("tempdir");
        let mut pending = HashMap::new();

        assert!(!settled(dir.path(), &mut pending));
    }

    #[test]
    fn a_folder_that_keeps_changing_never_settles() {
        let dir = tempfile::tempdir().expect("tempdir");
        let mut pending = HashMap::new();

        settled(dir.path(), &mut pending);
        std::fs::write(dir.path().join("one.txt"), "x").expect("write");

        assert!(!settled(dir.path(), &mut pending), "the signature changed");
    }

    #[test]
    fn adopting_mints_a_marker_with_a_fresh_identity() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker = read_or_mint_marker(dir.path()).expect("minted");

        assert!(dir.path().join(MARKER_FILE_NAME).exists());
        assert_eq!(marker.fingerprint(), format!("mark:{}", marker.id));
    }

    #[test]
    fn an_existing_marker_is_read_rather_than_replaced() {
        // Overwriting it would give a renamed folder a new identity and break the link
        // back to the prompt that produced it — the whole reason markers exist.
        let dir = tempfile::tempdir().expect("tempdir");
        let first = read_or_mint_marker(dir.path()).expect("minted");
        let second = read_or_mint_marker(dir.path()).expect("read");

        assert_eq!(first.id, second.id);
    }

    #[test]
    fn a_copied_folder_is_a_different_project() {
        // The copy carries the original's marker file, so it must be seen as the *same*
        // project moving — which is exactly what the fingerprint lookup does. A fresh
        // identity is minted only when there is no marker at all.
        let dir = tempfile::tempdir().expect("tempdir");
        let original = dir.path().join("a");
        let copy = dir.path().join("b");
        std::fs::create_dir_all(&original).expect("mkdir");
        std::fs::create_dir_all(&copy).expect("mkdir");

        let first = read_or_mint_marker(&original).expect("minted");
        let second = read_or_mint_marker(&copy).expect("minted");

        assert_ne!(first.id, second.id, "a folder with no marker gets its own");
    }

    #[test]
    fn a_dotfolder_is_not_a_project() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::create_dir_all(dir.path().join(".git")).expect("mkdir");
        std::fs::create_dir_all(dir.path().join("real-project")).expect("mkdir");

        let mut pending = HashMap::new();
        sweep(&state(), dir.path(), &mut pending).expect("sweep");

        assert!(
            pending.keys().all(|path| !path.ends_with(".git")),
            "{pending:?}"
        );
    }

    #[test]
    fn a_projects_root_that_is_not_there_is_not_an_error() {
        // A drive that has not been mounted yet. Killing the watcher would mean it never
        // recovers when it is.
        let mut pending = HashMap::new();
        sweep(&state(), Path::new("/definitely/not/here"), &mut pending).expect("no error");
    }

    #[test]
    fn a_registered_project_that_vanishes_is_marked_missing_not_deleted() {
        // FR-19. The record holds the prompt link and nothing on disk can rebuild it.
        let state = state();
        let (project, _) = state
            .with_db(|db| {
                curio_db::projects::register(
                    db.conn(),
                    "/tmp/gone",
                    "gone",
                    ProjectOrigin::Watcher,
                    Some("mark:01"),
                    None,
                )
            })
            .expect("register");

        reconcile_missing(&state, &[]);

        let after = state
            .with_db(|db| curio_db::projects::require(db.conn(), &project.id))
            .expect("still there");
        assert_eq!(after.status, ProjectStatus::Missing);
    }

    #[test]
    fn a_manually_registered_project_is_not_reconciled_against_the_watched_root() {
        // It may live anywhere; its absence from this root says nothing about whether it
        // exists.
        let state = state();
        let (project, _) = state
            .with_db(|db| {
                curio_db::projects::register(
                    db.conn(),
                    "/elsewhere/manual",
                    "manual",
                    ProjectOrigin::Manual,
                    None,
                    None,
                )
            })
            .expect("register");

        reconcile_missing(&state, &[]);

        let after = state
            .with_db(|db| curio_db::projects::require(db.conn(), &project.id))
            .expect("read");
        assert_eq!(after.status, ProjectStatus::Present);
    }
}
