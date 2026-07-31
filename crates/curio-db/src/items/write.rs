//! Changing items.
//!
//! Every function here goes through [`finish`], which re-reads the item, rewrites its
//! search row, and regenerates its sidecar — inside the same transaction as the change
//! (R-DA-4, FR-5). That is what makes "the database is the source of truth" checkable
//! rather than aspirational: there is no code path that commits a row and leaves the file
//! beside it describing something else.
//!
//! ## Authorship is load-bearing
//!
//! `last_edited_by` decides whether a re-assessment may overwrite a name (Inventory
//! §10.12). The stamping rules differ per path and are the caller's to choose, which is why
//! [`patch`] takes the actor rather than assuming one: a dashboard edit stamps `User`, an
//! MCP tool stamps `Ai`, and a bulk operation touches only `updated_at`.

use rusqlite::Connection;

use curio_core::domain::{Item, ItemStatus, LastEditedBy, VocabularyKind};

use crate::{Result, sidecars};

use super::links::{finish, set_families, set_terms};

use super::require;

/// A new item, as ingestion hands it over.
#[derive(Debug, Clone)]
pub struct NewItem {
    pub name: String,
    pub source_url: Option<String>,
    /// Relative to the data root (R-DA-1).
    pub screenshot_path: String,
    pub thumbnail_path: Option<String>,
}

/// What a PATCH may change. `None` means "leave alone"; `Some(None)` means "clear".
#[derive(Debug, Clone, Default)]
pub struct ItemPatch {
    pub name: Option<String>,
    pub short_description: Option<String>,
    pub source_url: Option<Option<String>>,
    pub image_recipe: Option<Option<String>>,
    /// Whole set, by **name** — tags and types are created on demand.
    pub tags: Option<Vec<String>>,
    pub design_types: Option<Vec<String>>,
    /// Whole set, by **id** — a family must already exist to be assigned.
    pub family_ids: Option<Vec<String>>,
    pub status: Option<ItemStatus>,
}

impl ItemPatch {
    /// Whether this patch would change nothing.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.name.is_none()
            && self.short_description.is_none()
            && self.source_url.is_none()
            && self.image_recipe.is_none()
            && self.tags.is_none()
            && self.design_types.is_none()
            && self.family_ids.is_none()
            && self.status.is_none()
    }
}

/// Insert an item and project it. It starts `processing` and is visible immediately (FR-3).
///
/// # Errors
/// Propagates a storage failure.
pub fn create(
    conn: &mut Connection,
    root: Option<&std::path::Path>,
    new: &NewItem,
) -> Result<Item> {
    let id = curio_core::ids::generate();
    let now = curio_core::time::now_iso();
    let tx = conn.transaction()?;

    tx.execute(
        "INSERT INTO items (id, name, short_description, source_url, screenshot_path,
                            thumbnail_path, status, last_edited_by, created_at, updated_at)
           VALUES (?1, ?2, '', ?3, ?4, ?5, 'processing', 'ai', ?6, ?6)",
        rusqlite::params![
            id,
            new.name,
            new.source_url,
            new.screenshot_path,
            new.thumbnail_path,
            now
        ],
    )?;

    let item = finish(&tx, root, &id)?;
    tx.commit()?;
    Ok(item)
}

