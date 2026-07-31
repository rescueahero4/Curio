//! Rewriting an item's links and its projections.
//!
//! Everything here runs **inside** the transaction that changed the item. That is the whole
//! of R-DA-4: there is no code path that commits a row and leaves the search index or the
//! sidecar beside it describing something else.
//!
//! [`finish`] is the single exit from every mutation in [`super::write`], [`super::curate`],
//! and [`super::bulk`], so the projections cannot be forgotten by a future path that only
//! remembers to write the row.

use rusqlite::Connection;

use curio_core::domain::{CreatedBy, Item, VocabularyKind};

use crate::{Result, sidecars, vocabulary};

use super::require;

/// Replace an item's whole tag or design-type set, creating names as needed.
pub(crate) fn set_terms(
    tx: &Connection,
    item_id: &str,
    kind: VocabularyKind,
    names: &[String],
    now: &str,
) -> Result<()> {
    let (_, link_table, column) = vocabulary::shape(kind);
    tx.execute(
        &format!("DELETE FROM {link_table} WHERE item_id = ?1"),
        [item_id],
    )?;

    for name in curio_core::assessment::clean_vocabulary_names(names) {
        let id = vocabulary::ensure(tx, kind, &name, CreatedBy::User, now)?;
        tx.execute(
            &format!("INSERT OR IGNORE INTO {link_table} (item_id, {column}) VALUES (?1, ?2)"),
            rusqlite::params![item_id, id],
        )?;
    }
    Ok(())
}

/// Replace an item's whole family set (Inventory §6).
///
/// Retained links keep their score; new ones are scored 1.0 because a person picked them,
/// and a person is not 87 % sure. Every surviving link is settled: the user just answered
/// the question the gray zone was asking.
pub(crate) fn set_families(tx: &Connection, item_id: &str, family_ids: &[String]) -> Result<()> {
    if family_ids.is_empty() {
        // `NOT IN ()` is a syntax error, and this is the ordinary path for a user clearing
        // every family off an item.
        tx.execute("DELETE FROM item_families WHERE item_id = ?1", [item_id])?;
        return Ok(());
    }

    let mut params: Vec<&dyn rusqlite::ToSql> = Vec::with_capacity(family_ids.len() + 1);
    params.push(&item_id);
    for id in family_ids {
        params.push(id);
    }
    let kept = super::placeholders(family_ids.len() + 1);
    let kept = kept.split_once(", ").map_or("", |(_, rest)| rest);
    tx.execute(
        &format!("DELETE FROM item_families WHERE item_id = ?1 AND family_id NOT IN ({kept})"),
        params.as_slice(),
    )?;

    for family_id in family_ids {
        tx.execute(
            "INSERT INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
               VALUES (?1, ?2, ?3, 0, 0)
             ON CONFLICT (item_id, family_id) DO UPDATE SET gray_zone = 0",
            rusqlite::params![
                item_id,
                family_id,
                curio_core::assessment::HUMAN_PICKED_SCORE
            ],
        )?;
    }
    Ok(())
}

/// Re-read the item, rewrite its search row, regenerate its sidecar.
///
/// The single exit from every mutation above, so the projections cannot be forgotten by a
/// future path that only remembers to write the row.
pub(crate) fn finish(conn: &Connection, root: Option<&std::path::Path>, id: &str) -> Result<Item> {
    let item = require(conn, id)?;
    sync_fts(conn, &item)?;
    sidecars::write_item(root, &item)?;
    Ok(item)
}

/// Rewrite one item's row in the search index (R-DA-10).
pub(crate) fn sync_fts(conn: &Connection, item: &Item) -> Result<()> {
    conn.execute("DELETE FROM items_fts WHERE item_id = ?1", [&item.id])?;
    conn.execute(
        "INSERT INTO items_fts (item_id, name, short_description, tags_concat)
           VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            item.id,
            item.name,
            item.short_description,
            item.searchable_tags()
        ],
    )?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;
    use crate::items;
    use crate::items::write::{ItemPatch, NewItem, create, patch};
    use curio_core::domain::{ItemStatus, LastEditedBy};

    fn library() -> (Db, tempfile::TempDir) {
        let dir = tempfile::tempdir().expect("tempdir");
        let db = Db::open(&dir.path().join("library.db")).expect("open");
        (db, dir)
    }

    fn new_item() -> NewItem {
        NewItem {
            name: "Stripe pricing".to_owned(),
            source_url: Some("https://stripe.com/pricing".to_owned()),
            screenshot_path: "items/x/screenshot.png".to_owned(),
            thumbnail_path: None,
        }
    }

    #[test]
    fn setting_the_family_set_promotes_an_item_out_of_review() {
        // Inventory §10.13: the whole-set edit IS the decision. Leaving the item in the
        // review queue would ask the user a question they just answered.
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");
        let family = vocabulary::create(
            db.conn(),
            VocabularyKind::Family,
            "Minimal",
            "Quiet",
            &curio_core::time::now_iso(),
        )
        .expect("family");
        db.conn()
            .execute(
                "UPDATE items SET status = 'needs_review' WHERE id = ?1",
                [&created.id],
            )
            .expect("hold");

        let resolved = patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                family_ids: Some(vec![family]),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");

        assert_eq!(resolved.status, ItemStatus::Ready);
        assert_eq!(resolved.families.len(), 1);
        assert!(!resolved.families[0].gray_zone);
    }

    #[test]
    fn a_family_the_user_keeps_holds_on_to_its_score() {
        // Retained links keep score, new links are 1.0. Re-scoring a family the user
        // merely left alone would destroy the model's measurement for no reason.
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");
        let family = vocabulary::create(
            db.conn(),
            VocabularyKind::Family,
            "Minimal",
            "Quiet",
            &curio_core::time::now_iso(),
        )
        .expect("family");
        db.conn()
            .execute(
                "INSERT INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
                   VALUES (?1, ?2, 0.62, 1, 0)",
                rusqlite::params![created.id, family],
            )
            .expect("link");

        let patched = patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                family_ids: Some(vec![family]),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");

        assert!((patched.families[0].score - 0.62).abs() < f64::EPSILON);
        assert!(
            !patched.families[0].gray_zone,
            "the decision settles the link"
        );
    }

    #[test]
    fn tags_are_replaced_whole_and_created_on_demand() {
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");

        let first = patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                tags: Some(vec!["saas".to_owned(), "minimal".to_owned()]),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");
        assert_eq!(first.tags, ["minimal", "saas"]);

        let second = patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                tags: Some(vec!["dark".to_owned()]),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");
        assert_eq!(second.tags, ["dark"], "the set is replaced, not merged");
    }

    #[test]
    fn editing_an_item_keeps_it_findable_under_its_new_name() {
        // The FTS row is rewritten in the same transaction (R-DA-10). Without it, search
        // answers with the old name until something else touches the item.
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");
        patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                name: Some("Vercel dashboard".to_owned()),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");

        let found = items::list(
            db.conn(),
            &curio_core::query::ItemQuery {
                search: Some("vercel".to_owned()),
                ..curio_core::query::ItemQuery::unfiltered()
            },
        )
        .expect("search");

        assert_eq!(found.items.len(), 1);
    }
}
