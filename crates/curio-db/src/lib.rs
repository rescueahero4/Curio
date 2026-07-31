//! Storage: opening the library, migrating it, and (from P2) reading and writing it.
//!
//! One SQLite file plays every structural role — relational now, vector and graph
//! post-v1 (D3, D7). One file means one backup artifact, one writer, and hybrid queries
//! that never cross a process boundary.
//!
//! ## One writer
//!
//! The service thread owns the write connection (R-DA-8). That is not a performance
//! choice; it is what makes "the database is the source of truth" checkable. Sidecars and
//! prompt snapshots are **write-only projections** regenerated inside the same transaction
//! as the mutation that changed them, so on any disagreement the database wins by
//! construction rather than by convention (R-DA-4).
//!
//! It is also why the MCP stdio transport is a proxy to the live instance rather than a
//! second process opening the same file (D24).

pub mod fts;
pub mod items;
pub mod jobs;
pub mod migrations;
pub mod projects;
pub mod prompts;
pub mod sidecars;
pub mod vocabulary;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

/// What can go wrong at this layer.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The database was written by a newer build.
    ///
    /// A refusal, not a warning: opening it anyway would mean writing an old schema over
    /// newer data, which is the one failure mode that loses work (R-DA-18).
    #[error(
        "this library was written by a newer version of Curio (schema v{found}; \
         this build understands v{supported}). Update Curio rather than risk writing \
         to it with an older schema."
    )]
    SchemaTooNew { found: i64, supported: i64 },

    /// A row the caller named does not exist.
    #[error("{kind} {id} not found")]
    NotFound { kind: &'static str, id: String },

    /// The request was well-formed and refused.
    #[error("{0}")]
    Invalid(String),

    /// The request collides with something already stored.
    ///
    /// Distinct from [`Error::Invalid`] because the transports render it differently — a
    /// 409 rather than a 400 — and because the fix is usually a different action (merge
    /// rather than rename), not a corrected field.
    #[error("{0}")]
    Conflict(String),

    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("io: {0}")]
    Io(#[from] std::io::Error),

    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<Error> for curio_core::Error {
    /// Flatten to the domain's error type.
    ///
    /// The domain must never type a `rusqlite::Error` (R-DEL-2), so storage failures cross
    /// the boundary as text. [`Error::SchemaTooNew`] keeps its full message because it is
    /// the one storage failure a user is expected to act on.
    fn from(err: Error) -> Self {
        match err {
            // These three are domain refusals that happen to be detected in SQL. Flattening
            // them to `Storage` would render a duplicate tag name as a 500 instead of the
            // 409 the user needs to see.
            Error::NotFound { kind, id } => curio_core::Error::NotFound { kind, id },
            Error::Invalid(message) => curio_core::Error::Invalid(message),
            Error::Conflict(message) => curio_core::Error::Conflict(message),
            other => curio_core::Error::Storage(other.to_string()),
        }
    }
}

/// A handle to the library.
#[derive(Debug)]
pub struct Db {
    conn: Connection,
    /// The data root, which is the directory holding `library.db` (R-DA-1).
    ///
    /// This crate writes files as well as rows because the sidecar contract requires it:
    /// `item.md` is regenerated **inside the same transaction** as the mutation that
    /// changed the item (R-DA-4). Splitting the two across crates would mean a committed
    /// row with an un-rewritten file whenever the process died between them — the exact
    /// disagreement the projection rule exists to make impossible.
    ///
    /// `None` for in-memory libraries, where there is nowhere to write and nothing to
    /// project.
    data_root: Option<PathBuf>,
}

impl Db {
    /// Open (or create) the library at `path`, apply the baseline, and migrate.
    ///
    /// The order matters and is the same one the previous implementation used: pragmas,
    /// then the baseline (idempotent), then the migration chain, then the search index.
    ///
    /// Nothing here writes `runtime.json`. That file is written only after **both**
    /// migrations and the socket bind succeed, so its existence proves a healthy instance
    /// is listening and no client can ever discover a half-migrated one (R-BE-5, R-BE-33).
    ///
    /// # Errors
    /// Returns [`Error::SchemaTooNew`] for a newer database, or a storage error.
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let mut conn = Connection::open(path)?;
        configure(&conn)?;

        conn.execute_batch(migrations::BASELINE_SQL)?;
        let upgrade = migrations::run(&mut conn)?;
        if upgrade.changed_anything() {
            tracing::info!(
                from = upgrade.from,
                to = upgrade.to,
                "library schema upgraded"
            );
        }

        fts::create(&conn)?;

        Ok(Self {
            conn,
            data_root: path.parent().map(Path::to_path_buf),
        })
    }

    /// Open an in-memory library. Tests only.
    ///
    /// # Errors
    /// Propagates a storage failure.
    pub fn open_in_memory() -> Result<Self> {
        let mut conn = Connection::open_in_memory()?;
        configure(&conn)?;
        conn.execute_batch(migrations::BASELINE_SQL)?;
        migrations::run(&mut conn)?;
        fts::create(&conn)?;
        Ok(Self {
            conn,
            data_root: None,
        })
    }

    #[must_use]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[must_use]
    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// The directory holding the library and everything the user owns.
    #[must_use]
    pub fn data_root(&self) -> Option<&Path> {
        self.data_root.as_deref()
    }

    /// Point this handle at a data root. Tests only — the open path derives it.
    pub fn set_data_root(&mut self, root: impl Into<PathBuf>) {
        self.data_root = Some(root.into());
    }

    /// The schema version this library is stamped with.
    ///
    /// # Errors
    /// Propagates a storage failure.
    pub fn schema_version(&self) -> Result<i64> {
        migrations::current_version(&self.conn)
    }

    /// Flush the write-ahead log into the main database file.
    ///
    /// Run on pause and on quit (R-DA-9). Without it, a library's recent writes live in
    /// `library.db-wal`, and a user who backs up only `library.db` — the obvious thing to
    /// do — silently copies a stale file.
    ///
    /// # Errors
    /// Propagates a storage failure.
    pub fn checkpoint(&self) -> Result<()> {
        self.conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")?;
        Ok(())
    }
}