/// Apply a patch, stamping authorship as `actor`.
///
/// # Errors
/// Returns [`crate::Error::NotFound`] for an unknown item, or a storage failure.
pub fn patch(
    conn: &mut Connection,
    root: Option<&std::path::Path>,
    id: &str,
    patch: &ItemPatch,
    actor: LastEditedBy,
) -> Result<Item> {
    let now = curio_core::time::now_iso();
    let tx = conn.transaction()?;
    let before = require(&tx, id)?;

    if let Some(name) = &patch.name {
        tx.execute(
            "UPDATE items SET name = ?2 WHERE id = ?1",
            rusqlite::params![id, name],
        )?;
    }
    if let Some(description) = &patch.short_description {
        tx.execute(
            "UPDATE items SET short_description = ?2 WHERE id = ?1",
            rusqlite::params![id, description],
        )?;
    }
    if let Some(url) = &patch.source_url {
        tx.execute(
            "UPDATE items SET source_url = ?2 WHERE id = ?1",
            rusqlite::params![id, url],
        )?;
    }
    if let Some(recipe) = &patch.image_recipe {
        tx.execute(
            "UPDATE items SET image_recipe = ?2 WHERE id = ?1",
            rusqlite::params![id, recipe],
        )?;
    }

    if let Some(tags) = &patch.tags {
        set_terms(&tx, id, VocabularyKind::Tag, tags, &now)?;
    }
    if let Some(types) = &patch.design_types {
        set_terms(&tx, id, VocabularyKind::DesignType, types, &now)?;
    }

    let mut status = patch.status;
    if let Some(family_ids) = &patch.family_ids {
        set_families(&tx, id, family_ids)?;
        // Inventory §10.13: editing the family set whole is the decision. An item held at
        // `needs_review` purely because of a gray-zone link has just had that link
        // replaced by a deliberate choice, so leaving it in the review queue would ask the
        // user to answer a question they have already answered.
        if status.is_none() && before.status == ItemStatus::NeedsReview {
            status = Some(ItemStatus::Ready);
        }
    }
    if let Some(status) = status {
        tx.execute(
            "UPDATE items SET status = ?2, error = NULL WHERE id = ?1",
            rusqlite::params![id, status.as_str()],
        )?;
    }

    tx.execute(
        "UPDATE items SET last_edited_by = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, actor.as_str(), now],
    )?;

    let item = finish(&tx, root, id)?;
    tx.commit()?;
    Ok(item)
}

/// Record where an item's media landed.
///
/// Separate from [`create`] because the id is minted by the insert and the file is named
/// after it: ingestion inserts, learns the id, writes `items/{id}/screenshot.png`, and comes
/// back here. Writing the file first would mean choosing a name before there is a row to
/// own it, and an upload that failed in between would leave an orphaned directory.
///
/// # Errors
/// Propagates a storage failure.
pub fn set_media(
    conn: &Connection,
    root: Option<&std::path::Path>,
    id: &str,
    screenshot_path: &str,
    thumbnail_path: Option<&str>,
) -> Result<Item> {
    conn.execute(
        "UPDATE items SET screenshot_path = ?2, thumbnail_path = ?3, updated_at = ?4
           WHERE id = ?1",
        rusqlite::params![
            id,
            screenshot_path,
            thumbnail_path,
            curio_core::time::now_iso()
        ],
    )?;
    finish(conn, root, id)
}

/// Move an item's status without touching authorship.
///
/// The assessment pipeline's path: a failure must **preserve** `last_edited_by`
/// (Inventory §10.12), or a failed re-assess would silently claim the AI had edited an
/// item the user last touched.
///
/// # Errors
/// Propagates a storage failure.
pub fn set_status(
    conn: &mut Connection,
    root: Option<&std::path::Path>,
    id: &str,
    status: ItemStatus,
    error: Option<&str>,
) -> Result<Item> {
    let tx = conn.transaction()?;
    tx.execute(
        "UPDATE items SET status = ?2, error = ?3, updated_at = ?4 WHERE id = ?1",
        rusqlite::params![id, status.as_str(), error, curio_core::time::now_iso()],
    )?;
    let item = finish(&tx, root, id)?;
    tx.commit()?;
    Ok(item)
}

/// Bump `updated_at` and rebuild the projections, changing nothing else.
///
/// What a bulk operation does (Inventory §10.12): it touched the item, so the timestamp
/// moves, but it was not a person editing a field and must not claim to be.
///
/// # Errors
/// Propagates a storage failure.
pub fn touch(conn: &Connection, root: Option<&std::path::Path>, id: &str) -> Result<Item> {
    conn.execute(
        "UPDATE items SET updated_at = ?2 WHERE id = ?1",
        rusqlite::params![id, curio_core::time::now_iso()],
    )?;
    finish(conn, root, id)
}

