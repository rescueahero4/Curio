//! Filtering, searching, and paging the library (FR-10).
//!
//! Two rules the SQL below exists to satisfy:
//!
//! * **AND across facets, OR within one.** Each facet contributes one `EXISTS` clause, and
//!   the ids inside it are an `IN` list. That is what makes two tags widen and a tag plus a
//!   family narrow — the behaviour every filter UI a designer has used already has.
//! * **Keyset, never offset** (R-FE-9). The cursor is the row value `(created_at, id)`.
//!   With an offset, a capture landing mid-scroll slides every later row down by one and the
//!   next page repeats or skips. Monotonic ULIDs (R-DA-5) are what make the `id` tie-break
//!   total, and therefore make the cursor exact rather than approximate.

use rusqlite::Connection;
use rusqlite::types::Value;

use curio_core::domain::Item;
use curio_core::query::{Cursor, ItemQuery, Page};

use crate::Result;
use crate::fts;

use super::{COLUMNS, hydrate, map_row};

/// One page of the library, newest first.
///
/// # Errors
/// Propagates a storage failure.
pub fn list(conn: &Connection, query: &ItemQuery) -> Result<Page<Item>> {
    let mut params = Vec::new();
    let where_clause = build_where(query, &mut params);

    // One row more than asked for. Its presence is what says "there is a next page"; a
    // separate COUNT would be a second query racing the first.
    let probe = query.limit.saturating_add(1);
    params.push(Value::from(i64::try_from(probe).unwrap_or(i64::MAX)));

    let sql = format!(
        "SELECT {COLUMNS} FROM items i {where_clause}
          ORDER BY i.created_at DESC, i.id DESC LIMIT ?{}",
        params.len()
    );

    let mut found = Vec::with_capacity(query.limit);
    {
        let mut statement = conn.prepare(&sql)?;
        let mut rows = statement.query(rusqlite::params_from_iter(params.iter()))?;
        while let Some(row) = rows.next()? {
            found.push(map_row(row)?);
        }
    }

    let has_more = found.len() > query.limit;
    found.truncate(query.limit);

    let next_cursor = has_more.then(|| {
        found.last().map(|last| {
            Cursor {
                created_at: last.created_at.clone(),
                id: last.id.clone(),
            }
            .encode()
        })
    });

    Ok(Page {
        items: hydrate(conn, found)?,
        next_cursor: next_cursor.flatten(),
    })
}

/// Every id the filter matches, oldest-id-last, ignoring the page size.
///
/// Used to freeze a bulk operation's membership at enqueue (R-BE-18). The caller applies
/// the cap and refuses over it — this function deliberately does not truncate, because the
/// count is what the refusal has to report.
///
/// # Errors
/// Propagates a storage failure.
pub fn matching_ids(conn: &Connection, query: &ItemQuery) -> Result<Vec<String>> {
    let mut params = Vec::new();
    let where_clause = build_where(query, &mut params);
    let sql =
        format!("SELECT i.id FROM items i {where_clause} ORDER BY i.created_at DESC, i.id DESC");

    let mut statement = conn.prepare(&sql)?;
    let rows = statement.query_map(rusqlite::params_from_iter(params.iter()), |row| row.get(0))?;
    Ok(rows.collect::<rusqlite::Result<Vec<String>>>()?)
}

/// How many items the library holds. Reported by `/health` (R-SEC-11).
///
/// # Errors
/// Propagates a storage failure.
pub fn count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))?)
}

/// How many items a filter matches, whole — not one page of them.
///
/// The library's filter row wants to say "Needs review 12" before the user has clicked
/// anything, and the paged read cannot answer that: it deliberately reports "there is
/// another page" rather than a total, because a total is a second query it does not need.
/// This is that second query, asked only by a caller that actually wants the number.
///
/// The cursor is dropped rather than honoured. A cursor means "the part of this result the
/// reader has not reached yet", and a count of that is not a count of the filter — it would
/// shrink as the user scrolled.
///
/// # Errors
/// Propagates a storage failure.
pub fn count_matching(conn: &Connection, query: &ItemQuery) -> Result<i64> {
    let whole = ItemQuery {
        cursor: None,
        ..query.clone()
    };

    let mut params = Vec::new();
    let where_clause = build_where(&whole, &mut params);
    let sql = format!("SELECT COUNT(*) FROM items i {where_clause}");

    Ok(
        conn.query_row(&sql, rusqlite::params_from_iter(params.iter()), |row| {
            row.get(0)
        })?,
    )
}

