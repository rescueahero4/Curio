//! Schema migrations.
//!
//! Versioning uses SQLite's own `PRAGMA user_version`, which needs no bookkeeping table
//! and is written atomically inside the migration's transaction. Each migration runs in
//! its own transaction together with its version bump, so an interrupted upgrade leaves
//! the database at the last **fully applied** version rather than half-migrated
//! (R-DA-18).
//!
//! ## The chain is carried over, not restarted
//!
//! Real users have real libraries. The Rust implementation adopts the shipped database
//! lineage: an existing `library.db` opens here and migrates forward on the same chain it
//! was already on (D20). The four migrations below are the shipped ones, transcribed —
//! including v4, whose reason is worth reading before anyone is tempted to tidy it away.
//!
//! ## Two failure modes, one loud
//!
//! A database **newer** than this build refuses to open ([`Error::SchemaTooNew`]). Silently
//! writing an old schema over newer data is the one failure that loses work.
//!
//! Either failure is a boot failure: the app exits visibly and `runtime.json` is never
//! written, so no client can discover a half-migrated instance (R-BE-33, R-BE-5).

use rusqlite::Connection;

use crate::{Error, Result};

/// The v1 baseline. Every statement is `IF NOT EXISTS`, so applying it to an existing
/// library is a no-op — which is why the open path can run it unconditionally.
pub const BASELINE_SQL: &str = include_str!("../sql/schema.sql");

/// The newest schema version this build understands.
pub const LATEST_VERSION: i64 = 4;

/// One step in the chain.
struct Migration {
    version: i64,
    description: &'static str,
    up: fn(&Connection) -> rusqlite::Result<()>,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        description: "Baseline — schema.sql as shipped in 0.1.0",
        // Intentionally empty. The baseline has already created (or verified) every
        // table; this entry exists to stamp a version onto both fresh databases and
        // pre-versioning ones, which are identical in shape.
        up: |_| Ok(()),
    },
    Migration {
        version: 2,
        description: "job results, prompt send claims, project fingerprints",
        up: |conn| {
            // `jobs.result` carries progress while a job runs and its summary when it
            // finishes. Bulk operations need both: the UI shows "31 of 240" to make a long
            // run cancellable with confidence, and a consistency pass has an outcome —
            // suggested merges — that is the whole point of running it.
            add_column(conn, "jobs", "result", "TEXT")?;

            // `prompts.sent_at` is an outstanding claim on the next project folder. A
            // project card should link back to the prompt that produced it, but the
            // watcher only ever sees a directory appear — nothing in it names a prompt.
            // Send to Claude is the one moment both are known, so it records the intent
            // and the watcher spends it.
            add_column(conn, "prompts", "sent_at", "TEXT")?;

            // `projects.fingerprint` identifies a folder independently of its path, so a
            // rename can be told apart from a delete-plus-create. Recorded at registration
            // because a deleted directory can no longer be stat'ed.
            add_column(conn, "projects", "fingerprint", "TEXT")
        },
    },
    Migration {
        version: 3,
        description: "park jobs until a resume time instead of retrying them",
        up: |conn| {
            // `jobs.not_before` lets a job be parked until a time rather than retried
            // immediately. A bulk run submitted to the Batch API may wait minutes or hours
            // for a result; polling it inside the worker would block every assessment
            // behind it, and requeueing without a delay would spin.
            //
            // This is a separate version rather than part of v2 because v2 had already
            // been applied to a real database by the time parking was added. Editing an
            // applied migration is the one thing this runner exists to prevent: the column
            // would reach fresh installs and never reach upgraded ones. It was caught
            // exactly that way — a library stamped v2 was missing the column.
            add_column(conn, "jobs", "not_before", "TEXT")
        },
    },
    Migration {
        version: 4,
        description: "retire inode-derived project fingerprints",
        up: |conn| {
            // `projects.fingerprint` used to hold `ino:<inode>` (or `born:<ms>`).
            // **ext4 re-issues a deleted directory's inode to the next directory
            // created**, so a deleted project and an unrelated new folder shared one
            // fingerprint, and the deleted project's record was re-pointed at the
            // stranger — carrying its `prompt_id`, with nothing on disk able to
            // reconstruct the link. Identity is now a ULID Curio mints into the folder
            // itself (`mark:<ulid>`).
            //
            // This migration exists because the project upsert backfills a fingerprint
            // only when the row has none. Without it, every project already in a real
            // library would keep its `ino:` string forever, never be re-derived, and stay
            // matchable on the unsound value.
            //
            // Accepted loss, stated rather than hidden: a project already `missing` when
            // this runs is left with NULL and can no longer be relocated. Its folder is
            // gone, so there is no marker to read — and keeping its `ino:` value would
            // preserve the defect for precisely the records most exposed to it.
            //
            // `NOT LIKE 'mark:%'` rather than a blanket clear, so re-running against a
            // hand-patched database cannot destroy identities already migrated.
            conn.execute_batch(
                "UPDATE projects SET fingerprint = NULL
                  WHERE fingerprint IS NOT NULL AND fingerprint NOT LIKE 'mark:%'",
            )
        },
    },
];

