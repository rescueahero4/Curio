//! `bulk_retag`: re-tag a frozen set of items, serially or through the Batch API.
//!
//! ## Membership is frozen at enqueue
//!
//! The payload holds **ids, never a filter** (R-BE-18). Re-resolving a filter at run time
//! would let the set change between the moment the user pressed the button and the moment
//! the worker got to it — so a retag started on "everything untagged" could quietly
//! include items captured while it queued, or miss ones that were tagged in between.
//!
//! ## Two paths, one boundary
//!
//! Under eight items runs serially and resumes from `progress.done`, so a crash costs at
//! most one item. Eight or more goes to the Batch API at half price, with `custom_id` set
//! to the item id — results come back in **any order**, and keying by position would
//! attach one item's tags to another's row.
//!
//! A batch-backed job **parks** between polls rather than sleeping (R-BE-17). A batch can
//! take an hour; blocking the single worker on it would stall every capture behind it.

use std::collections::HashMap;

use curio_core::Error;
use curio_core::domain::Job;
use curio_db::items;

mod apply;

use apply::{apply, finish_run, summary};

use super::Worker;

/// The serial/batch boundary (R-BE-18).
const BATCH_THRESHOLD: usize = 8;

/// How often a parked batch job looks again (R-BE-18).
const BATCH_POLL_SECONDS: i64 = 5;

/// How often progress reaches the UI (R-BE-18).
const PROGRESS_EVERY: usize = 10;

/// What the job is doing to each item's vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Mode {
    /// The model's answer becomes the item's whole vocabulary.
    Replace,
    /// The model's answer is added to what is already there.
    Augment,
}