/// Build the `WHERE` clause and push its bound values onto `params`.
fn build_where(query: &ItemQuery, params: &mut Vec<Value>) -> String {
    let mut clauses: Vec<String> = Vec::new();

    facet(&mut clauses, params, &query.tags, "item_tags", "tag_id");
    facet(&mut clauses, params, &query.types, "item_types", "type_id");
    facet(
        &mut clauses,
        params,
        &query.families,
        "item_families",
        "family_id",
    );

    if !query.statuses.is_empty() {
        let start = params.len();
        for status in &query.statuses {
            params.push(Value::from(status.as_str().to_owned()));
        }
        clauses.push(format!(
            "i.status IN ({})",
            numbered(start, query.statuses.len())
        ));
    }

    if query.needs_review {
        // Two conditions, not one. Lowering a threshold in Settings can put an already
        // `ready` item's link back inside the gray band without touching the status
        // column, and the review queue has to follow the data.
        clauses.push(
            "(i.status = 'needs_review' OR EXISTS (SELECT 1 FROM item_families g \
              WHERE g.item_id = i.id AND g.gray_zone = 1))"
                .to_owned(),
        );
    }

    if let Some(expression) = query.search.as_deref().and_then(fts::build_match_query) {
        params.push(Value::from(expression));
        clauses.push(format!(
            "i.id IN (SELECT item_id FROM items_fts WHERE items_fts MATCH ?{})",
            params.len()
        ));
    }

    if let Some(cursor) = &query.cursor {
        params.push(Value::from(cursor.created_at.clone()));
        let created = params.len();
        params.push(Value::from(cursor.id.clone()));
        // A row value comparison rather than `created_at < ? OR (created_at = ? AND id < ?)`:
        // one expression, and SQLite can use the (created_at, id) ordering directly.
        clauses.push(format!(
            "(i.created_at, i.id) < (?{created}, ?{})",
            params.len()
        ));
    }

    if clauses.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", clauses.join(" AND "))
    }
}

/// One facet: `EXISTS (… IN (ids))`. Empty means no constraint, never "match nothing".
fn facet(
    clauses: &mut Vec<String>,
    params: &mut Vec<Value>,
    ids: &[String],
    link_table: &str,
    column: &str,
) {
    if ids.is_empty() {
        return;
    }
    let start = params.len();
    for id in ids {
        params.push(Value::from(id.clone()));
    }
    clauses.push(format!(
        "EXISTS (SELECT 1 FROM {link_table} l WHERE l.item_id = i.id AND l.{column} IN ({}))",
        numbered(start, ids.len())
    ));
}

