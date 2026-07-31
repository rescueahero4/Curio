//! Merging two vocabulary entries, and pruning what nothing points at.
//!
//! Merge is the one vocabulary operation that can lose information, so the rules are
//! precise and inherited verbatim (R-DA-11, Inventory §10.14):
//!
//! * **Families keep `MAX(score)` and `MIN(gray_zone)`.** An item linked to both the
//!   source and the target keeps the better score, because the two names described one
//!   look and the higher number is the better measurement of it. It keeps the *settled*
//!   flag, because a decision already made must not be re-asked — merging two families is
//!   not new evidence about an item the user already adjudicated.
//! * **Tags and types are `INSERT OR IGNORE` then delete.** They carry no per-link data, so
//!   there is nothing to reconcile.
//! * **Orphans are pruned after the fact.** A merge, a bulk replace, or a delete can leave
//!   a name with no items behind it, and a filter list full of names that match nothing is
//!   worse than a short one.

use rusqlite::Connection;

use curio_core::domain::VocabularyKind;

use crate::{Error, Result};

use super::shape;

/// Fold `source` into `target`, returning the items whose links changed.
///
/// The returned ids are what the caller must rebuild sidecars and search rows for — the
/// merge changes what those items are *called*, and a sidecar naming a family that no
/// longer exists is exactly the disagreement R-DA-4 forbids.
///
/// # Errors
/// Returns [`Error::Invalid`] if the two ids are the same, [`Error::NotFound`] if either
/// is unknown, or a storage failure.
pub fn merge(
    conn: &Connection,
    kind: VocabularyKind,
    source: &str,
    target: &str,
) -> Result<Vec<String>> {
    if source == target {
        // Not a no-op worth allowing: the delete at the end would remove the only row and
        // strip the links from every item that carried it.
        return Err(Error::Invalid(
            "an entry cannot be merged into itself".to_owned(),
        ));
    }
    let (table, link_table, column) = shape(kind);

    for id in [source, target] {
        let exists: i64 = conn.query_row(
            &format!("SELECT COUNT(*) FROM {table} WHERE id = ?1"),
            [id],
            |row| row.get(0),
        )?;
        if exists == 0 {
            return Err(Error::NotFound {
                kind: "vocabulary entry",
                id: id.to_owned(),
            });
        }
    }

    let touched = super::linked_item_ids(conn, kind, source)?;

    if kind == VocabularyKind::Family {
        // Items already linked to both: fold the source's numbers into the target's row
        // before moving anything, or the move below would collide on the primary key.
        conn.execute(
            "UPDATE item_families AS t
                SET score = MAX(t.score, (SELECT s.score FROM item_families s
                                           WHERE s.item_id = t.item_id AND s.family_id = ?1)),
                    gray_zone = MIN(t.gray_zone, (SELECT s.gray_zone FROM item_families s
                                                   WHERE s.item_id = t.item_id AND s.family_id = ?1)),
                    ai_proposed = MIN(t.ai_proposed, (SELECT s.ai_proposed FROM item_families s
                                                       WHERE s.item_id = t.item_id AND s.family_id = ?1))
              WHERE t.family_id = ?2
                AND EXISTS (SELECT 1 FROM item_families s
                             WHERE s.item_id = t.item_id AND s.family_id = ?1)",
            rusqlite::params![source, target],
        )?;
    }

    // Items linked only to the source. `OR IGNORE` covers the rows the fold above already
    // reconciled — they are about to be deleted with the source row anyway.
    conn.execute(
        &format!("UPDATE OR IGNORE {link_table} SET {column} = ?2 WHERE {column} = ?1"),
        rusqlite::params![source, target],
    )?;
    conn.execute(&format!("DELETE FROM {table} WHERE id = ?1"), [source])?;

    Ok(touched)
}

