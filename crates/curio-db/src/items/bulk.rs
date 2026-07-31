//! Bulk vocabulary edits over a frozen set of items (FR-11).
//!
//! Additive and subtractive, never a whole-set replace: the caller selected items by
//! filter, and replacing every item's tags with one list would destroy everything the
//! selection had in common except the thing being edited.
//!
//! The invariant that is easiest to get backwards: **a bulk add preserves gray zones**
//! (Inventory §10.13). Adding a tag to forty items says nothing about the family question
//! hanging over three of them, so it must not silently answer it.

use rusqlite::Connection;

use curio_core::assessment::HUMAN_PICKED_SCORE;
use curio_core::domain::{CreatedBy, Item, VocabularyKind};

use crate::{Result, vocabulary};

use super::links::finish;

/// What a bulk edit does to a frozen set of items (Inventory §6).
#[derive(Debug, Clone, Default)]
pub struct BulkEdit {
    pub add_tags: Vec<String>,
    pub remove_tags: Vec<String>,
    pub add_types: Vec<String>,
    pub remove_types: Vec<String>,
    /// Family ids — a family must exist before it can be assigned in bulk.
    pub add_families: Vec<String>,
    pub remove_families: Vec<String>,
}

impl BulkEdit {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.add_tags.is_empty()
            && self.remove_tags.is_empty()
            && self.add_types.is_empty()
            && self.remove_types.is_empty()
            && self.add_families.is_empty()
            && self.remove_families.is_empty()
    }
}

/// Apply a bulk edit, returning the items that changed.
///
/// Additive and subtractive, never a whole-set replace: the caller selected items by
/// filter, and replacing every item's tags with one list would destroy everything the
/// selection had in common except the thing being edited.
///
/// # Errors
/// Propagates a storage failure.
pub fn bulk_edit(
    conn: &mut Connection,
    root: Option<&std::path::Path>,
    ids: &[String],
    edit: &BulkEdit,
) -> Result<Vec<Item>> {
    let now = curio_core::time::now_iso();
    let tx = conn.transaction()?;
    let mut changed = Vec::with_capacity(ids.len());

    for id in ids {
        if super::get(&tx, id)?.is_none() {
            // A stale selection: something was deleted between the user picking it and
            // pressing the button. Skipping is right — refusing the whole batch would
            // punish thirty-nine innocent items for one.
            continue;
        }

        for name in curio_core::assessment::clean_vocabulary_names(&edit.add_tags) {
            add_term(&tx, id, VocabularyKind::Tag, &name, &now)?;
        }
        for name in curio_core::assessment::clean_vocabulary_names(&edit.add_types) {
            add_term(&tx, id, VocabularyKind::DesignType, &name, &now)?;
        }
        remove_terms(&tx, id, VocabularyKind::Tag, &edit.remove_tags)?;
        remove_terms(&tx, id, VocabularyKind::DesignType, &edit.remove_types)?;

        for family_id in &edit.add_families {
            // `OR IGNORE`, not `OR REPLACE`: an item already in this family keeps its
            // score and, crucially, its gray-zone flag. Adding a family to forty items
            // says nothing about the question hanging over three of them.
            tx.execute(
                "INSERT OR IGNORE INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
                   VALUES (?1, ?2, ?3, 0, 0)",
                rusqlite::params![id, family_id, HUMAN_PICKED_SCORE],
            )?;
        }
        for family_id in &edit.remove_families {
            tx.execute(
                "DELETE FROM item_families WHERE item_id = ?1 AND family_id = ?2",
                rusqlite::params![id, family_id],
            )?;
        }

        tx.execute(
            "UPDATE items SET updated_at = ?2 WHERE id = ?1",
            rusqlite::params![id, now],
        )?;
        changed.push(finish(&tx, root, id)?);
    }

    tx.commit()?;
    Ok(changed)
}

fn add_term(
    tx: &Connection,
    item_id: &str,
    kind: VocabularyKind,
    name: &str,
    now: &str,
) -> Result<()> {
    let (_, link_table, column) = vocabulary::shape(kind);
    let id = vocabulary::ensure(tx, kind, name, CreatedBy::User, now)?;
    tx.execute(
        &format!("INSERT OR IGNORE INTO {link_table} (item_id, {column}) VALUES (?1, ?2)"),
        rusqlite::params![item_id, id],
    )?;
    Ok(())
}