/// Delete an item, its links, its search row, and its directory.
///
/// # Errors
/// Propagates a storage failure.
pub fn delete(conn: &mut Connection, root: Option<&std::path::Path>, id: &str) -> Result<()> {
    let tx = conn.transaction()?;
    // Link rows go by ON DELETE CASCADE; the search row is not a foreign key, so it does
    // not — and a stale FTS row makes a deleted item findable and then un-openable.
    tx.execute("DELETE FROM items_fts WHERE item_id = ?1", [id])?;
    tx.execute("DELETE FROM items WHERE id = ?1", [id])?;
    tx.commit()?;

    // After the commit: the row is already gone, so a locked file must not be able to
    // refuse a delete the user has been told succeeded.
    sidecars::remove_item(root, id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Db, items};

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
    fn a_new_item_is_processing_and_visible_immediately() {
        // FR-3: a capture is a card the instant it lands, not once the model replies.
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");

        assert_eq!(created.status, ItemStatus::Processing);
        assert_eq!(items::count(db.conn()).expect("count"), 1);
    }

    #[test]
    fn creating_an_item_writes_its_sidecar() {
        // R-DA-4 / FR-5: the projection exists from the first write, not from the first
        // assessment. An agent walking the library must see the capture.
        let (mut db, dir) = library();
        let root = dir.path().to_path_buf();
        db.set_data_root(&root);
        let created = create(db.conn_mut(), Some(&root), &new_item()).expect("create");

        let sidecar = root.join("items").join(&created.id).join("item.md");
        assert!(sidecar.exists(), "{}", sidecar.display());
    }

    #[test]
    fn a_dashboard_edit_stamps_the_user() {
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");

        let edited = patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                name: Some("My name for it".to_owned()),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");

        assert_eq!(edited.name, "My name for it");
        assert_eq!(edited.last_edited_by, LastEditedBy::User);
    }

    #[test]
    fn a_bulk_touch_does_not_claim_to_be_an_edit() {
        // Inventory §10.12. If a bulk operation stamped `user`, a later re-assess would
        // refuse to improve names nobody actually chose.
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");

        let touched = touch(db.conn(), None, &created.id).expect("touch");

        assert_eq!(touched.last_edited_by, LastEditedBy::Ai);
    }

    #[test]
    fn a_failed_assessment_preserves_authorship() {
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");
        patch(
            db.conn_mut(),
            None,
            &created.id,
            &ItemPatch {
                name: Some("Mine".to_owned()),
                ..ItemPatch::default()
            },
            LastEditedBy::User,
        )
        .expect("patch");

        let failed = set_status(
            db.conn_mut(),
            None,
            &created.id,
            ItemStatus::AssessmentFailed,
            Some("model call: timeout"),
        )
        .expect("status");

        assert_eq!(failed.last_edited_by, LastEditedBy::User);
        assert_eq!(failed.name, "Mine");
    }

    #[test]
    fn deleting_an_item_takes_its_search_row_with_it() {
        // A stale FTS row makes a deleted item findable and then un-openable.
        let (mut db, _dir) = library();
        let created = create(db.conn_mut(), None, &new_item()).expect("create");
        delete(db.conn_mut(), None, &created.id).expect("delete");

        let orphans: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM items_fts", [], |row| row.get(0))
            .expect("count");
        assert_eq!(orphans, 0);
    }

    #[test]
    fn deleting_an_item_removes_its_directory() {
        let (mut db, dir) = library();
        let root = dir.path().to_path_buf();
        let created = create(db.conn_mut(), Some(&root), &new_item()).expect("create");

        delete(db.conn_mut(), Some(&root), &created.id).expect("delete");

        assert!(!root.join("items").join(&created.id).exists());
    }

    #[test]
    fn an_empty_patch_changes_nothing_but_the_timestamp() {
        assert!(ItemPatch::default().is_empty());
        assert!(
            !ItemPatch {
                name: Some("x".to_owned()),
                ..ItemPatch::default()
            }
            .is_empty()
        );
    }
}