/// Delete vocabulary entries no item links to, returning how many went.
///
/// Run after deletes and after a replace-mode bulk retag (Inventory §9). Families created
/// by the **user** are kept: an empty family is a deliberate act — a name coined before the
/// captures that will fill it — whereas an empty AI-proposed one is residue.
///
/// # Errors
/// Propagates a storage failure.
pub fn prune_orphans(conn: &Connection) -> Result<usize> {
    let mut removed = 0usize;
    for kind in VocabularyKind::all() {
        let (table, link_table, column) = shape(kind);
        removed += conn.execute(
            &format!(
                "DELETE FROM {table}
                  WHERE created_by = 'ai'
                    AND NOT EXISTS (SELECT 1 FROM {link_table} l WHERE l.{column} = {table}.id)"
            ),
            [],
        )?;
    }
    Ok(removed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    const NOW: &str = "2026-01-01T00:00:00Z";

    fn seeded() -> Db {
        let db = Db::open_in_memory().expect("open");
        db.conn()
            .execute_batch(
                "INSERT INTO items (id, name, screenshot_path, created_at, updated_at) VALUES
                   ('01A','a','p','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
                   ('01B','b','p','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
                   ('01C','c','p','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
                 INSERT INTO aesthetic_families (id, name, description, created_by, created_at, updated_at) VALUES
                   ('src','Brutal','x','ai','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
                   ('dst','Brutalist','y','ai','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
                 INSERT INTO tags (id, name, created_by) VALUES ('ts','saas','ai'), ('td','b2b-saas','ai');",
            )
            .expect("seed");
        db
    }

    fn link(db: &Db, item: &str, family: &str, score: f64, gray: i64) {
        db.conn()
            .execute(
                "INSERT INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
                   VALUES (?1, ?2, ?3, ?4, 0)",
                rusqlite::params![item, family, score, gray],
            )
            .expect("link");
    }

    fn read(db: &Db, item: &str) -> (f64, i64) {
        db.conn()
            .query_row(
                "SELECT score, gray_zone FROM item_families WHERE item_id = ?1 AND family_id = 'dst'",
                [item],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .expect("read")
    }

    #[test]
    fn an_item_linked_to_both_keeps_the_better_score() {
        // Two names described one look; the higher number is the better measurement of it.
        let db = seeded();
        link(&db, "01A", "src", 0.9, 0);
        link(&db, "01A", "dst", 0.6, 0);

        merge(db.conn(), VocabularyKind::Family, "src", "dst").expect("merge");

        assert!((read(&db, "01A").0 - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn a_decision_already_made_survives_the_merge() {
        // MIN(gray_zone). Merging two families is not new evidence about an item the user
        // already adjudicated, so it must not put the badge back.
        let db = seeded();
        link(&db, "01A", "src", 0.4, 1);
        link(&db, "01A", "dst", 0.8, 0);

        merge(db.conn(), VocabularyKind::Family, "src", "dst").expect("merge");

        assert_eq!(read(&db, "01A").1, 0, "a settled link must stay settled");
    }

    #[test]
    fn an_item_linked_only_to_the_source_moves_across() {
        let db = seeded();
        link(&db, "01B", "src", 0.7, 1);

        merge(db.conn(), VocabularyKind::Family, "src", "dst").expect("merge");

        let (score, gray) = read(&db, "01B");
        assert!((score - 0.7).abs() < f64::EPSILON);
        assert_eq!(gray, 1, "an unresolved link stays unresolved");
    }

    #[test]
    fn the_source_family_is_gone_afterwards() {
        let db = seeded();
        link(&db, "01A", "src", 0.9, 0);

        merge(db.conn(), VocabularyKind::Family, "src", "dst").expect("merge");

        let remaining: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM aesthetic_families WHERE id = 'src'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(remaining, 0);
    }

    #[test]
    fn merging_reports_the_items_whose_sidecars_must_be_rebuilt() {
        // R-DA-4: a sidecar naming a family that no longer exists is exactly the
        // disagreement the projection rule forbids.
        let db = seeded();
        link(&db, "01A", "src", 0.9, 0);
        link(&db, "01B", "src", 0.5, 0);

        let mut touched = merge(db.conn(), VocabularyKind::Family, "src", "dst").expect("merge");
        touched.sort();

        assert_eq!(touched, ["01A", "01B"]);
    }

    #[test]
    fn tags_merge_without_duplicating_a_link() {
        let db = seeded();
        db.conn()
            .execute_batch(
                "INSERT INTO item_tags (item_id, tag_id) VALUES ('01A','ts'), ('01A','td'), ('01B','ts')",
            )
            .expect("seed");

        merge(db.conn(), VocabularyKind::Tag, "ts", "td").expect("merge");

        let links: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM item_tags WHERE tag_id = 'td'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(links, 2, "01A once, 01B once");
    }

    #[test]
    fn merging_an_entry_into_itself_is_refused() {
        // The delete at the end would otherwise remove the only row and strip the links
        // from every item that carried it.
        let db = seeded();
        assert!(matches!(
            merge(db.conn(), VocabularyKind::Family, "src", "src"),
            Err(Error::Invalid(_))
        ));
    }

    #[test]
    fn merging_an_unknown_entry_is_not_found() {
        let db = seeded();
        assert!(matches!(
            merge(db.conn(), VocabularyKind::Family, "src", "01NOPE"),
            Err(Error::NotFound { .. })
        ));
    }

    #[test]
    fn pruning_removes_ai_names_nothing_points_at() {
        let db = seeded();
        assert_eq!(
            prune_orphans(db.conn()).expect("prune"),
            4,
            "two families, two tags"
        );
    }

    #[test]
    fn pruning_keeps_a_name_the_user_coined() {
        // An empty user family is a deliberate act — a name coined before the captures
        // that will fill it. An empty AI-proposed one is residue.
        let db = seeded();
        db.conn()
            .execute(
                "INSERT INTO aesthetic_families (id, name, description, created_by, created_at, updated_at)
                   VALUES ('mine','Mine','x','user',?1,?1)",
                [NOW],
            )
            .expect("seed");

        prune_orphans(db.conn()).expect("prune");

        let kept: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM aesthetic_families WHERE id = 'mine'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(kept, 1);
    }

    #[test]
    fn pruning_keeps_a_name_that_is_still_in_use() {
        let db = seeded();
        link(&db, "01A", "dst", 0.9, 0);

        prune_orphans(db.conn()).expect("prune");

        let kept: i64 = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM aesthetic_families WHERE id = 'dst'",
                [],
                |row| row.get(0),
            )
            .expect("count");
        assert_eq!(kept, 1);
    }
}
