//! Projects: the folders the user's AI tools write.
//!
//! ## Identity is a file, not a filesystem attribute
//!
//! A project is identified by a ULID Curio mints into a `.curio-project` marker inside the
//! folder (R-BE-21). The previous scheme used the directory's inode, and **ext4 re-issues a
//! deleted directory's inode to the next directory created** — so a deleted project and an
//! unrelated new folder shared one identity, and the deleted project's record was
//! re-pointed at the stranger, carrying its `prompt_id` with it. Migration v4 exists to
//! clear those values.
//!
//! ## Missing is not deleted
//!
//! A folder that disappears marks its record `missing` (FR-19). The record holds the link
//! back to the prompt that produced it, and nothing on disk can reconstruct that link — so
//! deleting the row to match the filesystem would destroy the one thing the filesystem
//! never held.

use rusqlite::{Connection, OptionalExtension as _, Row};

use curio_core::domain::{Project, ProjectOrigin, ProjectStatus};

use crate::{Error, Result};

/// Every project, newest first.
///
/// # Errors
/// Propagates a storage failure.
pub fn list(conn: &Connection) -> Result<Vec<Project>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM projects ORDER BY detected_at DESC, id DESC"
    ))?;
    let rows = statement.query_map([], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Read one project.
///
/// # Errors
/// Propagates a storage failure.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Project>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM projects WHERE id = ?1"),
            [id],
            map_row,
        )
        .optional()?)
}

/// Read one project or fail.
///
/// # Errors
/// Returns [`Error::NotFound`] if there is no such project.
pub fn require(conn: &Connection, id: &str) -> Result<Project> {
    get(conn, id)?.ok_or_else(|| Error::NotFound {
        kind: "project",
        id: id.to_owned(),
    })
}

/// Find a project by its marker fingerprint.
///
/// This is how a rename is told apart from a delete-plus-create: the folder moved, but the
/// marker inside it did not change.
///
/// # Errors
/// Propagates a storage failure.
pub fn by_fingerprint(conn: &Connection, fingerprint: &str) -> Result<Option<Project>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM projects WHERE fingerprint = ?1"),
            [fingerprint],
            map_row,
        )
        .optional()?)
}

/// Find a project by its path.
///
/// # Errors
/// Propagates a storage failure.
pub fn by_path(conn: &Connection, path: &str) -> Result<Option<Project>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM projects WHERE path = ?1"),
            [path],
            map_row,
        )
        .optional()?)
}

/// Register a folder, or re-point an existing record at it.
///
/// Three cases, in the order they are checked:
///
/// 1. **Known marker.** The folder was renamed or moved: update the path, mark it present.
/// 2. **Known path.** The same folder, seen again: mark it present and backfill the
///    fingerprint if the record predates marker identity.
/// 3. **New.** Insert.
///
/// A fingerprint is **never overwritten** once set (Inventory §10.17). Changing the format
/// is a migration, not a scan side effect — the whole point of v4 was that a silently
/// re-derived identity is unauditable.
///
/// # Errors
/// Propagates a storage failure.
pub fn register(
    conn: &Connection,
    path: &str,
    name: &str,
    origin: ProjectOrigin,
    fingerprint: Option<&str>,
    prompt_id: Option<&str>,
) -> Result<(Project, bool)> {
    let now = curio_core::time::now_iso();

    if let Some(fingerprint) = fingerprint
        && let Some(existing) = by_fingerprint(conn, fingerprint)?
    {
        conn.execute(
            "UPDATE projects SET path = ?2, name = ?3, status = 'present' WHERE id = ?1",
            rusqlite::params![existing.id, path, name],
        )?;
        return Ok((require(conn, &existing.id)?, false));
    }

    if let Some(existing) = by_path(conn, path)? {
        conn.execute(
            "UPDATE projects SET status = 'present', fingerprint = COALESCE(fingerprint, ?2)
              WHERE id = ?1",
            rusqlite::params![existing.id, fingerprint],
        )?;
        return Ok((require(conn, &existing.id)?, false));
    }

    let id = curio_core::ids::generate();
    conn.execute(
        "INSERT INTO projects (id, name, path, origin, prompt_id, status, fingerprint, detected_at)
           VALUES (?1, ?2, ?3, ?4, ?5, 'present', ?6, ?7)",
        rusqlite::params![id, name, path, origin.as_str(), prompt_id, fingerprint, now],
    )?;
    Ok((require(conn, &id)?, true))
}

