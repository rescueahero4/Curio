//! Timestamps.
//!
//! Every stored timestamp is ISO-8601 UTC at **second** precision, with exactly one
//! exception: `prompts.sent_at` keeps milliseconds because it is an ordering key — the
//! watcher matches a new project folder against the most recent outstanding claim, and
//! two prompts sent in the same second must still be distinguishable (R-DA-6,
//! Inventory §10.15).
//!
//! Both formats are fixed-width. The previous implementation used JavaScript's
//! `toISOString()`, which always emits three subsecond digits, so a format that dropped
//! trailing zeros would sort differently against existing rows — and these strings are
//! compared as text in keyset pagination.

use time::format_description::BorrowedFormatItem;
use time::macros::format_description;
use time::{OffsetDateTime, PrimitiveDateTime};

/// `2026-07-31T14:03:09Z` — what almost every column stores.
const SECONDS: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");

/// `2026-07-31T14:03:09.412Z` — `prompts.sent_at` only.
const MILLIS: &[BorrowedFormatItem<'_>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:3]Z");

/// Now, at second precision. The default for every timestamp column.
#[must_use]
pub fn now_iso() -> String {
    format_seconds(OffsetDateTime::now_utc())
}

/// Now, at millisecond precision. For `prompts.sent_at` and nothing else.
#[must_use]
pub fn now_iso_millis() -> String {
    format_millis(OffsetDateTime::now_utc())
}

/// Format an instant at second precision.
#[must_use]
pub fn format_seconds(at: OffsetDateTime) -> String {
    at.format(&SECONDS)
        .unwrap_or_else(|_| unreachable!("a fixed-width format cannot fail on a valid instant"))
}

/// Format an instant at millisecond precision.
#[must_use]
pub fn format_millis(at: OffsetDateTime) -> String {
    at.format(&MILLIS)
        .unwrap_or_else(|_| unreachable!("a fixed-width format cannot fail on a valid instant"))
}

/// An instant `seconds` in the future, at second precision.
///
/// Used for job parking (`not_before`) and retry backoff, where the value is written to
/// the database and compared as text against `now_iso()`.
#[must_use]
pub fn seconds_from_now(seconds: i64) -> String {
    format_seconds(OffsetDateTime::now_utc() + time::Duration::seconds(seconds))
}

/// Parse a stored timestamp.
///
/// # Errors
/// Returns [`crate::Error::Invalid`] if `raw` is neither format.
pub fn parse(raw: &str) -> crate::Result<OffsetDateTime> {
    // Parsed as a primitive and then assumed UTC: the trailing `Z` in both formats is a
    // literal character, not an offset specifier, so there is nothing for an offset-aware
    // parse to read. Assuming UTC is correct rather than convenient — every timestamp
    // Curio writes is UTC by construction (R-DA-6).
    PrimitiveDateTime::parse(raw, &MILLIS)
        .or_else(|_| PrimitiveDateTime::parse(raw, &SECONDS))
        .map(PrimitiveDateTime::assume_utc)
        .map_err(|_| crate::Error::invalid(format!("not a timestamp: {raw}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn second_precision_is_fixed_width() {
        let stamp = now_iso();
        assert_eq!(stamp.len(), 20, "{stamp}");
        assert!(stamp.ends_with('Z'), "{stamp}");
        assert!(!stamp.contains('.'), "no subsecond digits: {stamp}");
    }

    #[test]
    fn millisecond_precision_always_emits_three_digits() {
        // Fixed width matters: these strings are compared as text, and existing rows were
        // written by JavaScript's toISOString(), which never abbreviates.
        let stamp = now_iso_millis();
        assert_eq!(stamp.len(), 24, "{stamp}");
        assert_eq!(&stamp[19..20], ".", "{stamp}");
    }

    #[test]
    fn a_whole_second_still_emits_its_zeros() {
        // The case a "trim trailing zeros" formatter gets wrong, silently, only sometimes.
        let midnight = OffsetDateTime::UNIX_EPOCH;
        assert_eq!(format_millis(midnight), "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn text_ordering_matches_chronological_ordering() {
        // Keyset pagination compares these as strings, so this is the property that makes
        // the cursor correct rather than approximately correct.
        let earlier = format_seconds(OffsetDateTime::UNIX_EPOCH);
        let later = format_seconds(OffsetDateTime::UNIX_EPOCH + time::Duration::days(20_000));
        assert!(earlier < later, "{earlier} should sort before {later}");
    }

    #[test]
    fn parses_both_of_our_own_formats() {
        assert!(parse(&now_iso()).is_ok());
        assert!(parse(&now_iso_millis()).is_ok());
        assert!(parse("yesterday").is_err());
    }

    #[test]
    fn parking_produces_a_future_instant() {
        assert!(seconds_from_now(30) > now_iso());
    }
}
