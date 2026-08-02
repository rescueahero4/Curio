//! How the library is filtered, searched, and paged.
//!
//! Two rules shape everything here, and both are user-visible rather than technical.
//!
//! **AND across facets, OR within one** (FR-10). Ticking two tags widens — a user picking
//! `brutalist` and `editorial` wants either. Ticking a tag and a family narrows — they
//! want a brutalist thing that is also in that family. This is what every filter UI a
//! designer has used already does, and getting it backwards produces empty results that
//! look like missing data.
//!
//! **Keyset pagination, never offset** (R-FE-9, R-DA-5). The cursor is `(created_at, id)`.
//! An offset page shifts under the reader the moment a capture lands mid-scroll: rows slide
//! down by one and the next page repeats or skips. Monotonic ULIDs are what make the tie
//! break on `id` total.

use serde::{Deserialize, Serialize};

use crate::domain::ItemStatus;

/// The default page size, and the maximum a caller may ask for (Inventory §1).
pub const DEFAULT_PAGE_SIZE: usize = 60;
/// The largest page a caller may request. Beyond this the cost lands on the browser, which
/// has to render every card.
pub const MAX_PAGE_SIZE: usize = 200;

/// A position in the library's ordering.
///
/// Both halves are needed: `created_at` is second-precision, so a burst of captures shares
/// one timestamp and the id is what orders them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cursor {
    pub created_at: String,
    pub id: String,
}

impl Cursor {
    /// Parse the `created_at|id` form the API takes on the wire.
    ///
    /// # Errors
    /// Returns [`crate::Error::Invalid`] if the cursor is not two `|`-separated parts.
    pub fn parse(raw: &str) -> crate::Result<Self> {
        let (created_at, id) = raw
            .split_once('|')
            .ok_or_else(|| crate::Error::invalid("a cursor is created_at|id"))?;
        if created_at.is_empty() || id.is_empty() {
            return Err(crate::Error::invalid("a cursor is created_at|id"));
        }
        Ok(Self {
            created_at: created_at.to_owned(),
            id: id.to_owned(),
        })
    }

    /// The wire form.
    #[must_use]
    pub fn encode(&self) -> String {
        format!("{}|{}", self.created_at, self.id)
    }
}

/// What the library grid is asking for.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ItemQuery {
    /// Design-type ids. Empty means no constraint.
    pub types: Vec<String>,
    /// Family ids.
    pub families: Vec<String>,
    /// Tag ids.
    pub tags: Vec<String>,
    pub statuses: Vec<ItemStatus>,
    /// Free text, matched by FTS5 (R-DA-10).
    pub search: Option<String>,
    /// Only items awaiting a decision — held at `needs_review` **or** carrying a gray-zone
    /// link. Two conditions, because a threshold change can produce the second without the
    /// first.
    pub needs_review: bool,
    pub limit: usize,
    pub cursor: Option<Cursor>,
}

impl ItemQuery {
    /// A query with the default page size and no constraints.
    #[must_use]
    pub fn unfiltered() -> Self {
        Self {
            limit: DEFAULT_PAGE_SIZE,
            ..Self::default()
        }
    }

    /// Whether anything narrows this query.
    ///
    /// Load-bearing on the client (R-FE-10, Inventory §10.25): `item.created` prepends a
    /// card only in the unfiltered view. Prepending into a filtered grid would show the
    /// user a card that does not match what they asked for.
    #[must_use]
    pub fn is_filtered(&self) -> bool {
        !self.types.is_empty()
            || !self.families.is_empty()
            || !self.tags.is_empty()
            || !self.statuses.is_empty()
            || self.needs_review
            || self
                .search
                .as_deref()
                .is_some_and(|term| !term.trim().is_empty())
    }

    /// Clamp the page size into the documented range.
    ///
    /// A caller asking for zero means "the default", not "nothing" — an empty page with a
    /// cursor is indistinguishable from the end of the library.
    #[must_use]
    pub fn with_limit(mut self, requested: Option<usize>) -> Self {
        self.limit = match requested {
            None | Some(0) => DEFAULT_PAGE_SIZE,
            Some(value) => value.min(MAX_PAGE_SIZE),
        };
        self
    }
}

