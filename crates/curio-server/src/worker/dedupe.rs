//! `vocab_dedupe`: which vocabulary entries mean the same thing.
//!
//! The result is **stored and shown, never auto-applied** (R-FE-15a, Inventory §10.8). A
//! merge rewrites every item that used either name and cannot be undone by pressing the
//! button again, so the job's output is a proposal and the apply is a separate, per-group
//! action the user takes.
//!
//! One utility call per job (R-BE-24): `max_tokens 2000`, a single cached system block,
//! and **no `effort` parameter** — the cheap model rejects it outright.

use curio_core::Error;
use curio_core::domain::{Job, VocabularyKind};
use curio_db::vocabulary;

use super::Worker;

/// Run one dedupe pass.
pub async fn run(worker: &Worker, job: &Job) -> curio_core::Result<serde_json::Value> {
    // The job names which vocabulary to tidy. Tags are the default because they are what
    // AI generation actually floods — a `vocab_dedupe {}` from the previous
    // implementation's payload shape still means "tags" and keeps working.
    let kind = match job
        .payload
        .get("kind")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("tag")
    {
        "design_type" => VocabularyKind::DesignType,
        "family" => VocabularyKind::Family,
        _ => VocabularyKind::Tag,
    };

    let state = worker.state();
    let names = read_names(state, kind)?;

    // Nothing to compare. A call here would spend a request to be told what we already
    // know, and would occasionally invent a group out of two entries.
    if names.len() < 2 {
        return Ok(serde_json::json!({
            "kind": kind.as_str(),
            "groups": [],
            "note": "there is not enough vocabulary to find duplicates in yet",
        }));
    }

    let api_key = crate::secrets::api_key().ok_or(Error::MissingApiKey)?;
    if worker.should_stop(&job.id) {
        return Ok(serde_json::json!({ "cancelled": true }));
    }

    let config = state.config();
    let reply = crate::ai::Anthropic::new(api_key)?
        .messages(&curio_core::ai::prompt::dedupe(
            &config.models.utility,
            plural(kind),
            &names,
        ))
        .await?;

    let output: curio_core::ai::dedupe::Output = serde_json::from_str(&reply)
        .map_err(|err| Error::invalid(format!("the dedupe reply was malformed: {err}")))?;

    // Everything unusable is removed here rather than at apply time — in front of the
    // user, after they clicked (Inventory §10.8).
    let groups = curio_core::ai::dedupe::sanitize(&output, &names);

    Ok(serde_json::json!({
        "kind": kind.as_str(),
        "groups": groups,
        "considered": names.len(),
    }))
}

fn read_names(state: &crate::AppState, kind: VocabularyKind) -> curio_core::Result<Vec<String>> {
    state.with_db(|db| match kind {
        VocabularyKind::Family => Ok(vocabulary::list_families(db.conn())?
            .into_iter()
            .map(|family| family.name)
            .collect()),
        other => Ok(vocabulary::list_terms(db.conn(), other)?
            .into_iter()
            .map(|term| term.name)
            .collect()),
    })
}

fn plural(kind: VocabularyKind) -> &'static str {
    match kind {
        VocabularyKind::Tag => "tags",
        VocabularyKind::DesignType => "design types",
        VocabularyKind::Family => "aesthetic families",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_has_a_plural_a_prompt_can_use() {
        // The word lands in the middle of a sentence the model reads; a missing case
        // would produce "tidying a  library's" and a worse answer.
        for kind in [
            VocabularyKind::Tag,
            VocabularyKind::DesignType,
            VocabularyKind::Family,
        ] {
            assert!(!plural(kind).is_empty());
        }
    }

    #[test]
    fn an_absent_kind_still_means_tags() {
        // The previous implementation enqueued `vocab_dedupe {}`. A row written by it must
        // not fail on this build (Inventory §9).
        let payload = serde_json::json!({});
        let resolved = payload
            .get("kind")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("tag");

        assert_eq!(resolved, "tag");
    }
}
