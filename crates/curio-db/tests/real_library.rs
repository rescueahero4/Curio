//! Opening a real, shipped library.
//!
//! P2's exit criterion is that an existing `library.db` opens losslessly and round-trips
//! on the existing chain (D20, R-DEL-21). Unit tests build their fixtures from our own
//! baseline, which proves the migrations are self-consistent but cannot prove they match
//! what the previous implementation actually wrote — the two only diverge on details
//! nobody transcribed wrongly on purpose.
//!
//! This test closes that gap by opening a copy of a genuine vault. It is **skipped unless
//! `CURIO_TEST_LIBRARY` names one**, so a fresh clone and CI stay green without shipping
//! someone's library as a fixture:
//!
//! ```text
//! CURIO_TEST_LIBRARY=~/Curio/library.db cargo test -p curio-db --test real_library
//! ```
//!
//! It always works on a **copy**, including the `-wal` and `-shm` sidecars. Opening the
//! original would migrate a user's real library as a side effect of running tests, and
//! copying `library.db` alone would silently drop every write still sitting in the WAL.

use std::path::{Path, PathBuf};

use curio_db::{Db, migrations};

#[test]
fn a_real_library_opens_and_reaches_the_latest_version() {
    let Some(source) = library_under_test() else {
        eprintln!("skipped: set CURIO_TEST_LIBRARY to a library.db to run this");
        return;
    };

    let staged = tempfile::tempdir().expect("tempdir");
    let target = stage_copy(&source, staged.path());

    // A library newer than this build refuses to open (R-DA-18), so a successful open is
    // itself the assertion that the fixture is on a chain we understand.
    let db = Db::open(&target).expect("a shipped library must open");
    let after = db.schema_version().expect("version");

    assert_eq!(
        after,
        migrations::LATEST_VERSION,
        "opening must leave the library fully migrated"
    );

    // Reopening must be inert. Every launch does it, so a migration that is not
    // idempotent would corrode a real library one start at a time.
    let reopened = Db::open(&target).expect("second open");
    assert_eq!(reopened.schema_version().expect("version"), after);
}

#[test]
fn every_table_the_domain_expects_is_present_in_a_real_library() {
    let Some(source) = library_under_test() else {
        eprintln!("skipped: set CURIO_TEST_LIBRARY to a library.db to run this");
        return;
    };

    let staged = tempfile::tempdir().expect("tempdir");
    let db = Db::open(&stage_copy(&source, staged.path())).expect("open");

    // Named rather than derived from the baseline: this asserts the *shipped* shape, so
    // that a table quietly dropped from our own baseline would still fail here.
    for table in [
        "items",
        "aesthetic_families",
        "design_types",
        "tags",
        "item_families",
        "item_types",
        "item_tags",
        "prompts",
        "projects",
        "jobs",
        "items_fts",
    ] {
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE name = ?1",
                [table],
                |row| row.get(0),
            )
            .expect("query");
        assert_eq!(count, 1, "{table} is missing from a real library");
    }
}

#[test]
fn a_real_library_survives_a_read_after_migration() {
    let Some(source) = library_under_test() else {
        eprintln!("skipped: set CURIO_TEST_LIBRARY to a library.db to run this");
        return;
    };

    let staged = tempfile::tempdir().expect("tempdir");
    let db = Db::open(&stage_copy(&source, staged.path())).expect("open");

    // Reading every status back through the domain enum proves the two agree on a real
    // corpus, not just on rows we wrote ourselves.
    let mut statement = db
        .conn()
        .prepare("SELECT DISTINCT status FROM items")
        .expect("prepare");
    let statuses: Vec<String> = statement
        .query_map([], |row| row.get::<_, String>(0))
        .expect("query")
        .map(|row| row.expect("row"))
        .collect();

    let known: Vec<&str> = curio_core::domain::ItemStatus::all()
        .iter()
        .map(|status| status.as_str())
        .collect();
    for status in &statuses {
        assert!(
            known.contains(&status.as_str()),
            "a real library holds status {status:?}, which curio-core does not know"
        );
    }

    let items: i64 = db
        .conn()
        .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
        .expect("count");
    eprintln!(
        "read {items} items across {} distinct statuses",
        statuses.len()
    );
}

/// The library named by `CURIO_TEST_LIBRARY`, if it exists.
fn library_under_test() -> Option<PathBuf> {
    let raw = std::env::var_os("CURIO_TEST_LIBRARY")?;
    let path = PathBuf::from(raw);
    path.is_file().then_some(path)
}

/// Copy the database **and its WAL sidecars** into `into`, returning the copy's path.
///
/// The sidecars are not optional: SQLite keeps committed-but-uncheckpointed transactions
/// in `-wal`, so copying `library.db` alone produces a database that opens cleanly and is
/// quietly missing the most recent captures — the exact failure this test exists to rule
/// out.
fn stage_copy(source: &Path, into: &Path) -> PathBuf {
    let name = source.file_name().expect("file name");
    let target = into.join(name);
    std::fs::copy(source, &target).expect("copy database");

    for suffix in ["-wal", "-shm"] {
        let sidecar = with_suffix(source, suffix);
        if sidecar.is_file() {
            std::fs::copy(&sidecar, with_suffix(&target, suffix)).expect("copy sidecar");
        }
    }
    target
}

fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(suffix);
    path.with_file_name(name)
}