/// What an upgrade did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Upgrade {
    pub from: i64,
    pub to: i64,
    pub applied: Vec<i64>,
}

impl Upgrade {
    #[must_use]
    pub fn changed_anything(&self) -> bool {
        !self.applied.is_empty()
    }
}

/// The version stamped on this database. Zero means pre-versioning or brand new.
///
/// # Errors
/// Propagates a SQLite failure.
pub fn current_version(conn: &Connection) -> Result<i64> {
    Ok(conn.pragma_query_value(None, "user_version", |row| row.get(0))?)
}

/// Bring the database up to [`LATEST_VERSION`].
///
/// # Errors
/// Returns [`Error::SchemaTooNew`] if the database was written by a newer build, or a
/// storage error if a step fails.
pub fn run(conn: &mut Connection) -> Result<Upgrade> {
    let from = current_version(conn)?;

    if from > LATEST_VERSION {
        return Err(Error::SchemaTooNew {
            found: from,
            supported: LATEST_VERSION,
        });
    }
    if from == LATEST_VERSION {
        return Ok(Upgrade {
            from,
            to: from,
            applied: Vec::new(),
        });
    }

    let mut applied = Vec::new();
    for migration in MIGRATIONS {
        if migration.version <= from {
            continue;
        }

        let tx = conn.transaction()?;
        (migration.up)(&tx)?;
        // PRAGMA does not accept bound parameters. The value is a literal from our own
        // migration list, never user input.
        tx.pragma_update(None, "user_version", migration.version)?;
        tx.commit()?;

        applied.push(migration.version);
        if migration.version > 1 {
            // Deliberately a tracing event and not a println. `--mcp-stdio` reserves
            // stdout for protocol frames, and the database opens before the transport
            // connects — a first-time user's agent used to receive these lines ahead of
            // every JSON-RPC frame (R-MCP-5, stdout purity).
            tracing::info!(
                version = migration.version,
                description = migration.description,
                "migrated"
            );
        }
    }

    Ok(Upgrade {
        from,
        to: current_version(conn)?,
        applied,
    })
}

