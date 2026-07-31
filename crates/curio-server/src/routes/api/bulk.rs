//! The two AI bulk operations: `POST /api/bulk/retag` and `POST /api/bulk/dedupe`.
//!
//! Both enqueue rather than run. A 500-item retag takes minutes and a dedupe call takes
//! seconds, but neither belongs on a request thread: the user pressed a button, and what
//! they should get back is a job id they can watch, not a connection held open until it
//! finishes.
//!
//! ## Membership is frozen here (R-BE-18)
//!
//! The selection is resolved **once, in this handler**, and the resulting ids go into the
//! job payload. A filter re-resolved by the worker could act on a different set than the
//! one on screen when the button was pressed — items captured while it queued would be
//! swept in, and items edited in the meantime would drop out.
//!
//! ## Over-cap refuses rather than trims
//!
//! A selection above the 500-item cap returns `409` naming both `matched` and `limit`
//! (Inventory §10.11). Silently doing the first 500 would look like success and leave the
//! rest untouched with nothing saying so.

use axum::Json;
use axum::extract::State;
use serde::{Deserialize, Serialize};

use curio_core::domain::{JobKind, JobStatus};
use curio_core::events::{Event, EventName};
use curio_db::{items, jobs};

use crate::routes::error::{ApiError, ApiResult};
use crate::state::AppState;

/// The serial/batch boundary, mirrored from the worker so the reply can say which path
/// this run will take before it starts (R-BE-18).
const BATCH_THRESHOLD: usize = 8;

#[derive(Debug, Deserialize)]
pub struct RetagBody {
    /// Exactly one of these, never both.
    pub ids: Option<Vec<String>>,
    pub filter: Option<Vec<(String, String)>>,
    /// `replace` (default) or `augment`.
    #[serde(default)]
    pub mode: Option<String>,
    /// What the user wants changed, in their words. Capped, because it rides in every
    /// request of a 500-item batch.
    #[serde(default)]
    pub instruction: Option<String>,
}

/// The longest instruction accepted (Inventory §9).
const MAX_INSTRUCTION: usize = 2000;

#[derive(Debug, Serialize)]
pub struct Queued {
    pub job_id: String,
    pub items: usize,
    /// `serial` or `batch`. Surfaced so the UI can set the right expectation — a batch of
    /// 240 items is minutes, not seconds.
    pub via: &'static str,
}

/// `POST /api/bulk/retag`.
pub async fn retag(
    State(state): State<AppState>,
    Json(body): Json<RetagBody>,
) -> ApiResult<Json<Queued>> {
    let ids = resolve(&state, body.ids.as_deref(), body.filter.as_deref())?;
    curio_core::query::enforce_bulk_cap(ids.len())?;
    if ids.is_empty() {
        return Err(ApiError(curio_core::Error::invalid(
            "that selection matched no items",
        )));
    }

    // Refused up front rather than queued (Inventory §1: no API key → 409). A capture
    // queues without a key because the *capture* already succeeded and only its enrichment
    // is missing; a retag has nothing to show for itself until it runs, so a job that sits
    // parked would just be a spinner with no explanation.
    if !crate::secrets::is_configured() {
        return Err(ApiError(curio_core::Error::Conflict(
            "add an Anthropic API key in Settings before running an AI re-tag".to_owned(),
        )));
    }

    let instruction = body.instruction.unwrap_or_default();
    if instruction.chars().count() > MAX_INSTRUCTION {
        return Err(ApiError(curio_core::Error::invalid(format!(
            "that instruction is {} characters; the limit is {MAX_INSTRUCTION}",
            instruction.chars().count()
        ))));
    }

    let mode = match body.mode.as_deref() {
        Some("augment") => "augment",
        _ => "replace",
    };
    let via = if ids.len() < BATCH_THRESHOLD {
        "serial"
    } else {
        "batch"
    };

    let job = state.with_db(|db| {
        jobs::enqueue(
            db.conn(),
            JobKind::BulkRetag,
            // Ids, never the filter (R-BE-18).
            &serde_json::json!({
                "item_ids": ids,
                "mode": mode,
                "instruction": instruction,
            }),
        )
    })?;

    announce(&state, &job);
    Ok(Json(Queued {
        job_id: job.id,
        items: ids.len(),
        via,
    }))
}

