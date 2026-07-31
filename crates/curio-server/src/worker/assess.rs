//! `assess_item`: one screenshot becomes a described, classified, filterable item.
//!
//! This is the job whose absence was visible from the outside — a capture landed, showed
//! as `processing`, and stayed there. Everything around it already worked.
//!
//! ## The shape is deliberately boring
//!
//! Read the item, downscale its screenshot, build one call, apply the answer. There is no
//! second pass, no retry-with-a-different-prompt, and no place where the model decides
//! anything Curio could decide itself. The one call is [`curio_core::ai::prompt::assessment`]
//! and the one decision is [`curio_db::items::apply_assessment`], which applies the
//! thresholds in ordinary code (FR-5, FR-6).

use curio_core::Error;
use curio_core::assessment::AssessmentOutput;
use curio_core::domain::{Item, ItemStatus, Job};
use curio_core::events::{Event, EventName};
use curio_db::items;

use super::Worker;

/// Run one assessment.
pub async fn run(worker: &Worker, job: &Job) -> curio_core::Result<serde_json::Value> {
    let item_id = job
        .payload
        .get("item_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| Error::invalid("this assessment job names no item"))?
        .to_owned();

    let state = worker.state();
    let root = state.data_root().to_path_buf();
    let item = state.with_db(|db| items::require(db.conn(), &item_id))?;

    // Before anything expensive. A user with no key must accumulate a queue that drains
    // the moment they add one, not a pile of half-done work (FR-26).
    let api_key = crate::secrets::api_key().ok_or(Error::MissingApiKey)?;

    let config = state.config();
    let request = {
        let image = payload_for(&root, &item)?;
        let vocabulary = super::assess_vocabulary(state)?;
        curio_core::ai::prompt::assessment(
            &config.models.vision,
            &read_rubric(&root),
            &vocabulary,
            &image,
            &curio_core::ai::Context {
                source_url: item.source_url.clone(),
                title: None,
            },
            config.thresholds,
        )
    };

    // The last boundary before spending money (R-BE-19). A user who cancelled while the
    // job sat in the queue should not be billed for it.
    if worker.should_stop(&job.id) {
        return Ok(serde_json::json!({ "cancelled": true }));
    }

    let reply = crate::ai::Anthropic::new(api_key)?
        .messages(&request)
        .await?;

    // Re-validated rather than trusted: `output_config.format` constrains the model, it
    // does not make the reply part of our type system (R-BE-25).
    let output: AssessmentOutput = serde_json::from_str(&reply).map_err(|err| {
        Error::invalid(format!(
            "the assessment came back in an unexpected shape: {err}"
        ))
    })?;
    let output = clean(output);

    let assessed = state.with_db_mut(|db| {
        items::apply_assessment(
            db.conn_mut(),
            Some(&root),
            &item_id,
            &output,
            config.thresholds,
        )
    })?;

    if let Ok(payload) = serde_json::to_value(&assessed) {
        state.publish(Event::new(EventName::ItemUpdated, payload));
    }

    Ok(serde_json::json!({
        "item_id": item_id,
        "status": assessed.status.as_str(),
        "families": assessed.families.len(),
    }))
}

/// Mark the item behind an exhausted job as failed (R-BE-17).
///
/// The screenshot and the row survive: only the enrichment is missing, and re-assess is
/// one click. `set_status` deliberately does not touch `last_edited_by` — a failure must
/// not claim the AI edited an item the user last touched (Inventory §10.12).
pub fn mark_failed(worker: &Worker, job: &Job, message: &str) {
    let Some(item_id) = job
        .payload
        .get("item_id")
        .and_then(serde_json::Value::as_str)
    else {
        return;
    };

    let state = worker.state();
    let root = state.data_root().to_path_buf();
    let updated = state.with_db_mut(|db| {
        items::set_status(
            db.conn_mut(),
            Some(&root),
            item_id,
            ItemStatus::AssessmentFailed,
            Some(message),
        )
    });

    match updated {
        Ok(item) => {
            if let Ok(payload) = serde_json::to_value(&item) {
                state.publish(Event::new(EventName::ItemUpdated, payload));
            }
        }
        Err(err) => tracing::warn!(%err, item = item_id, "could not mark the item failed"),
    }
}

/// The screenshot, downscaled for the model (R-BE-26).
fn payload_for(root: &std::path::Path, item: &Item) -> curio_core::Result<curio_core::ai::Image> {
    let path = root.join(&item.screenshot_path);
    let bytes = std::fs::read(&path).map_err(|err| {
        // A missing screenshot will still be missing on retry, so this must not read as a
        // transient model failure.
        Error::invalid(format!(
            "the screenshot for this item could not be read: {err}"
        ))
    })?;

    let encoded = crate::images::vision_payload(&bytes);
    if !encoded.processed {
        // R-BE-26's degrade rule fired. Worth a line: it means every assessment of this
        // item is paying full-resolution image tokens.
        tracing::info!(item = %item.id, "sending the screenshot at full resolution");
    }

    let media_type = if encoded.processed {
        encoded.media_type.to_owned()
    } else {
        crate::images::sniff_media_type(&encoded.bytes).to_owned()
    };

    use base64::Engine as _;
    Ok(curio_core::ai::Image {
        media_type,
        base64: base64::engine::general_purpose::STANDARD.encode(&encoded.bytes),
    })
}

/// The user's rubric, or the reason there isn't one.
///
/// Seeded once at boot and never overwritten (R-BE-29) — a user is meant to edit it. A
/// read failure degrades to a one-line instruction rather than failing the job: an
/// assessment against a weak rubric is worth more than no assessment at all, and the
/// missing file is visible in the data root.
fn read_rubric(root: &std::path::Path) -> String {
    let path = root.join(curio_core::paths::SKILL_FILE_RELATIVE);
    match std::fs::read_to_string(&path) {
        Ok(rubric) if !rubric.trim().is_empty() => rubric,
        _ => {
            tracing::warn!(path = %path.display(), "the assessment rubric is missing");
            "You are assessing a screenshot of a web page for a designer's reference \
             library. Describe it precisely and score it against the supplied vocabulary."
                .to_owned()
        }
    }
}

/// Tidy what the model produced before it becomes rows (Inventory §9).
///
/// A model occasionally answers a request for a tag with a sentence. A sentence in the tag
/// filter is permanent clutter every future filter list has to carry, so it is dropped
/// here rather than stored and regretted.
fn clean(mut output: AssessmentOutput) -> AssessmentOutput {
    use curio_core::assessment::clean_vocabulary_names;

    output.tags = clean_vocabulary_names(&output.tags);
    output.design_types = clean_vocabulary_names(&output.design_types);
    output.name_suggestion = output.name_suggestion.trim().to_owned();
    output.short_description = output.short_description.trim().to_owned();
    output.image_recipe = output
        .image_recipe
        .map(|recipe| recipe.trim().to_owned())
        .filter(|recipe| !recipe.is_empty());

    // A proposal with no name cannot become a family; one with no description becomes a
    // family nobody can score against later.
    output.new_family_proposal = output.new_family_proposal.filter(|proposal| {
        !proposal.name.trim().is_empty() && !proposal.description.trim().is_empty()
    });

    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use curio_core::assessment::{FamilyScore, NewFamilyProposal};

    fn output() -> AssessmentOutput {
        AssessmentOutput {
            name_suggestion: "  Stripe pricing  ".to_owned(),
            short_description: " A clean pricing table ".to_owned(),
            design_types: vec!["pricing page".to_owned(), "Pricing Page".to_owned()],
            tags: vec!["saas".to_owned(), "   ".to_owned(), "x".repeat(80)],
            family_scores: vec![FamilyScore {
                family: "Minimal".to_owned(),
                score: 0.8,
            }],
            new_family_proposal: None,
            image_recipe: Some("   ".to_owned()),
        }
    }

    #[test]
    fn duplicate_and_oversized_vocabulary_is_dropped_before_it_becomes_rows() {
        // `tags.name` is UNIQUE COLLATE NOCASE, so the duplicate is a write failure rather
        // than a second tag — and an 80-character "tag" is clutter in every future filter.
        let cleaned = clean(output());

        assert_eq!(cleaned.design_types, vec!["pricing page"]);
        assert_eq!(cleaned.tags, vec!["saas"]);
    }

    #[test]
    fn whitespace_around_the_name_never_reaches_the_grid() {
        let cleaned = clean(output());

        assert_eq!(cleaned.name_suggestion, "Stripe pricing");
        assert_eq!(cleaned.short_description, "A clean pricing table");
    }

    #[test]
    fn a_blank_image_recipe_is_absent_rather_than_empty() {
        // The sidecar and the UI both branch on presence; an empty string renders as a
        // recipe section containing nothing.
        assert!(clean(output()).image_recipe.is_none());
    }

    #[test]
    fn a_proposal_missing_its_description_is_discarded() {
        // The description becomes the family's own, and it is what every later assessment
        // scores against. An empty one produces a family nothing can match.
        let mut raw = output();
        raw.new_family_proposal = Some(NewFamilyProposal {
            name: "Warm Editorial".to_owned(),
            description: "  ".to_owned(),
        });

        assert!(clean(raw).new_family_proposal.is_none());
    }

    #[test]
    fn a_complete_proposal_survives() {
        let mut raw = output();
        raw.new_family_proposal = Some(NewFamilyProposal {
            name: "Warm Editorial".to_owned(),
            description: "Serif headlines on paper-warm neutrals.".to_owned(),
        });

        assert!(clean(raw).new_family_proposal.is_some());
    }

    #[test]
    fn a_missing_rubric_degrades_to_an_instruction_rather_than_an_empty_prompt() {
        // An assessment against a weak rubric beats no assessment; the missing file is
        // visible in the data root and logged.
        let rubric = read_rubric(std::path::Path::new("a directory that is not there"));

        assert!(!rubric.trim().is_empty());
        assert!(rubric.contains("screenshot"));
    }
}