/// Mark a project missing. Never deletes (FR-19).
///
/// # Errors
/// Propagates a storage failure.
pub fn mark_missing(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("UPDATE projects SET status = 'missing' WHERE id = ?1", [id])?;
    Ok(())
}

/// Link a project to the prompt that produced it.
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown project, or a storage failure.
pub fn set_prompt(conn: &Connection, id: &str, prompt_id: Option<&str>) -> Result<Project> {
    require(conn, id)?;
    conn.execute(
        "UPDATE projects SET prompt_id = ?2 WHERE id = ?1",
        rusqlite::params![id, prompt_id],
    )?;
    require(conn, id)
}

/// Record that the project was opened.
///
/// # Errors
/// Propagates a storage failure.
pub fn mark_opened(conn: &Connection, id: &str) -> Result<()> {
    conn.execute(
        "UPDATE projects SET last_opened_at = ?2 WHERE id = ?1",
        rusqlite::params![id, curio_core::time::now_iso()],
    )?;
    Ok(())
}

/// Delete a project record. The folder on disk is never touched.
///
/// # Errors
/// Propagates a storage failure.
pub fn forget(conn: &Connection, id: &str) -> Result<()> {
    conn.execute("DELETE FROM projects WHERE id = ?1", [id])?;
    Ok(())
}

