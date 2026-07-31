//! The three vocabularies: families, design types, tags.
//!
//! All three share CRUD and merge behaviour, differ in one table each, and only families
//! carry a description and a per-link score. [`curio_core::domain::VocabularyKind`] is what
//! lets the shared paths be written once instead of three times with two of them subtly
//! wrong.
//!
//! ## Names are the identity users see, ids are the identity rows use
//!
//! The model answers in **names** — it is shown the vocabulary as text — so every
//! write-back path resolves a name to a row, creating it if needed ([`ensure`]). Users
//! rename freely, and a rename must not orphan a single link, which is why nothing outside
//! this module ever stores a name as a foreign key.

pub mod merge;

use rusqlite::{Connection, OptionalExtension as _};

use curio_core::domain::{CreatedBy, Family, Term, VocabularyKind};

use crate::{Error, Result};

pub use merge::{merge, prune_orphans};

/// Every family, with how many items link to it.
///
/// # Errors
/// Propagates a storage failure.
pub fn list_families(conn: &Connection) -> Result<Vec<Family>> {
    let mut statement = conn.prepare(
        "SELECT f.id, f.name, f.description, f.created_by, f.created_at, f.updated_at,
                (SELECT COUNT(*) FROM item_families l WHERE l.family_id = f.id)
           FROM aesthetic_families f ORDER BY f.name COLLATE NOCASE",
    )?;
    let rows = statement.query_map([], |row| {
        Ok(Family {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            created_by: authorship(&row.get::<_, String>(3)?),
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
            item_count: row.get(6)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Every tag, or every design type, with its item count.
///
/// # Errors
/// Propagates a storage failure.
pub fn list_terms(conn: &Connection, kind: VocabularyKind) -> Result<Vec<Term>> {
    let (table, link_table, column) = shape(kind);
    let sql = format!(
        "SELECT v.id, v.name, v.created_by,
                (SELECT COUNT(*) FROM {link_table} l WHERE l.{column} = v.id)
           FROM {table} v ORDER BY v.name COLLATE NOCASE"
    );
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([], |row| {
        Ok(Term {
            id: row.get(0)?,
            name: row.get(1)?,
            created_by: authorship(&row.get::<_, String>(2)?),
            item_count: row.get(3)?,
        })
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Find a vocabulary entry by name, or create it.
///
/// Matching is **case-insensitive for every kind**, not only tags. `tags.name` enforces it
/// with `COLLATE NOCASE`, but families and types do not — and a model that returns
/// "Brutalist" today and "brutalist" tomorrow would otherwise split one family in two,
/// with the items divided between them and neither filter showing the whole set.
///
/// # Errors
/// Propagates a storage failure.
pub fn ensure(
    conn: &Connection,
    kind: VocabularyKind,
    name: &str,
    created_by: CreatedBy,
    now: &str,
) -> Result<String> {
    let (table, _, _) = shape(kind);

    let existing: Option<String> = conn
        .query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1 COLLATE NOCASE"),
            [name],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(id) = existing {
        return Ok(id);
    }

    let id = curio_core::ids::generate();
    match kind {
        VocabularyKind::Family => {
            conn.execute(
                "INSERT INTO aesthetic_families (id, name, description, created_by, created_at, updated_at)
                   VALUES (?1, ?2, '', ?3, ?4, ?4)",
                rusqlite::params![id, name, created_by.as_str(), now],
            )?;
        }
        _ => {
            conn.execute(
                &format!("INSERT INTO {table} (id, name, created_by) VALUES (?1, ?2, ?3)"),
                rusqlite::params![id, name, created_by.as_str()],
            )?;
        }
    }
    Ok(id)
}

/// Create a vocabulary entry, refusing a name that already exists.
///
/// A refusal rather than a silent no-op: a user typing a name that is already there has
/// either forgotten it exists or means a different thing by it, and both deserve to be
/// told rather than to watch nothing happen.
///
/// # Errors
/// Returns [`Error::Conflict`] if the name is taken, or a storage failure.
pub fn create(
    conn: &Connection,
    kind: VocabularyKind,
    name: &str,
    description: &str,
    now: &str,
) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        return Err(Error::Invalid("a name is required".to_owned()));
    }
    let (table, _, _) = shape(kind);
    let taken: Option<String> = conn
        .query_row(
            &format!("SELECT id FROM {table} WHERE name = ?1 COLLATE NOCASE"),
            [name],
            |row| row.get(0),
        )
        .optional()?;
    if taken.is_some() {
        return Err(Error::Conflict(format!("\"{name}\" already exists")));
    }

    let id = ensure(conn, kind, name, CreatedBy::User, now)?;
    if kind == VocabularyKind::Family && !description.is_empty() {
        conn.execute(
            "UPDATE aesthetic_families SET description = ?2, updated_at = ?3 WHERE id = ?1",
            rusqlite::params![id, description, now],
        )?;
    }
    Ok(id)
}

/// Rename an entry, or edit a family's description.
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown id, [`Error::Conflict`] if the new name is
/// taken, or a storage failure.
pub fn update(
    conn: &Connection,
    kind: VocabularyKind,
    id: &str,
    name: Option<&str>,
    description: Option<&str>,
    now: &str,
) -> Result<()> {
    let (table, _, _) = shape(kind);
    let exists: Option<String> = conn
        .query_row(
            &format!("SELECT id FROM {table} WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )
        .optional()?;
    if exists.is_none() {
        return Err(Error::NotFound {
            kind: "vocabulary entry",
            id: id.to_owned(),
        });
    }

    if let Some(name) = name.map(str::trim) {
        if name.is_empty() {
            return Err(Error::Invalid("a name is required".to_owned()));
        }
        let clash: Option<String> = conn
            .query_row(
                &format!("SELECT id FROM {table} WHERE name = ?1 COLLATE NOCASE AND id <> ?2"),
                rusqlite::params![name, id],
                |row| row.get(0),
            )
            .optional()?;
        if clash.is_some() {
            return Err(Error::Conflict(format!(
                "\"{name}\" already exists — merge into it instead"
            )));
        }
        conn.execute(
            &format!("UPDATE {table} SET name = ?2 WHERE id = ?1"),
            rusqlite::params![id, name],
        )?;
    }

    if let Some(description) = description
        && kind == VocabularyKind::Family
    {
        conn.execute(
            "UPDATE aesthetic_families SET description = ?2 WHERE id = ?1",
            rusqlite::params![id, description],
        )?;
    }

    if kind == VocabularyKind::Family {
        conn.execute(
            "UPDATE aesthetic_families SET updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, now],
        )?;
    }
    Ok(())
}

/// Delete an entry and its links.
///
/// The link rows go by `ON DELETE CASCADE`. Items keep existing — deleting a tag is a
/// vocabulary edit, never a way to lose a capture.
///
/// # Errors
/// Propagates a storage failure.
pub fn delete(conn: &Connection, kind: VocabularyKind, id: &str) -> Result<()> {
    let (table, _, _) = shape(kind);
    conn.execute(&format!("DELETE FROM {table} WHERE id = ?1"), [id])?;
    Ok(())
}

/// Which items link to an entry. Used to rebuild their sidecars and search rows after a
/// rename, a merge, or a delete.
///
/// # Errors
/// Propagates a storage failure.
pub fn linked_item_ids(conn: &Connection, kind: VocabularyKind, id: &str) -> Result<Vec<String>> {
    let (_, link_table, column) = shape(kind);
    let sql = format!("SELECT item_id FROM {link_table} WHERE {column} = ?1");
    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map([id], |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<String>>>()?)
}

/// `(table, link table, link column)` for a kind.
pub(crate) fn shape(kind: VocabularyKind) -> (&'static str, &'static str, &'static str) {
    match kind {
        VocabularyKind::Family => ("aesthetic_families", "item_families", "family_id"),
        VocabularyKind::DesignType => ("design_types", "item_types", "type_id"),
        VocabularyKind::Tag => ("tags", "item_tags", "tag_id"),
    }
}

fn authorship(raw: &str) -> CreatedBy {
    if raw == "user" {
        CreatedBy::User
    } else {
        CreatedBy::Ai
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    const NOW: &str = "2026-01-01T00:00:00Z";

    #[test]
    fn ensuring_a_name_twice_returns_one_row() {
        let db = Db::open_in_memory().expect("open");
        let first =
            ensure(db.conn(), VocabularyKind::Tag, "saas", CreatedBy::Ai, NOW).expect("first");
        let second =
            ensure(db.conn(), VocabularyKind::Tag, "saas", CreatedBy::Ai, NOW).expect("second");

        assert_eq!(first, second);
    }

    #[test]
    fn family_names_match_case_insensitively_even_though_the_column_does_not() {
        // `aesthetic_families.name` is UNIQUE but case-SENSITIVE. A model returning
        // "Brutalist" today and "brutalist" tomorrow would otherwise split one family in
        // two, with the items divided and neither filter showing the whole set.
        let db = Db::open_in_memory().expect("open");
        let first = ensure(
            db.conn(),
            VocabularyKind::Family,
            "Brutalist",
            CreatedBy::Ai,
            NOW,
        )
        .expect("first");
        let second = ensure(
            db.conn(),
            VocabularyKind::Family,
            "brutalist",
            CreatedBy::Ai,
            NOW,
        )
        .expect("second");

        assert_eq!(first, second);
    }

    #[test]
    fn creating_a_duplicate_name_is_refused_rather_than_ignored() {
        let db = Db::open_in_memory().expect("open");
        create(db.conn(), VocabularyKind::Tag, "saas", "", NOW).expect("create");

        match create(db.conn(), VocabularyKind::Tag, "SAAS", "", NOW) {
            Err(Error::Conflict(message)) => {
                assert!(message.contains("already exists"), "{message}")
            }
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    #[test]
    fn a_family_carries_its_description_and_item_count() {
        let db = Db::open_in_memory().expect("open");
        let id = create(
            db.conn(),
            VocabularyKind::Family,
            "Minimal",
            "Quiet and spare",
            NOW,
        )
        .expect("create");
        db.conn()
            .execute_batch(&format!(
                "INSERT INTO items (id, name, screenshot_path, created_at, updated_at)
                   VALUES ('01A','x','p','{NOW}','{NOW}');
                 INSERT INTO item_families (item_id, family_id, score) VALUES ('01A','{id}',0.9)"
            ))
            .expect("seed");

        let families = list_families(db.conn()).expect("list");

        assert_eq!(families.len(), 1);
        assert_eq!(families[0].description, "Quiet and spare");
        assert_eq!(families[0].item_count, 1);
        assert_eq!(families[0].created_by, CreatedBy::User);
    }

    #[test]
    fn renaming_onto_an_existing_name_points_at_merge() {
        // The user's intent is almost always a merge; silently refusing without saying so
        // leaves them retyping the same name.
        let db = Db::open_in_memory().expect("open");
        create(db.conn(), VocabularyKind::Tag, "saas", "", NOW).expect("a");
        let b = create(db.conn(), VocabularyKind::Tag, "b2b", "", NOW).expect("b");

        match update(db.conn(), VocabularyKind::Tag, &b, Some("saas"), None, NOW) {
            Err(Error::Conflict(message)) => assert!(message.contains("merge"), "{message}"),
            other => panic!("expected a conflict, got {other:?}"),
        }
    }

    #[test]
    fn renaming_to_a_different_case_of_its_own_name_is_allowed() {
        // Fixing capitalisation is a rename, not a collision with itself.
        let db = Db::open_in_memory().expect("open");
        let id = create(db.conn(), VocabularyKind::Tag, "saas", "", NOW).expect("create");

        update(db.conn(), VocabularyKind::Tag, &id, Some("SaaS"), None, NOW).expect("rename");
        assert_eq!(
            list_terms(db.conn(), VocabularyKind::Tag).expect("list")[0].name,
            "SaaS"
        );
    }

    #[test]
    fn updating_an_unknown_id_is_not_found() {
        let db = Db::open_in_memory().expect("open");
        assert!(matches!(
            update(
                db.conn(),
                VocabularyKind::Tag,
                "01NOPE",
                Some("x"),
                None,
                NOW
            ),
            Err(Error::NotFound { .. })
        ));
    }

    #[test]
    fn deleting_a_tag_keeps_the_items_that_carried_it() {
        // A vocabulary edit must never be a way to lose a capture.
        let db = Db::open_in_memory().expect("open");
        let id = create(db.conn(), VocabularyKind::Tag, "saas", "", NOW).expect("create");
        db.conn()
            .execute_batch(&format!(
                "INSERT INTO items (id, name, screenshot_path, created_at, updated_at)
                   VALUES ('01A','x','p','{NOW}','{NOW}');
                 INSERT INTO item_tags (item_id, tag_id) VALUES ('01A','{id}')"
            ))
            .expect("seed");

        delete(db.conn(), VocabularyKind::Tag, &id).expect("delete");

        let items: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            .expect("count");
        let links: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM item_tags", [], |row| row.get(0))
            .expect("count");

        assert_eq!(items, 1);
        assert_eq!(links, 0, "the link goes with the tag");
    }

    #[test]
    fn linked_items_are_reported_so_their_sidecars_can_be_rebuilt() {
        let db = Db::open_in_memory().expect("open");
        let id = create(db.conn(), VocabularyKind::Tag, "saas", "", NOW).expect("create");
        db.conn()
            .execute_batch(&format!(
                "INSERT INTO items (id, name, screenshot_path, created_at, updated_at)
                   VALUES ('01A','x','p','{NOW}','{NOW}'), ('01B','y','p','{NOW}','{NOW}');
                 INSERT INTO item_tags (item_id, tag_id) VALUES ('01A','{id}'), ('01B','{id}')"
            ))
            .expect("seed");

        let mut linked = linked_item_ids(db.conn(), VocabularyKind::Tag, &id).expect("linked");
        linked.sort();

        assert_eq!(linked, ["01A", "01B"]);
    }
}
