//! Reading and writing items.
//!
//! The vocabulary an item carries lives in three link tables, so an `Item` is never one
//! row. Rather than a join that multiplies rows by tags × types × families and is then
//! de-duplicated in Rust, this module reads the page of items and then fetches its links in
//! three further queries keyed by the page's ids ([`hydrate`]). Four small statements beat
//! one that returns a hundred rows per item and makes the ordering harder to reason about.

pub mod bulk;
pub mod curate;
pub mod links;
pub mod list;
pub mod write;

use rusqlite::{Connection, Row};

use curio_core::domain::{Item, ItemFamily, ItemStatus, LastEditedBy};

use crate::{Error, Result};

pub use bulk::{BulkEdit, bulk_edit};
pub use curate::{GrayZoneDecision, apply_assessment, resolve_gray_zone};
pub use list::{count, count_matching, list, matching_ids};
pub use write::{ItemPatch, NewItem, create, delete, patch, set_media, set_status, touch};

/// Read one item, with its vocabulary.
///
/// # Errors
/// Propagates a storage failure.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Item>> {
    let bare = conn
        .query_row(
            "SELECT id, name, short_description, source_url, image_recipe, screenshot_path,
                    thumbnail_path, status, last_edited_by, error, created_at, updated_at
               FROM items WHERE id = ?1",
            [id],
            map_row,
        )
        .map(Some)
        .or_else(|err| match err {
            rusqlite::Error::QueryReturnedNoRows => Ok(None),
            other => Err(Error::Sqlite(other)),
        })?;

    match bare {
        Some(item) => Ok(hydrate(conn, vec![item])?.pop()),
        None => Ok(None),
    }
}

/// Read one item or fail with a domain-level not-found.
///
/// # Errors
/// Returns [`Error::NotFound`] if there is no such item.
pub fn require(conn: &Connection, id: &str) -> Result<Item> {
    get(conn, id)?.ok_or_else(|| Error::NotFound {
        kind: "item",
        id: id.to_owned(),
    })
}

/// Attach tags, design types, and families to a page of items.
///
/// # Errors
/// Propagates a storage failure.
pub fn hydrate(conn: &Connection, mut items: Vec<Item>) -> Result<Vec<Item>> {
    if items.is_empty() {
        return Ok(items);
    }

    let ids: Vec<String> = items.iter().map(|item| item.id.clone()).collect();
    let placeholders = placeholders(ids.len());
    let bound = rusqlite::params_from_iter(ids.iter());

    let mut tags = std::collections::HashMap::<String, Vec<String>>::new();
    collect_names(
        conn,
        &format!(
            "SELECT l.item_id, t.name FROM item_tags l JOIN tags t ON t.id = l.tag_id
              WHERE l.item_id IN ({placeholders}) ORDER BY t.name COLLATE NOCASE"
        ),
        rusqlite::params_from_iter(ids.iter()),
        &mut tags,
    )?;

    let mut types = std::collections::HashMap::<String, Vec<String>>::new();
    collect_names(
        conn,
        &format!(
            "SELECT l.item_id, d.name FROM item_types l JOIN design_types d ON d.id = l.type_id
              WHERE l.item_id IN ({placeholders}) ORDER BY d.name COLLATE NOCASE"
        ),
        bound,
        &mut types,
    )?;

    let mut families = std::collections::HashMap::<String, Vec<ItemFamily>>::new();
    {
        let sql = format!(
            "SELECT l.item_id, f.id, f.name, l.score, l.gray_zone, l.ai_proposed
               FROM item_families l JOIN aesthetic_families f ON f.id = l.family_id
              WHERE l.item_id IN ({placeholders}) ORDER BY l.score DESC, f.name COLLATE NOCASE"
        );
        let mut statement = conn.prepare(&sql)?;
        let mut rows = statement.query(rusqlite::params_from_iter(ids.iter()))?;
        while let Some(row) = rows.next()? {
            families
                .entry(row.get::<_, String>(0)?)
                .or_default()
                .push(ItemFamily {
                    id: row.get(1)?,
                    name: row.get(2)?,
                    score: row.get(3)?,
                    gray_zone: row.get::<_, i64>(4)? != 0,
                    ai_proposed: row.get::<_, i64>(5)? != 0,
                });
        }
    }

    for item in &mut items {
        item.tags = tags.remove(&item.id).unwrap_or_default();
        item.design_types = types.remove(&item.id).unwrap_or_default();
        item.families = families.remove(&item.id).unwrap_or_default();
    }
    Ok(items)
}