fn numbered(start: usize, count: usize) -> String {
    (start + 1..=start + count)
        .map(|index| format!("?{index}"))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;
    use curio_core::domain::ItemStatus;

    /// A library with four items, deliberately sharing timestamps so the id tie-break is
    /// exercised rather than accidentally avoided.
    fn seeded() -> Db {
        let db = Db::open_in_memory().expect("open");
        db.conn()
            .execute_batch(
                "INSERT INTO items (id, name, short_description, screenshot_path, status, created_at, updated_at) VALUES
                   ('01A','Alpha','white pricing','items/01A/s.png','ready','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
                   ('01B','Bravo','dark dashboard','items/01B/s.png','ready','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z'),
                   ('01C','Charlie','warm editorial','items/01C/s.png','needs_review','2026-01-02T00:00:00Z','2026-01-02T00:00:00Z'),
                   ('01D','Delta','loud brutalist','items/01D/s.png','processing','2026-01-03T00:00:00Z','2026-01-03T00:00:00Z');
                 INSERT INTO tags (id, name, created_by) VALUES ('t1','saas','ai'), ('t2','dark','ai');
                 INSERT INTO design_types (id, name, created_by) VALUES ('d1','pricing page','ai');
                 INSERT INTO aesthetic_families (id, name, description, created_by, created_at, updated_at)
                   VALUES ('f1','Minimal','Quiet','ai','2026-01-01T00:00:00Z','2026-01-01T00:00:00Z');
                 INSERT INTO item_tags (item_id, tag_id) VALUES ('01A','t1'), ('01B','t2'), ('01C','t1');
                 INSERT INTO item_types (item_id, type_id) VALUES ('01A','d1');
                 INSERT INTO item_families (item_id, family_id, score, gray_zone, ai_proposed)
                   VALUES ('01A','f1',0.9,0,0), ('01B','f1',0.45,1,0);
                 INSERT INTO items_fts (item_id, name, short_description, tags_concat) VALUES
                   ('01A','Alpha','white pricing','saas'),
                   ('01B','Bravo','dark dashboard','dark'),
                   ('01C','Charlie','warm editorial','saas'),
                   ('01D','Delta','loud brutalist','');",
            )
            .expect("seed");
        db
    }

    fn ids(page: &Page<Item>) -> Vec<&str> {
        page.items.iter().map(|item| item.id.as_str()).collect()
    }

    #[test]
    fn the_library_comes_back_newest_first() {
        let db = seeded();
        let page = list(db.conn(), &ItemQuery::unfiltered()).expect("list");

        assert_eq!(ids(&page), ["01D", "01C", "01B", "01A"]);
        assert!(page.next_cursor.is_none(), "one page holds all four");
    }

    #[test]
    fn paging_walks_the_library_exactly_once() {
        // The property offset pagination cannot promise: every item seen, none twice.
        let db = seeded();
        let mut seen = Vec::new();
        let mut cursor = None;

        loop {
            let query = ItemQuery {
                limit: 2,
                cursor: cursor.clone(),
                ..ItemQuery::default()
            };
            let page = list(db.conn(), &query).expect("list");
            seen.extend(page.items.iter().map(|item| item.id.clone()));
            match page.next_cursor {
                Some(raw) => cursor = Some(Cursor::parse(&raw).expect("cursor")),
                None => break,
            }
        }

        assert_eq!(seen, ["01D", "01C", "01B", "01A"]);
    }

    #[test]
    fn items_sharing_a_timestamp_are_ordered_by_id() {
        // 01A and 01B were captured in the same second. Without the id tie-break the page
        // boundary between them is undefined and one of them can be skipped.
        let db = seeded();
        let first = list(
            db.conn(),
            &ItemQuery {
                limit: 3,
                ..ItemQuery::default()
            },
        )
        .expect("list");

        let next = list(
            db.conn(),
            &ItemQuery {
                limit: 3,
                cursor: Some(Cursor::parse(&first.next_cursor.expect("more")).expect("parse")),
                ..ItemQuery::default()
            },
        )
        .expect("list");

        assert_eq!(ids(&next), ["01A"]);
    }

    #[test]
    fn two_tags_widen_the_result() {
        // OR within a facet. A user ticking two tags wants either.
        let db = seeded();
        let page = list(
            db.conn(),
            &ItemQuery {
                tags: vec!["t1".to_owned(), "t2".to_owned()],
                ..ItemQuery::unfiltered()
            },
        )
        .expect("list");

        assert_eq!(ids(&page), ["01C", "01B", "01A"]);
    }

    #[test]
    fn a_tag_and_a_type_narrow_the_result() {
        // AND across facets. Only 01A is both.
        let db = seeded();
        let page = list(
            db.conn(),
            &ItemQuery {
                tags: vec!["t1".to_owned()],
                types: vec!["d1".to_owned()],
                ..ItemQuery::unfiltered()
            },
        )
        .expect("list");

        assert_eq!(ids(&page), ["01A"]);
    }

    #[test]
    fn needs_review_catches_a_gray_link_on_a_ready_item() {
        // 01B is `ready` but carries a gray-zone link — the exact case a status-only
        // filter loses, and the one a threshold change creates.
        let db = seeded();
        let page = list(
            db.conn(),
            &ItemQuery {
                needs_review: true,
                ..ItemQuery::unfiltered()
            },
        )
        .expect("list");

        assert_eq!(ids(&page), ["01C", "01B"]);
    }

    #[test]
    fn a_status_facet_filters_on_the_stored_string() {
        let db = seeded();
        let page = list(
            db.conn(),
            &ItemQuery {
                statuses: vec![ItemStatus::Processing],
                ..ItemQuery::unfiltered()
            },
        )
        .expect("list");

        assert_eq!(ids(&page), ["01D"]);
    }

    #[test]
    fn search_narrows_alongside_the_facets() {
        let db = seeded();
        let page = list(
            db.conn(),
            &ItemQuery {
                search: Some("dark".to_owned()),
                ..ItemQuery::unfiltered()
            },
        )
        .expect("list");

        assert_eq!(ids(&page), ["01B"]);
    }

    #[test]
    fn a_blank_search_does_not_filter_anything_out() {
        // The search box clears to "", and treating that as a match against nothing would
        // empty the grid the moment a user erased their query.
        let db = seeded();
        let page = list(
            db.conn(),
            &ItemQuery {
                search: Some("   ".to_owned()),
                ..ItemQuery::unfiltered()
            },
        )
        .expect("list");

        assert_eq!(page.items.len(), 4);
    }

    #[test]
    fn items_come_back_with_their_vocabulary_attached() {
        let db = seeded();
        let page = list(db.conn(), &ItemQuery::unfiltered()).expect("list");
        let alpha = page
            .items
            .iter()
            .find(|item| item.id == "01A")
            .expect("01A");

        assert_eq!(alpha.tags, ["saas"]);
        assert_eq!(alpha.design_types, ["pricing page"]);
        assert_eq!(alpha.families.len(), 1);
        assert_eq!(alpha.families[0].name, "Minimal");
        assert!((alpha.families[0].score - 0.9).abs() < f64::EPSILON);
    }

    #[test]
    fn matching_ids_ignores_the_page_size() {
        // Bulk membership is frozen from this, and a page-limited answer would silently
        // change what "everything matching" means.
        let db = seeded();
        let all = matching_ids(
            db.conn(),
            &ItemQuery {
                limit: 1,
                ..ItemQuery::default()
            },
        )
        .expect("ids");

        assert_eq!(all.len(), 4);
    }

    #[test]
    fn a_filtered_count_agrees_with_the_rows_that_filter_returns() {
        // The number on a pill and the grid under it come from two different queries. If
        // they can disagree, the pill is worse than no pill at all.
        let db = seeded();
        let query = ItemQuery {
            needs_review: true,
            ..ItemQuery::unfiltered()
        };

        let page = list(db.conn(), &query).expect("list");
        assert_eq!(
            count_matching(db.conn(), &query).expect("count"),
            i64::try_from(page.items.len()).expect("fits")
        );
        assert_eq!(count_matching(db.conn(), &query).expect("count"), 2);
    }

    #[test]
    fn a_count_ignores_the_page_size_and_the_cursor() {
        // Both would turn "how many match" into "how many are left", which is a different
        // question and a shrinking number.
        let db = seeded();
        let first = list(
            db.conn(),
            &ItemQuery {
                limit: 2,
                ..ItemQuery::default()
            },
        )
        .expect("list");

        let counted = count_matching(
            db.conn(),
            &ItemQuery {
                limit: 2,
                cursor: Some(Cursor::parse(&first.next_cursor.expect("more")).expect("parse")),
                ..ItemQuery::default()
            },
        )
        .expect("count");

        assert_eq!(counted, 4);
    }

    #[test]
    fn counting_an_empty_library_is_zero() {
        assert_eq!(
            count(Db::open_in_memory().expect("open").conn()).expect("count"),
            0
        );
    }
}