const COLUMNS: &str =
    "id, name, path, origin, prompt_id, status, fingerprint, detected_at, last_opened_at";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Project> {
    Ok(Project {
        id: row.get(0)?,
        name: row.get(1)?,
        path: row.get(2)?,
        origin: match row.get::<_, String>(3)?.as_str() {
            "mcp" => ProjectOrigin::Mcp,
            "manual" => ProjectOrigin::Manual,
            _ => ProjectOrigin::Watcher,
        },
        prompt_id: row.get(4)?,
        status: if row.get::<_, String>(5)? == "missing" {
            ProjectStatus::Missing
        } else {
            ProjectStatus::Present
        },
        fingerprint: row.get(6)?,
        detected_at: row.get(7)?,
        last_opened_at: row.get(8)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn register_new(db: &Db, path: &str, fingerprint: Option<&str>) -> Project {
        register(
            db.conn(),
            path,
            "proj",
            ProjectOrigin::Watcher,
            fingerprint,
            None,
        )
        .expect("register")
        .0
    }

    #[test]
    fn registering_a_new_folder_reports_it_as_new() {
        let db = Db::open_in_memory().expect("open");
        let (project, is_new) = register(
            db.conn(),
            "/tmp/a",
            "a",
            ProjectOrigin::Watcher,
            Some("mark:01"),
            None,
        )
        .expect("register");

        assert!(is_new, "the watcher publishes project.detected off this");
        assert_eq!(project.status, ProjectStatus::Present);
        assert_eq!(project.origin, ProjectOrigin::Watcher);
    }

    #[test]
    fn a_renamed_folder_keeps_its_identity_and_its_prompt() {
        // The whole reason identity is a marker file: a rename must not look like a delete
        // plus a create, because the new record would have no prompt link and nothing on
        // disk could rebuild one.
        let db = Db::open_in_memory().expect("open");
        let prompt = crate::prompts::create(db.conn(), None).expect("prompt");
        let original = register(
            db.conn(),
            "/tmp/old-name",
            "old-name",
            ProjectOrigin::Watcher,
            Some("mark:01"),
            Some(&prompt.id),
        )
        .expect("register")
        .0;

        let (moved, is_new) = register(
            db.conn(),
            "/tmp/new-name",
            "new-name",
            ProjectOrigin::Watcher,
            Some("mark:01"),
            None,
        )
        .expect("re-register");

        assert!(!is_new);
        assert_eq!(moved.id, original.id);
        assert_eq!(moved.path, "/tmp/new-name");
        assert_eq!(moved.prompt_id.as_deref(), Some(prompt.id.as_str()));
    }

    #[test]
    fn seeing_the_same_path_again_does_not_duplicate_it() {
        let db = Db::open_in_memory().expect("open");
        let first = register_new(&db, "/tmp/a", None);
        let (second, is_new) =
            register(db.conn(), "/tmp/a", "a", ProjectOrigin::Watcher, None, None).expect("again");

        assert!(!is_new);
        assert_eq!(first.id, second.id);
        assert_eq!(list(db.conn()).expect("list").len(), 1);
    }

    #[test]
    fn a_record_predating_marker_identity_gets_one_backfilled() {
        // Migration v4 cleared every unsound fingerprint, so real libraries have records
        // with none. They must become rename-followable the next time they are scanned.
        let db = Db::open_in_memory().expect("open");
        let bare = register_new(&db, "/tmp/a", None);
        assert!(bare.fingerprint.is_none());

        register(
            db.conn(),
            "/tmp/a",
            "a",
            ProjectOrigin::Watcher,
            Some("mark:01"),
            None,
        )
        .expect("rescan");

        assert_eq!(
            require(db.conn(), &bare.id)
                .expect("read")
                .fingerprint
                .as_deref(),
            Some("mark:01")
        );
    }

    #[test]
    fn an_existing_fingerprint_is_never_overwritten() {
        // Inventory §10.17. Changing the format is a migration, not a scan side effect —
        // a silently re-derived identity is unauditable, which is what made the inode bug
        // so hard to see.
        let db = Db::open_in_memory().expect("open");
        let original = register_new(&db, "/tmp/a", Some("mark:01"));

        register(
            db.conn(),
            "/tmp/a",
            "a",
            ProjectOrigin::Watcher,
            Some("mark:99"),
            None,
        )
        .expect("rescan");

        assert_eq!(
            require(db.conn(), &original.id)
                .expect("read")
                .fingerprint
                .as_deref(),
            Some("mark:01")
        );
    }

    #[test]
    fn a_vanished_folder_is_marked_missing_not_deleted() {
        // FR-19. The record holds the prompt link, and nothing on disk can rebuild it.
        let db = Db::open_in_memory().expect("open");
        let project = register_new(&db, "/tmp/a", Some("mark:01"));

        mark_missing(db.conn(), &project.id).expect("missing");

        let after = require(db.conn(), &project.id).expect("still there");
        assert_eq!(after.status, ProjectStatus::Missing);
    }

    #[test]
    fn a_missing_folder_that_reappears_comes_back_present() {
        let db = Db::open_in_memory().expect("open");
        let project = register_new(&db, "/tmp/a", Some("mark:01"));
        mark_missing(db.conn(), &project.id).expect("missing");

        register_new(&db, "/tmp/a", Some("mark:01"));

        assert_eq!(
            require(db.conn(), &project.id).expect("read").status,
            ProjectStatus::Present
        );
    }

    #[test]
    fn a_copy_of_a_folder_is_a_different_project() {
        // Copies get a fresh identity: the marker is re-minted on adoption, so the copy
        // arrives here with a different fingerprint and must not adopt the original's
        // record.
        let db = Db::open_in_memory().expect("open");
        register_new(&db, "/tmp/a", Some("mark:01"));
        let copy = register_new(&db, "/tmp/a-copy", Some("mark:02"));

        assert_eq!(list(db.conn()).expect("list").len(), 2);
        assert_eq!(copy.path, "/tmp/a-copy");
    }

    #[test]
    fn an_mcp_registration_carries_no_fingerprint() {
        // R-MCP-1 / Inventory §10.17: identity is minted on adoption, never on
        // registration, so an MCP-registered project is deliberately not rename-followable.
        let db = Db::open_in_memory().expect("open");
        let (project, _) = register(
            db.conn(),
            "/tmp/agent",
            "agent",
            ProjectOrigin::Mcp,
            None,
            None,
        )
        .expect("register");

        assert_eq!(project.origin, ProjectOrigin::Mcp);
        assert!(project.fingerprint.is_none());
    }

    #[test]
    fn linking_an_unknown_project_to_a_prompt_is_not_found() {
        let db = Db::open_in_memory().expect("open");
        assert!(matches!(
            set_prompt(db.conn(), "01NOPE", None),
            Err(Error::NotFound { .. })
        ));
    }
}