/// Map a bare `items` row. The vocabulary arrives later, in [`hydrate`].
pub(crate) fn map_row(row: &Row<'_>) -> rusqlite::Result<Item> {
    Ok(Item {
        id: row.get(0)?,
        name: row.get(1)?,
        short_description: row.get(2)?,
        source_url: row.get(3)?,
        image_recipe: row.get(4)?,
        screenshot_path: row.get(5)?,
        thumbnail_path: row.get(6)?,
        status: parse_status(&row.get::<_, String>(7)?),
        last_edited_by: parse_authorship(&row.get::<_, String>(8)?),
        error: row.get(9)?,
        created_at: row.get(10)?,
        updated_at: row.get(11)?,
        design_types: Vec::new(),
        tags: Vec::new(),
        families: Vec::new(),
    })
}

/// The column list every read of `items` uses, in [`map_row`]'s order.
pub(crate) const COLUMNS: &str = "id, name, short_description, source_url, image_recipe, \
     screenshot_path, thumbnail_path, status, last_edited_by, error, created_at, updated_at";

/// `?1, ?2, …` for an `IN` list of `count` values.
pub(crate) fn placeholders(count: usize) -> String {
    (1..=count)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn collect_names(
    conn: &Connection,
    sql: &str,
    params: impl rusqlite::Params,
    into: &mut std::collections::HashMap<String, Vec<String>>,
) -> Result<()> {
    let mut statement = conn.prepare(sql)?;
    let mut rows = statement.query(params)?;
    while let Some(row) = rows.next()? {
        into.entry(row.get::<_, String>(0)?)
            .or_default()
            .push(row.get::<_, String>(1)?);
    }
    Ok(())
}

/// The database's CHECK constraint already guarantees one of four values, so an unknown
/// string means the row was written by something other than Curio. Treating it as
/// `Processing` keeps the item visible and editable rather than hiding it behind an error.
fn parse_status(raw: &str) -> ItemStatus {
    ItemStatus::all()
        .into_iter()
        .find(|status| status.as_str() == raw)
        .unwrap_or(ItemStatus::Processing)
}

fn parse_authorship(raw: &str) -> LastEditedBy {
    if raw == "user" {
        LastEditedBy::User
    } else {
        LastEditedBy::Ai
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    #[test]
    fn a_missing_item_is_none_rather_than_an_error() {
        let db = Db::open_in_memory().expect("open");
        assert!(get(db.conn(), "01NOPE").expect("get").is_none());
    }

    #[test]
    fn requiring_a_missing_item_names_it() {
        let db = Db::open_in_memory().expect("open");
        match require(db.conn(), "01NOPE") {
            Err(Error::NotFound { kind, id }) => {
                assert_eq!(kind, "item");
                assert_eq!(id, "01NOPE");
            }
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn hydrating_nothing_is_not_a_query() {
        // The `IN ()` an empty id list would build is a SQL syntax error, and this is the
        // path an empty last page takes on every scroll to the bottom.
        let db = Db::open_in_memory().expect("open");
        assert!(hydrate(db.conn(), Vec::new()).expect("hydrate").is_empty());
    }

    #[test]
    fn an_unknown_status_string_does_not_hide_the_item() {
        assert_eq!(parse_status("processing"), ItemStatus::Processing);
        assert_eq!(parse_status("ready"), ItemStatus::Ready);
        assert_eq!(parse_status("something_else"), ItemStatus::Processing);
    }

    #[test]
    fn placeholders_are_one_indexed_and_comma_separated() {
        assert_eq!(placeholders(1), "?1");
        assert_eq!(placeholders(3), "?1, ?2, ?3");
    }
}