/// Run one bulk retag.
pub async fn run(worker: &Worker, job: &Job) -> curio_core::Result<serde_json::Value> {
    let ids: Vec<String> = job
        .payload
        .get("item_ids")
        .and_then(serde_json::Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(|value| value.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();

    if ids.is_empty() {
        return Err(Error::invalid("this retag job names no items"));
    }

    let mode = match job.payload.get("mode").and_then(serde_json::Value::as_str) {
        Some("augment") => Mode::Augment,
        _ => Mode::Replace,
    };
    let instruction = job
        .payload
        .get("instruction")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_owned();

    let api_key = crate::secrets::api_key().ok_or(Error::MissingApiKey)?;
    let client = crate::ai::Anthropic::new(api_key)?;

    if ids.len() < BATCH_THRESHOLD {
        serial(worker, job, &client, &ids, mode, &instruction).await
    } else {
        batched(worker, job, &client, &ids, mode, &instruction).await
    }
}

/// Progress, as `jobs.result` carries it (R-BE-18).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
struct Progress {
    total: usize,
    done: usize,
    changed: usize,
    failed: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    batch_id: Option<String>,
    /// `serial` or `batch`. The UI says which, because "240 items, still waiting" reads
    /// very differently when the user knows a batch is in flight.
    via: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    note: Option<String>,
}

impl Progress {
    fn read(job: &Job, total: usize, via: &str) -> Self {
        job.result
            .as_ref()
            .and_then(|raw| serde_json::from_value::<Self>(raw.clone()).ok())
            .map_or(
                Self {
                    total,
                    via: via.to_owned(),
                    ..Self::default()
                },
                |mut stored| {
                    stored.total = total;
                    stored.via = via.to_owned();
                    stored
                },
            )
    }

    fn value(&self) -> serde_json::Value {
        serde_json::to_value(self).unwrap_or(serde_json::Value::Null)
    }
}

/// Fewer than eight: one call per item, resumable.
async fn serial(
    worker: &Worker,
    job: &Job,
    client: &crate::ai::Anthropic,
    ids: &[String],
    mode: Mode,
    instruction: &str,
) -> curio_core::Result<serde_json::Value> {
    let mut progress = Progress::read(job, ids.len(), "serial");
    let state = worker.state();
    let config = state.config();
    let vocabulary = super::assess_vocabulary(state)?;

    // Resume where the last run stopped rather than re-spending on items already done.
    for id in ids.iter().skip(progress.done) {
        if worker.should_stop(&job.id) {
            progress.note = Some("stopped before finishing".to_owned());
            return Ok(progress.value());
        }

        let Some(item) = state.with_db(|db| items::get(db.conn(), id))? else {
            // Deleted between enqueue and now. Skipping is right — failing the whole run
            // would punish the rest of the set for one stale id.
            progress.done += 1;
            continue;
        };

        let request = curio_core::ai::prompt::retag(
            &config.models.utility,
            &vocabulary,
            instruction,
            &summary(&item),
        );

        match client.messages(&request).await {
            Ok(reply) => match apply(worker, &item, mode, &reply) {
                Ok(true) => progress.changed += 1,
                Ok(false) => {}
                Err(err) => {
                    tracing::warn!(%err, item = %id, "could not apply a retag");
                    progress.failed += 1;
                }
            },
            Err(err) => {
                // One item's failure is not the run's. The count is reported, and the
                // user can re-run the job over what did not take.
                tracing::warn!(%err, item = %id, "retag call failed");
                progress.failed += 1;
            }
        }

        progress.done += 1;
        if progress.done.is_multiple_of(PROGRESS_EVERY) {
            worker.report(&job.id, &progress.value());
        }
    }

    finish_run(worker, mode);
    Ok(progress.value())
}

/// Eight or more: the Batch API, polled from a parked job.
async fn batched(
    worker: &Worker,
    job: &Job,
    client: &crate::ai::Anthropic,
    ids: &[String],
    mode: Mode,
    instruction: &str,
) -> curio_core::Result<serde_json::Value> {
    let mut progress = Progress::read(job, ids.len(), "batch");
    let state = worker.state();

    let Some(batch_id) = progress.batch_id.clone() else {
        // First run: submit and park. Nothing is applied yet.
        let submitted = submit(state, client, ids, instruction).await?;
        progress.batch_id = Some(submitted);
        progress.note = Some("submitted; waiting for the batch to run".to_owned());
        worker.report(&job.id, &progress.value());

        return Err(Error::Parked {
            retry_after: curio_core::time::seconds_from_now(BATCH_POLL_SECONDS),
        });
    };

    // R-BE-19: cancelling a batch-backed job must also cancel the batch, or it keeps
    // running and billing after the user believes they stopped it.
    if worker.should_stop(&job.id) {
        if let Err(err) = client.cancel_batch(&batch_id).await {
            tracing::warn!(%err, batch = %batch_id, "could not cancel the batch upstream");
        }
        progress.note = Some("cancelled".to_owned());
        return Ok(progress.value());
    }

    if !client.batch_status(&batch_id).await?.is_finished() {
        return Err(Error::Parked {
            retry_after: curio_core::time::seconds_from_now(BATCH_POLL_SECONDS),
        });
    }

    let results = client.batch_results(&batch_id).await?;
    apply_batch(worker, job, ids, mode, &results, &mut progress);
    finish_run(worker, mode);

    progress.note = None;
    Ok(progress.value())
}

async fn submit(
    state: &crate::AppState,
    client: &crate::ai::Anthropic,
    ids: &[String],
    instruction: &str,
) -> curio_core::Result<String> {
    let config = state.config();
    let vocabulary = super::assess_vocabulary(state)?;

    let mut requests = Vec::with_capacity(ids.len());
    for id in ids {
        let Some(item) = state.with_db(|db| items::get(db.conn(), id))? else {
            continue;
        };
        requests.push((
            // `custom_id` is the item id, all the way back. Results arrive in any order.
            id.clone(),
            curio_core::ai::prompt::retag(
                &config.models.utility,
                &vocabulary,
                instruction,
                &summary(&item),
            ),
        ));
    }

    if requests.is_empty() {
        return Err(Error::invalid(
            "every item in this retag has been deleted since it was queued",
        ));
    }
    client.create_batch(&requests).await
}

fn apply_batch(
    worker: &Worker,
    job: &Job,
    ids: &[String],
    mode: Mode,
    results: &HashMap<String, Result<String, String>>,
    progress: &mut Progress,
) {
    let state = worker.state();

    for (index, id) in ids.iter().enumerate() {
        if worker.is_stopping() {
            progress.note = Some("stopped before applying every result".to_owned());
            return;
        }

        match results.get(id) {
            Some(Ok(reply)) => {
                // `None` means the item was deleted while the batch ran; there is nothing
                // to apply the result to and nothing went wrong.
                if let Some(item) = state.with_db(|db| items::get(db.conn(), id)).ok().flatten() {
                    match apply(worker, &item, mode, reply) {
                        Ok(true) => progress.changed += 1,
                        Ok(false) => {}
                        Err(err) => {
                            tracing::warn!(%err, item = %id, "could not apply a batched retag");
                            progress.failed += 1;
                        }
                    }
                }
            }
            Some(Err(reason)) => {
                tracing::warn!(item = %id, reason, "a batched retag failed");
                progress.failed += 1;
            }
            // A result the batch never produced. Counted, not silently dropped — the
            // difference between "240 done" and "238 done, 2 failed" is the difference
            // between trusting the number and not.
            None => progress.failed += 1,
        }

        progress.done += 1;
        if (index + 1).is_multiple_of(PROGRESS_EVERY) {
            worker.report(&job.id, &progress.value());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_batch_boundary_is_eight() {
        // R-BE-18. Seven serially, eight through the Batch API at half price.
        assert_eq!(BATCH_THRESHOLD, 8);
    }

    #[test]
    fn progress_resumes_from_what_was_recorded() {
        // A crash mid-run must cost at most one item, not the whole spend again.
        let job = Job {
            id: "01J".to_owned(),
            kind: "bulk_retag".to_owned(),
            payload: serde_json::Value::Null,
            status: curio_core::domain::JobStatus::Running,
            attempts: 0,
            error: None,
            result: Some(
                serde_json::json!({ "total": 5, "done": 3, "changed": 2, "failed": 0, "via": "serial" }),
            ),
            not_before: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let progress = Progress::read(&job, 5, "serial");
        assert_eq!(progress.done, 3);
        assert_eq!(progress.changed, 2);
    }

    #[test]
    fn a_first_run_starts_from_nothing() {
        let job = Job {
            id: "01J".to_owned(),
            kind: "bulk_retag".to_owned(),
            payload: serde_json::Value::Null,
            status: curio_core::domain::JobStatus::Running,
            attempts: 0,
            error: None,
            result: None,
            not_before: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let progress = Progress::read(&job, 40, "batch");
        assert_eq!(progress.done, 0);
        assert_eq!(progress.total, 40);
        assert!(progress.batch_id.is_none());
    }
}
