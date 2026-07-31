//! Vocabulary dedupe: what the utility model returns, and what survives the filter.
//!
//! The result is **stored and shown, never auto-applied** (R-FE-15a, Inventory §10.8). A
//! merge destroys the distinction between two words across every item that used either, so
//! the model's job ends at "here is a group you might want to merge" and the user's begins.
//!
//! Everything here runs *after* the model, on the assumption that some of what came back is
//! wrong. Three specific wrongs are filtered rather than trusted:
//!
//! * **Hallucinated names.** A name the model invented cannot be merged — there is no row
//!   with that name — so a group containing one would fail at apply time, in front of the
//!   user, after they clicked.
//! * **Self-merges.** Folding a name into itself is a no-op that renders as a real
//!   suggestion, and a user who accepts it learns that accepting suggestions does nothing.
//! * **Empty groups.** An empty merge list is the model **withdrawing** the group. It is
//!   not an error and not a group with zero members; it is a "never mind", and showing it
//!   would be showing the user a question the model already retracted.

use serde::{Deserialize, Serialize};

/// One proposed merge.
///
/// Field order matches [`super::schema::DEDUPE`] deliberately — `reason` first, because
/// the model generates in schema order and reasoning before committing produces better
/// groups (Inventory §10.8).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Group {
    pub reason: String,
    /// The name to keep.
    pub canonical: String,
    /// The names to fold into it. Empty means the model withdrew this group.
    #[serde(default)]
    pub merge: Vec<String>,
}

/// The utility call's structured reply.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Output {
    #[serde(default)]
    pub groups: Vec<Group>,
}

/// Drop everything that could not be applied (Inventory §10.8).
///
/// `known` is the vocabulary as it actually exists. Matching is case-insensitive but the
/// **stored spelling wins**: a group naming "brutalist" is rewritten to the library's
/// "Brutalist" so the merge targets a real row rather than failing a lookup over casing.
#[must_use]
pub fn sanitize(output: &Output, known: &[String]) -> Vec<Group> {
    let resolve = |candidate: &str| -> Option<String> {
        let needle = candidate.trim().to_lowercase();
        known
            .iter()
            .find(|name| name.to_lowercase() == needle)
            .cloned()
    };

    output
        .groups
        .iter()
        .filter_map(|group| {
            let canonical = resolve(&group.canonical)?;
            let mut seen = std::collections::HashSet::new();
            let merge: Vec<String> = group
                .merge
                .iter()
                .filter_map(|name| resolve(name))
                .filter(|name| !name.eq_ignore_ascii_case(&canonical))
                .filter(|name| seen.insert(name.to_lowercase()))
                .collect();

            // Withdrawal, or nothing left after filtering — either way there is no
            // question worth putting in front of the user.
            if merge.is_empty() {
                return None;
            }

            Some(Group {
                reason: group.reason.trim().to_owned(),
                canonical,
                merge,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known() -> Vec<String> {
        vec![
            "Brutalist".to_owned(),
            "brutalism".to_owned(),
            "Editorial".to_owned(),
            "Minimal".to_owned(),
        ]
    }

    fn group(canonical: &str, merge: &[&str]) -> Group {
        Group {
            reason: "same thing".to_owned(),
            canonical: canonical.to_owned(),
            merge: merge.iter().map(|s| (*s).to_owned()).collect(),
        }
    }

    fn sanitized(groups: Vec<Group>) -> Vec<Group> {
        sanitize(&Output { groups }, &known())
    }

    #[test]
    fn a_hallucinated_canonical_drops_the_whole_group() {
        // There is no row called "Neo-Brutalist" to merge into, so the group would fail at
        // apply time — in front of the user, after they clicked accept.
        assert!(sanitized(vec![group("Neo-Brutalist", &["Brutalist"])]).is_empty());
    }

    #[test]
    fn a_hallucinated_member_is_dropped_without_losing_the_group() {
        let result = sanitized(vec![group("Brutalist", &["brutalism", "Invented"])]);

        assert_eq!(result.len(), 1);
        assert_eq!(result[0].merge, vec!["brutalism"]);
    }

    #[test]
    fn a_self_merge_is_removed_and_can_empty_a_group() {
        // Folding a name into itself renders as a real suggestion that does nothing when
        // accepted.
        assert!(sanitized(vec![group("Brutalist", &["brutalist"])]).is_empty());
    }

    #[test]
    fn an_empty_merge_list_is_a_withdrawal_not_an_error() {
        // Inventory §10.8. The model reconsidered; showing the group anyway would put a
        // retracted question in front of the user.
        assert!(sanitized(vec![group("Brutalist", &[])]).is_empty());
    }

    #[test]
    fn the_libraries_own_spelling_wins() {
        // The merge has to resolve to a real row. "brutalist" lowercase would miss a
        // lookup that the stored "Brutalist" satisfies.
        let result = sanitized(vec![group("brutalist", &["BRUTALISM"])]);

        assert_eq!(result[0].canonical, "Brutalist");
        assert_eq!(result[0].merge, vec!["brutalism"]);
    }

    #[test]
    fn a_repeated_member_is_listed_once() {
        let result = sanitized(vec![group("Brutalist", &["brutalism", "Brutalism"])]);
        assert_eq!(result[0].merge.len(), 1);
    }

    #[test]
    fn a_clean_vocabulary_produces_nothing_rather_than_something() {
        assert!(sanitize(&Output::default(), &known()).is_empty());
    }

    #[test]
    fn the_reply_parses_with_groups_absent() {
        // "Nothing to merge" is a good answer, and the model may express it by omitting
        // the array rather than sending an empty one.
        let parsed: Output = serde_json::from_str("{}").expect("parse");
        assert!(parsed.groups.is_empty());
    }

    #[test]
    fn a_group_serializes_with_reason_first() {
        // The stored result is re-read by the UI, and the field order documents the
        // generation-order fix for anyone who opens the row.
        let json = serde_json::to_string(&group("Brutalist", &["brutalism"])).expect("serialize");
        assert!(json.starts_with(r#"{"reason":"#), "{json}");
    }
}