/// One page of results.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Page<T> {
    pub items: Vec<T>,
    /// Absent when this was the last page.
    pub next_cursor: Option<String>,
}

/// Refuse a selection that exceeds the cap, naming both numbers.
///
/// Never a silent trim (R-BE-18, R-FE-11): trimming gives the user a confident wrong
/// answer about what they just changed, and they find out later, one item at a time.
///
/// # Errors
/// Returns [`crate::Error::OverCap`] when `matched` exceeds [`crate::domain::BULK_ITEM_CAP`].
pub fn enforce_bulk_cap(matched: usize) -> crate::Result<()> {
    let limit = crate::domain::BULK_ITEM_CAP;
    if matched > limit {
        return Err(crate::Error::OverCap { matched, limit });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cursor_round_trips() {
        let cursor = Cursor {
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            id: "01J000000000000000000000".to_owned(),
        };

        assert_eq!(Cursor::parse(&cursor.encode()).expect("parse"), cursor);
    }

    #[test]
    fn a_malformed_cursor_is_refused() {
        // Silently starting from the beginning would loop the grid forever: the client
        // asks for the next page, receives page one, and appends it.
        assert!(Cursor::parse("nonsense").is_err());
        assert!(Cursor::parse("|01J").is_err());
        assert!(Cursor::parse("2026-01-01T00:00:00Z|").is_err());
    }

    #[test]
    fn an_unfiltered_query_is_unfiltered() {
        assert!(!ItemQuery::unfiltered().is_filtered());
    }

    #[test]
    fn any_facet_makes_a_query_filtered() {
        // R-FE-10 hangs off this: `item.created` prepends only in the unfiltered view.
        for query in [
            ItemQuery {
                tags: vec!["t".to_owned()],
                ..ItemQuery::unfiltered()
            },
            ItemQuery {
                families: vec!["f".to_owned()],
                ..ItemQuery::unfiltered()
            },
            ItemQuery {
                types: vec!["d".to_owned()],
                ..ItemQuery::unfiltered()
            },
            ItemQuery {
                statuses: vec![ItemStatus::Ready],
                ..ItemQuery::unfiltered()
            },
            ItemQuery {
                needs_review: true,
                ..ItemQuery::unfiltered()
            },
            ItemQuery {
                search: Some("dark".to_owned()),
                ..ItemQuery::unfiltered()
            },
        ] {
            assert!(query.is_filtered(), "{query:?}");
        }
    }

    #[test]
    fn a_blank_search_box_does_not_count_as_a_filter() {
        // The search input is cleared to "" rather than to null, and treating that as an
        // active filter would stop new captures appearing after a user clears their search.
        let query = ItemQuery {
            search: Some("   ".to_owned()),
            ..ItemQuery::unfiltered()
        };

        assert!(!query.is_filtered());
    }

    #[test]
    fn page_size_is_clamped_to_the_documented_range() {
        assert_eq!(
            ItemQuery::default().with_limit(None).limit,
            DEFAULT_PAGE_SIZE
        );
        assert_eq!(
            ItemQuery::default().with_limit(Some(0)).limit,
            DEFAULT_PAGE_SIZE
        );
        assert_eq!(ItemQuery::default().with_limit(Some(25)).limit, 25);
        assert_eq!(
            ItemQuery::default().with_limit(Some(9_000)).limit,
            MAX_PAGE_SIZE
        );
    }

    #[test]
    fn the_bulk_cap_refuses_rather_than_trims() {
        // Inventory §10.11. A trim is a confident wrong answer the user discovers later,
        // one item at a time.
        assert!(enforce_bulk_cap(500).is_ok());
        match enforce_bulk_cap(812) {
            Err(crate::Error::OverCap { matched, limit }) => {
                assert_eq!(matched, 812);
                assert_eq!(limit, 500);
            }
            other => panic!("expected an over-cap refusal, got {other:?}"),
        }
    }
}
