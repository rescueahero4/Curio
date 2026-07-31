//! Turning one model reply into one item's vocabulary.
//!
//! Split from the orchestration because the interesting decisions are here and none of
//! them involve the network: what `replace` removes, why a case-only difference is not a
//! change, and when writing nothing is the right answer.
//!
//! ## Writing nothing is a feature
//!
//! A 500-item run where the model returns what each item already had must not touch a
//! single row. `bulk_edit` bumps `updated_at` on everything it is handed, and the grid is
//! sorted by it — so a "no-op" that wrote anyway would silently reorder the user's entire
//! library to say nothing (Inventory §9, §10.12).

use curio_core::Error;
use curio_core::ai::prompt::ItemSummary;
use curio_core::domain::Item;
use curio_core::events::{Event, EventName};
use curio_db::items::{self, BulkEdit};

use super::super::Worker;
use super::Mode;

/// Apply one reply to one item. Returns whether anything actually changed.
pub(super) fn apply(
    worker: &Worker,
    item: &Item,
    mode: Mode,
    reply: &str,
) -> curio_core::Result<bool> {
    #[derive(serde::Deserialize)]
    struct Retagged {
        #[serde(default)]
        tags: Vec<String>,
        #[serde(default)]
        design_types: Vec<String>,
    }

    let parsed: Retagged = serde_json::from_str(reply)
        .map_err(|err| Error::invalid(format!("the retag reply was malformed: {err}")))?;

    use curio_core::assessment::clean_vocabulary_names;
    let tags = clean_vocabulary_names(&parsed.tags);
    let types = clean_vocabulary_names(&parsed.design_types);

    let edit = match mode {
        Mode::Augment => BulkEdit {
            add_tags: tags,
            add_types: types,
            ..BulkEdit::default()
        },
        Mode::Replace => BulkEdit {
            remove_tags: missing_from(&item.tags, &tags),
            remove_types: missing_from(&item.design_types, &types),
            add_tags: tags,
            add_types: types,
            ..BulkEdit::default()
        },
    };

    // Case-insensitive no-op detection (Inventory §9). Writing anyway would bump
    // `updated_at` on every item of a 500-item run that changed nothing, reordering the
    // user's whole grid for no reason.
    if is_noop(item, &edit) {
        return Ok(false);
    }

    let state = worker.state();
    let root = state.data_root().to_path_buf();
    let changed = state.with_db_mut(|db| {
        items::bulk_edit(
            db.conn_mut(),
            Some(&root),
            std::slice::from_ref(&item.id),
            &edit,
        )
    })?;

    for updated in &changed {
        if let Ok(payload) = serde_json::to_value(updated) {
            state.publish(Event::new(EventName::ItemUpdated, payload));
        }
    }
    Ok(!changed.is_empty())
}

/// Names in `current` that `next` does not contain, compared case-insensitively.
fn missing_from(current: &[String], next: &[String]) -> Vec<String> {
    current
        .iter()
        .filter(|existing| {
            !next
                .iter()
                .any(|candidate| candidate.eq_ignore_ascii_case(existing))
        })
        .cloned()
        .collect()
}

fn is_noop(item: &Item, edit: &BulkEdit) -> bool {
    if !edit.remove_tags.is_empty() || !edit.remove_types.is_empty() {
        return false;
    }
    let already_has = |current: &[String], additions: &[String]| {
        additions
            .iter()
            .all(|name| current.iter().any(|held| held.eq_ignore_ascii_case(name)))
    };
    already_has(&item.tags, &edit.add_tags) && already_has(&item.design_types, &edit.add_types)
}

/// Replace mode can strand vocabulary nothing points at any more (Inventory §9).
pub(super) fn finish_run(worker: &Worker, mode: Mode) {
    if mode != Mode::Replace {
        return;
    }
    let state = worker.state();
    match state.with_db(|db| curio_db::vocabulary::prune_orphans(db.conn())) {
        Ok(0) => {}
        Ok(pruned) => {
            tracing::info!(pruned, "removed vocabulary nothing links to any more");
            state.publish(Event::vocabulary_updated());
        }
        Err(err) => tracing::warn!(%err, "could not prune orphaned vocabulary"),
    }
}

pub(super) fn summary(item: &Item) -> ItemSummary {
    ItemSummary {
        name: item.name.clone(),
        short_description: item.short_description.clone(),
        design_types: item.design_types.clone(),
        tags: item.tags.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(tags: &[&str], types: &[&str]) -> Item {
        Item {
            id: "01J".to_owned(),
            name: "Stripe pricing".to_owned(),
            short_description: "A clean pricing table".to_owned(),
            source_url: None,
            image_recipe: None,
            screenshot_path: "items/01J/screenshot.png".to_owned(),
            thumbnail_path: None,
            status: curio_core::domain::ItemStatus::Ready,
            last_edited_by: curio_core::domain::LastEditedBy::Ai,
            error: None,
            created_at: "2026-01-01T00:00:00Z".to_owned(),
            updated_at: "2026-01-01T00:00:00Z".to_owned(),
            design_types: types.iter().map(|s| (*s).to_owned()).collect(),
            tags: tags.iter().map(|s| (*s).to_owned()).collect(),
            families: Vec::new(),
        }
    }

    fn names(values: &[&str]) -> Vec<String> {
        values.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn replace_removes_only_what_the_new_set_leaves_out() {
        let removed = missing_from(&names(&["saas", "minimal"]), &names(&["minimal", "dark"]));
        assert_eq!(removed, vec!["saas"]);
    }

    #[test]
    fn a_case_only_difference_is_not_a_removal() {
        // Removing "SaaS" to add "saas" would delete the link and re-create it, churning
        // `updated_at` and the sidecar for a spelling the model chose differently.
        assert!(missing_from(&names(&["SaaS"]), &names(&["saas"])).is_empty());
    }

    #[test]
    fn an_augment_that_adds_nothing_new_is_a_no_op() {
        // Inventory §9. A 500-item run that changed nothing must not reorder the grid by
        // bumping every `updated_at`.
        let edit = BulkEdit {
            add_tags: names(&["SAAS"]),
            ..BulkEdit::default()
        };
        assert!(is_noop(&item(&["saas"], &[]), &edit));
    }

    #[test]
    fn an_augment_that_adds_something_is_not_a_no_op() {
        let edit = BulkEdit {
            add_tags: names(&["saas", "dark"]),
            ..BulkEdit::default()
        };
        assert!(!is_noop(&item(&["saas"], &[]), &edit));
    }

    #[test]
    fn any_removal_makes_it_a_real_edit() {
        let edit = BulkEdit {
            remove_tags: names(&["saas"]),
            ..BulkEdit::default()
        };
        assert!(!is_noop(&item(&["saas"], &[]), &edit));
    }
}