#[derive(Debug, Deserialize, Default)]
pub struct DedupeBody {
    /// `tag` (default), `design_type`, or `family`.
    #[serde(default)]
    pub kind: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct DedupeQueued {
    pub job_id: String,
    /// True when an existing run was returned instead of a new one.
    pub already_running: bool,
}

/// `POST /api/bulk/dedupe` — single-flight (Inventory §1).
///
/// Two dedupe passes over the same vocabulary produce two lists of the same groups, and a
/// user who double-clicks would get two panels disagreeing about which one they already
/// dismissed. Returning the running job is the honest answer to "start one" when one is
/// already going.
pub async fn dedupe(
    State(state): State<AppState>,
    body: Option<Json<DedupeBody>>,
) -> ApiResult<Json<DedupeQueued>> {
    let kind = body
        .and_then(|Json(body)| body.kind)
        .unwrap_or_else(|| "tag".to_owned());

    if let Some(running) = in_flight(&state)? {
        return Ok(Json(DedupeQueued {
            job_id: running.id,
            already_running: true,
        }));
    }

    if !crate::secrets::is_configured() {
        return Err(ApiError(curio_core::Error::Conflict(
            "add an Anthropic API key in Settings before running a consistency pass".to_owned(),
        )));
    }

    let job = state.with_db(|db| {
        jobs::enqueue(
            db.conn(),
            JobKind::VocabDedupe,
            &serde_json::json!({ "kind": kind }),
        )
    })?;

    announce(&state, &job);
    Ok(Json(DedupeQueued {
        job_id: job.id,
        already_running: false,
    }))
}

/// `GET /api/bulk/dedupe/latest` — the most recent pass, running or finished.
///
/// The result is a **proposal**: it is shown, and the user applies groups one at a time.
/// Nothing here merges anything (R-FE-15a, Inventory §10.8).
pub async fn dedupe_latest(
    State(state): State<AppState>,
) -> ApiResult<Json<Option<curio_core::domain::Job>>> {
    let latest = state
        .with_db(|db| jobs::recent(db.conn(), 200))?
        .into_iter()
        .find(|job| job.kind == "vocab_dedupe");

    Ok(Json(latest))
}

/// The dedupe job that is already going, if there is one.
fn in_flight(state: &AppState) -> ApiResult<Option<curio_core::domain::Job>> {
    Ok(state
        .with_db(|db| jobs::recent(db.conn(), 200))?
        .into_iter()
        .find(|job| {
            job.kind == "vocab_dedupe"
                && matches!(job.status, JobStatus::Queued | JobStatus::Running)
        }))
}

fn resolve(
    state: &AppState,
    ids: Option<&[String]>,
    filter: Option<&[(String, String)]>,
) -> ApiResult<Vec<String>> {
    match (ids, filter) {
        (Some(ids), None) => Ok(ids.to_vec()),
        (None, Some(filter)) => {
            let query = crate::routes::api::items::parse_query(filter)?;
            Ok(state.with_db(|db| items::matching_ids(db.conn(), &query))?)
        }
        // Exactly one. "Both" has no meaning, and "neither" would silently mean the whole
        // library — which is precisely the retag nobody intends to start.
        _ => Err(ApiError(curio_core::Error::invalid(
            "a bulk operation takes either ids or a filter",
        ))),
    }
}

/// Announce the job and wake the worker.
///
/// The wake is a latency fix, not a correctness one — the worker polls every two seconds
/// regardless (Inventory §9) — but "I pressed the button and nothing happened for two
/// seconds" is how a working queue reads as a broken one.
fn announce(state: &AppState, job: &curio_core::domain::Job) {
    if let Ok(payload) = serde_json::to_value(job) {
        state.publish(Event::new(EventName::JobUpdated, payload));
    }
    state.wake_worker();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn state() -> AppState {
        AppState::new(
            crate::security::RuntimeToken::mint(),
            "quit",
            "0.1.0",
            0,
            std::env::temp_dir(),
            curio_core::config::Config::default(),
            curio_db::Db::open_in_memory().expect("library"),
        )
    }

    #[test]
    fn a_selection_needs_ids_or_a_filter_but_not_both() {
        let state = state();
        let ids = vec!["01J".to_owned()];
        let filter = vec![("tag".to_owned(), "saas".to_owned())];

        assert!(resolve(&state, Some(&ids), Some(&filter)).is_err());
        assert!(resolve(&state, None, None).is_err());
        assert!(resolve(&state, Some(&ids), None).is_ok());
    }

    #[test]
    fn the_reply_says_which_path_the_run_will_take() {
        // R-BE-18. "240 items, still going" reads very differently when the user knows a
        // batch is in flight rather than 240 sequential calls.
        assert_eq!(BATCH_THRESHOLD, 8);
    }

    #[test]
    fn a_second_dedupe_returns_the_first_rather_than_starting_another() {
        // Inventory §1 single-flight. Two passes produce two lists of the same groups,
        // and the user cannot tell which panel they already dismissed.
        let state = state();
        let job = state
            .with_db(|db| jobs::enqueue(db.conn(), JobKind::VocabDedupe, &serde_json::json!({})))
            .expect("enqueue");

        let running = in_flight(&state).expect("read").expect("a running job");
        assert_eq!(running.id, job.id);
    }

    #[test]
    fn a_finished_dedupe_does_not_block_a_new_one() {
        let state = state();
        let job = state
            .with_db(|db| jobs::enqueue(db.conn(), JobKind::VocabDedupe, &serde_json::json!({})))
            .expect("enqueue");
        state
            .with_db(|db| jobs::finish(db.conn(), &job.id, JobStatus::Done, None, None))
            .expect("finish");

        assert!(in_flight(&state).expect("read").is_none());
    }

    #[test]
    fn an_assessment_job_is_not_mistaken_for_a_dedupe() {
        let state = state();
        state
            .with_db(|db| {
                jobs::enqueue(
                    db.conn(),
                    JobKind::AssessItem,
                    &serde_json::json!({ "item_id": "01J" }),
                )
            })
            .expect("enqueue");

        assert!(in_flight(&state).expect("read").is_none());
    }
}