/// Apply the connection pragmas (R-DA-8).
fn configure(conn: &Connection) -> Result<()> {
    // WAL keeps readers from blocking the writer, which is what lets the watcher, the
    // jobs worker, and HTTP handlers share one file without stalling each other.
    // `journal_mode` returns the mode it settled on, so it is a query, not an update.
    // An in-memory database reports "memory" and cannot use WAL, which is fine — the
    // pragma is advisory there.
    let _mode: String = conn.query_row("PRAGMA journal_mode = WAL", [], |row| row.get(0))?;

    // NORMAL trades a fsync per commit for a fsync per checkpoint. With WAL that is
    // durable against process crash — only power loss can lose the most recent commits,
    // and this is a local library, not a ledger.
    conn.execute_batch("PRAGMA synchronous = NORMAL")?;

    // Not the default in SQLite, and the schema leans on it: item links are ON DELETE
    // CASCADE, so deleting an item without this would leave orphaned rows in three link
    // tables.
    conn.execute_batch("PRAGMA foreign_keys = ON")?;

    // The writer holds the lock briefly; five seconds is far longer than any transaction
    // here takes, so hitting this timeout means something is genuinely wrong.
    conn.execute_batch("PRAGMA busy_timeout = 5000")?;

    // Negative means KiB rather than pages: cap the page cache at 2 MB. This is a line
    // item in the ≤ 25 MB idle RSS budget (R-BE-31), and the default cache is large
    // enough to matter against it.
    conn.execute_batch("PRAGMA cache_size = -2000")?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_new_library_opens_at_the_latest_version() {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Db::open(&dir.path().join("library.db")).expect("open");

        assert_eq!(
            db.schema_version().expect("version"),
            migrations::LATEST_VERSION
        );
    }

    #[test]
    fn reopening_a_library_is_a_no_op() {
        // Every launch does this. It must not re-run migrations or fail on the baseline.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("library.db");

        Db::open(&path).expect("first open");
        let second = Db::open(&path).expect("second open");

        assert_eq!(
            second.schema_version().expect("version"),
            migrations::LATEST_VERSION
        );
    }

    #[test]
    fn foreign_keys_are_enforced() {
        // The schema's ON DELETE CASCADE links are inert without this pragma, and SQLite
        // defaults it off — so a missing line here would silently orphan link rows rather
        // than fail visibly.
        let db = Db::open_in_memory().expect("open");
        let enabled: i64 = db
            .conn()
            .query_row("PRAGMA foreign_keys", [], |row| row.get(0))
            .expect("query");

        assert_eq!(enabled, 1);
    }

    #[test]
    fn deleting_an_item_cascades_to_its_links() {
        let db = Db::open_in_memory().expect("open");
        db.conn()
            .execute_batch(
                "INSERT INTO items (id, name, screenshot_path, created_at, updated_at)
                   VALUES ('01A', 'x', 'items/01A/screenshot.png', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                 INSERT INTO tags (id, name, created_by) VALUES ('01T', 'brutalist', 'ai');
                 INSERT INTO item_tags (item_id, tag_id) VALUES ('01A', '01T');
                 DELETE FROM items WHERE id = '01A';",
            )
            .expect("seed and delete");

        let orphans: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM item_tags", [], |row| row.get(0))
            .expect("count");

        assert_eq!(orphans, 0);
    }

    #[test]
    fn tags_are_case_insensitive() {
        // `tags.name` is UNIQUE COLLATE NOCASE. "Brutalist" and "brutalist" are one tag —
        // a distinction no user ever intends to draw, and one that would otherwise
        // fragment the filter list.
        let db = Db::open_in_memory().expect("open");
        db.conn()
            .execute_batch(
                "INSERT INTO tags (id, name, created_by) VALUES ('01T', 'Brutalist', 'ai')",
            )
            .expect("insert");

        let clash = db.conn().execute_batch(
            "INSERT INTO tags (id, name, created_by) VALUES ('01U', 'brutalist', 'ai')",
        );

        assert!(clash.is_err(), "differing only in case must collide");
    }

    #[test]
    fn item_status_is_constrained() {
        // The CHECK constraint is what makes curio-core's ItemStatus strings load-bearing.
        let db = Db::open_in_memory().expect("open");
        let bad = db.conn().execute_batch(
            "INSERT INTO items (id, name, screenshot_path, status, created_at, updated_at)
               VALUES ('01A', 'x', 'p', 'almost_ready', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
        );

        assert!(bad.is_err());
    }

    #[test]
    fn every_status_curio_core_knows_is_accepted_by_the_database() {
        // The other half of the previous test: the domain enum and the CHECK constraint
        // must agree in both directions, or a legal state becomes unwritable.
        let db = Db::open_in_memory().expect("open");
        for (index, status) in curio_core::domain::ItemStatus::all().iter().enumerate() {
            db.conn()
                .execute(
                    "INSERT INTO items (id, name, screenshot_path, status, created_at, updated_at)
                       VALUES (?1, 'x', 'p', ?2, '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z')",
                    rusqlite::params![format!("01{index}"), status.as_str()],
                )
                .unwrap_or_else(|err| panic!("{} rejected: {err}", status.as_str()));
        }
    }

    #[test]
    fn checkpointing_works_on_a_file_backed_library() {
        // Runs on pause and quit (R-DA-9). A user who backs up library.db without it
        // copies a file missing their most recent captures.
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Db::open(&dir.path().join("library.db")).expect("open");
        db.checkpoint().expect("checkpoint");
    }
}