fn remove_terms(
    tx: &Connection,
    item_id: &str,
    kind: VocabularyKind,
    names: &[String],
) -> Result<()> {
    let (table, link_table, column) = vocabulary::shape(kind);
    for name in names {
        tx.execute(
            &format!(
                "DELETE FROM {link_table} WHERE item_id = ?1 AND {column} IN
                   (SELECT id FROM {table} WHERE name = ?2 COLLATE NOCASE)"
            ),
            rusqlite::params![item_id, name],
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;
    use crate::items::curate::apply_assessment;
    use crate::items::write::{NewItem, create};
    use curio_core::assessment::{AssessmentOutput, FamilyScore};
    use curio_core::config::Thresholds;
    use curio_core::domain::{ItemStatus, LastEditedBy};

    fn library() -> Db {
        Db::open_in_memory().expect("open")
    }

    fn seeded(db: &mut Db) -> String {
        create(
            db.conn_mut(),
            None,
            &NewItem {
                name: "Capture".to_owned(),
                source_url: None,
                screenshot_path: "items/x/screenshot.png".to_owned(),
                thumbnail_path: None,
            },
        )
        .expect("create")
        .id
    }

    fn family(db: &Db, name: &str) -> String {
        vocabulary::create(
            db.conn(),
            VocabularyKind::Family,
            name,
            "desc",
            &curio_core::time::now_iso(),
        )
        .expect("family")
    }

    fn output(scores: Vec<FamilyScore>) -> AssessmentOutput {
        AssessmentOutput {
            name_suggestion: "Model's name".to_owned(),
            short_description: "As described".to_owned(),
            design_types: vec!["pricing page".to_owned()],
            tags: vec!["saas".to_owned()],
            family_scores: scores,
            new_family_proposal: None,
            image_recipe: None,
        }
    }

    #[test]
    fn a_bulk_add_preserves_a_gray_zone_it_knows_nothing_about() {
        // Inventory §10.13. Adding a tag to forty items says nothing about the family
        // question hanging over three of them.
        let mut db = library();
        let id = seeded(&mut db);
        family(&db, "Minimal");
        apply_assessment(
            db.conn_mut(),
            None,
            &id,
            &output(vec![FamilyScore {
                family: "Minimal".to_owned(),
                score: 0.45,
            }]),
            Thresholds::default(),
        )
        .expect("assess");

        let changed = bulk_edit(
            db.conn_mut(),
            None,
            &[id],
            &BulkEdit {
                add_tags: vec!["reviewed".to_owned()],
                ..BulkEdit::default()
            },
        )
        .expect("bulk");

        assert!(changed[0].tags.contains(&"reviewed".to_owned()));
        assert!(
            changed[0].families[0].gray_zone,
            "the question is still open"
        );
        assert_eq!(changed[0].status, ItemStatus::NeedsReview);
    }

    #[test]
    fn a_bulk_edit_does_not_claim_a_human_edited_the_fields() {
        let mut db = library();
        let id = seeded(&mut db);

        let changed = bulk_edit(
            db.conn_mut(),
            None,
            &[id],
            &BulkEdit {
                add_tags: vec!["batch".to_owned()],
                ..BulkEdit::default()
            },
        )
        .expect("bulk");

        assert_eq!(changed[0].last_edited_by, LastEditedBy::Ai);
    }

    #[test]
    fn a_bulk_edit_skips_an_item_that_vanished_from_under_the_selection() {
        // Refusing the whole batch would punish thirty-nine innocent items for one.
        let mut db = library();
        let id = seeded(&mut db);

        let changed = bulk_edit(
            db.conn_mut(),
            None,
            &[id.clone(), "01GONE".to_owned()],
            &BulkEdit {
                add_tags: vec!["batch".to_owned()],
                ..BulkEdit::default()
            },
        )
        .expect("bulk");

        assert_eq!(changed.len(), 1);
        assert_eq!(changed[0].id, id);
    }

    #[test]
    fn removing_a_tag_is_case_insensitive() {
        // `tags.name` is COLLATE NOCASE, so the filter chip the user clicked and the stored
        // row can differ in case. A case-sensitive delete would silently do nothing.
        let mut db = library();
        let id = seeded(&mut db);
        bulk_edit(
            db.conn_mut(),
            None,
            std::slice::from_ref(&id),
            &BulkEdit {
                add_tags: vec!["SaaS".to_owned()],
                ..BulkEdit::default()
            },
        )
        .expect("add");

        let changed = bulk_edit(
            db.conn_mut(),
            None,
            &[id],
            &BulkEdit {
                remove_tags: vec!["saas".to_owned()],
                ..BulkEdit::default()
            },
        )
        .expect("remove");

        assert!(changed[0].tags.is_empty());
    }

    #[test]
    fn an_empty_bulk_edit_is_recognisable_as_one() {
        assert!(BulkEdit::default().is_empty());
    }
}