/// Add a column only if it is missing, so a migration survives being re-run against a
/// hand-patched or partially upgraded database.
fn add_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    if column_exists(conn, table, column)? {
        return Ok(());
    }
    // Identifiers cannot be bound as parameters. These are literals from the migration
    // list above — never user input — and `quote_ident` rejects anything that is not a
    // plain identifier, so a future migration cannot smuggle SQL through this helper.
    conn.execute_batch(&format!(
        "ALTER TABLE {} ADD COLUMN {} {definition}",
        quote_ident(table),
        quote_ident(column),
    ))
}

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut statement = conn.prepare(&format!("PRAGMA table_info({})", quote_ident(table)))?;
    let mut rows = statement.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn quote_ident(name: &str) -> String {
    assert!(
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') && !name.is_empty(),
        "unsafe SQL identifier: {name}"
    );
    format!("\"{name}\"")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh() -> Connection {
        let conn = Connection::open_in_memory().expect("open");
        conn.execute_batch(BASELINE_SQL).expect("baseline");
        conn
    }

    /// A database genuinely at version `target`: the baseline plus every migration up to
    /// and including it, stamped accordingly.
    ///
    /// Stamping a version onto a bare baseline would be a lie — it would claim v3 while
    /// missing v2's columns, which no real library can be — and the resulting test would
    /// exercise a state that cannot occur.
    fn at_version(target: i64) -> Connection {
        let mut conn = fresh();
        for migration in MIGRATIONS {
            if migration.version > target {
                break;
            }
            let tx = conn.transaction().expect("begin");
            (migration.up)(&tx).expect("apply");
            tx.pragma_update(None, "user_version", migration.version)
                .expect("stamp");
            tx.commit().expect("commit");
        }
        assert_eq!(current_version(&conn).expect("version"), target);
        conn
    }

    #[test]
    fn a_fresh_database_reaches_the_latest_version() {
        let mut conn = fresh();
        let upgrade = run(&mut conn).expect("migrate");

        assert_eq!(upgrade.from, 0);
        assert_eq!(upgrade.to, LATEST_VERSION);
        assert_eq!(upgrade.applied, vec![1, 2, 3, 4]);
    }

    #[test]
    fn migrating_twice_is_a_no_op() {
        // Every boot runs this. The second run must not re-apply v4's UPDATE, which would
        // clear fingerprints that the first run's backfill had legitimately re-derived.
        let mut conn = fresh();
        run(&mut conn).expect("first");
        let second = run(&mut conn).expect("second");

        assert!(!second.changed_anything());
        assert_eq!(second.from, LATEST_VERSION);
    }

    #[test]
    fn a_newer_database_refuses_to_open() {
        // R-DA-18. Writing an old schema over newer data is the one failure mode that
        // loses work, so this is a refusal rather than a warning.
        let mut conn = fresh();
        conn.pragma_update(None, "user_version", LATEST_VERSION + 1)
            .expect("stamp");

        match run(&mut conn) {
            Err(Error::SchemaTooNew { found, supported }) => {
                assert_eq!(found, LATEST_VERSION + 1);
                assert_eq!(supported, LATEST_VERSION);
            }
            other => panic!("expected SchemaTooNew, got {other:?}"),
        }
    }

    #[test]
    fn upgrading_from_v2_adds_only_what_is_missing() {
        // The real upgrade path for a library that shipped before parking existed. v3's
        // note records that exactly this case was found broken in the field.
        let mut conn = at_version(2);

        let upgrade = run(&mut conn).expect("migrate");

        assert_eq!(upgrade.applied, vec![3, 4]);
        assert!(column_exists(&conn, "jobs", "not_before").expect("check"));
    }

    #[test]
    fn v2_columns_exist_after_migrating() {
        let mut conn = fresh();
        run(&mut conn).expect("migrate");

        assert!(column_exists(&conn, "jobs", "result").expect("check"));
        assert!(column_exists(&conn, "prompts", "sent_at").expect("check"));
        assert!(column_exists(&conn, "projects", "fingerprint").expect("check"));
    }

    #[test]
    fn v4_clears_unsound_fingerprints_and_keeps_marker_ones() {
        // The ext4 inode-reuse defect. `ino:` values must go; `mark:` values must survive,
        // or an upgraded library loses the identities v4 exists to protect.
        let mut conn = at_version(3);
        conn.execute_batch(
            "INSERT INTO projects (id, name, path, detected_at, fingerprint) VALUES
               ('01A', 'inode',  '/tmp/a', '2026-01-01T00:00:00Z', 'ino:12345'),
               ('01B', 'marker', '/tmp/b', '2026-01-01T00:00:00Z', 'mark:01J000000000000000000000'),
               ('01C', 'born',   '/tmp/c', '2026-01-01T00:00:00Z', 'born:1700000000000'),
               ('01D', 'none',   '/tmp/d', '2026-01-01T00:00:00Z', NULL)",
        )
        .expect("seed");

        run(&mut conn).expect("migrate");

        let read = |id: &str| -> Option<String> {
            conn.query_row(
                "SELECT fingerprint FROM projects WHERE id = ?1",
                [id],
                |row| row.get(0),
            )
            .expect("query")
        };

        assert_eq!(read("01A"), None, "ino: fingerprints are unsound");
        assert_eq!(
            read("01B").as_deref(),
            Some("mark:01J000000000000000000000"),
            "marker identities must survive the upgrade"
        );
        assert_eq!(read("01C"), None, "born: fingerprints are unsound too");
        assert_eq!(read("01D"), None);
    }

    #[test]
    fn the_baseline_is_idempotent() {
        // The open path runs it on every boot, including against a fully migrated
        // library. A single missing IF NOT EXISTS would make the app fail to start.
        let conn = fresh();
        conn.execute_batch(BASELINE_SQL).expect("second apply");
        conn.execute_batch(BASELINE_SQL).expect("third apply");
    }
}
