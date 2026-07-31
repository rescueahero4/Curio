//! Full-text search over items.
//!
//! FTS5, unconditionally. The previous implementation treated FTS5 as optional and
//! degraded to `LIKE` when the runtime's SQLite lacked it — a reasonable hedge when the
//! host provided SQLite. We bundle it now, so the extension is always compiled in and the
//! fallback is retired (R-DA-10; a deliberate break, ARCH-08 #6). One search path means
//! one set of ranking behavior to reason about instead of two that quietly differ.
//!
//! The table is **non-contentless** — it stores its four columns rather than reading them
//! back from `items`. An external-content table keyed by ULID would need a rowid mapping
//! maintained by hand; storing four small text columns is cheaper to keep correct, and it
//! means the index can be rebuilt from `items` at any time without a mapping to repair.
//!
//! Whatever writes an item writes this in the same transaction (R-DA-4). An index that can
//! disagree with the table it indexes is worse than no index, because the disagreement is
//! invisible until a user notices a capture they know exists is unfindable.

use rusqlite::Connection;

use crate::Result;

/// Create the search index if it is absent.
///
/// # Errors
/// Propagates a storage failure. Unlike the previous implementation, a failure here is a
/// real error rather than a signal to degrade: with bundled SQLite, FTS5 missing means
/// something is wrong with the build, not with the user's machine.
pub fn create(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE VIRTUAL TABLE IF NOT EXISTS items_fts USING fts5(
           item_id UNINDEXED,
           name,
           short_description,
           tags_concat,
           tokenize = 'unicode61'
         )",
    )?;
    Ok(())
}

/// Turn a user's search box into an FTS5 MATCH expression.
///
/// Three rules, all inherited because each was earned (Inventory §4):
///
/// * **Every term is quoted.** FTS5's query language treats `AND`, `OR`, `NOT`, `*`, `^`,
///   `-` and `:` as syntax. A user typing `pricing - dark` is searching, not writing a
///   boolean expression, and an unquoted term turns their search into a syntax error.
/// * **Terms are ANDed.** Adding a word narrows results, which is what every search box a
///   user has ever used does.
/// * **Only the last term gets a prefix wildcard.** Search runs while typing, so the last
///   word is usually half-finished — but wildcarding earlier words would make `dark mode`
///   match `darkroom`, and the user has finished typing those.
///
/// Returns `None` when there is nothing to search for, so callers can skip the join
/// rather than match against an empty string.
#[must_use]
pub fn build_match_query(raw: &str) -> Option<String> {
    let terms: Vec<&str> = raw.split_whitespace().collect();
    if terms.is_empty() {
        return None;
    }

    let last = terms.len() - 1;
    let expression = terms
        .iter()
        .enumerate()
        .map(|(index, term)| {
            // Double quotes are the escape *inside* an FTS5 string literal.
            let escaped = term.replace('"', "\"\"");
            if index == last {
                format!("\"{escaped}\"*")
            } else {
                format!("\"{escaped}\"")
            }
        })
        .collect::<Vec<_>>()
        .join(" AND ");

    Some(expression)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn indexed() -> Connection {
        let conn = Connection::open_in_memory().expect("open");
        create(&conn).expect("create");
        conn.execute_batch(
            "INSERT INTO items_fts (item_id, name, short_description, tags_concat) VALUES
               ('01A', 'Stripe pricing page', 'A clean pricing table on white', 'saas pricing minimal'),
               ('01B', 'Linear dark dashboard', 'Dense dark dashboard UI', 'dark dashboard dense'),
               ('01C', 'Darkroom photo site', 'Photography portfolio', 'photography dark')",
        )
        .expect("seed");
        conn
    }

    fn search(conn: &Connection, raw: &str) -> Vec<String> {
        let Some(query) = build_match_query(raw) else {
            return Vec::new();
        };
        let mut statement = conn
            .prepare("SELECT item_id FROM items_fts WHERE items_fts MATCH ?1 ORDER BY rank")
            .expect("prepare");
        let rows = statement
            .query_map([query], |row| row.get::<_, String>(0))
            .expect("query");
        rows.map(|row| row.expect("row")).collect()
    }

    #[test]
    fn fts5_is_available_because_we_bundle_sqlite() {
        // The premise of R-DA-10. If this ever fails, the LIKE fallback was retired on a
        // false assumption and search is broken, not degraded.
        create(&Connection::open_in_memory().expect("open")).expect("FTS5 must be compiled in");
    }

    #[test]
    fn a_partial_last_word_still_matches() {
        // Search runs while the user types. "dashb" is a word in progress.
        assert_eq!(search(&indexed(), "dashb"), vec!["01B"]);
    }

    #[test]
    fn earlier_words_are_not_wildcarded() {
        // "dark dashboard" must not reach "Darkroom" — the user finished typing "dark".
        let hits = search(&indexed(), "dark dashboard");
        assert_eq!(hits, vec!["01B"]);
    }

    #[test]
    fn extra_words_narrow_rather_than_widen() {
        let conn = indexed();
        assert_eq!(search(&conn, "dark").len(), 2);
        assert_eq!(search(&conn, "dark dense").len(), 1);
    }

    #[test]
    fn fts_operators_typed_by_a_user_are_searched_for_not_executed() {
        // The reason every term is quoted: these are all FTS5 syntax, and a user typing
        // them into a search box means the characters, not the operators. Unquoted, each
        // of these is a syntax error rather than an empty result.
        let conn = indexed();
        for raw in ["pricing - dark", "dark OR", "NOT dark", "dark^", "a:b"] {
            let query = build_match_query(raw).expect("query");
            let mut statement = conn
                .prepare("SELECT item_id FROM items_fts WHERE items_fts MATCH ?1")
                .expect("prepare");
            statement
                .query_map([&query], |row| row.get::<_, String>(0))
                .unwrap_or_else(|err| panic!("{raw:?} produced invalid FTS: {err}"))
                .for_each(|row| {
                    row.expect("row");
                });
        }
    }

    #[test]
    fn a_quote_in_the_query_does_not_break_the_expression() {
        let conn = indexed();
        let query = build_match_query("say \"hello\"").expect("query");
        let mut statement = conn
            .prepare("SELECT item_id FROM items_fts WHERE items_fts MATCH ?1")
            .expect("prepare");
        statement
            .query_map([&query], |row| row.get::<_, String>(0))
            .expect("a quoted term must stay well-formed")
            .for_each(|row| {
                row.expect("row");
            });
    }

    #[test]
    fn an_empty_search_is_none_rather_than_an_empty_match() {
        // Callers skip the join entirely; matching against "" is an FTS5 error.
        assert!(build_match_query("").is_none());
        assert!(build_match_query("   ").is_none());
    }

    #[test]
    fn tags_are_searchable() {
        assert_eq!(search(&indexed(), "photography"), vec!["01C"]);
    }
}
