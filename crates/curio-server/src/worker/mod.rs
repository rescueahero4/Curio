//! The jobs worker: one loop, one claim, one job at a time (R-BE-16).
//!
//! ## Why one worker and not a pool
//!
//! The database has exactly one write connection (R-DA-8), and every job ends in a write.
//! A pool would spend its time queueing on that mutex while making the ordering guarantees
//! — FIFO, tie-broken on ULID — much harder to reason about. The claim query is written to
//! be safe for a second worker anyway (`UPDATE … WHERE id = (SELECT …)` in one statement),
//! so adding one later is a change of `spawn` count rather than a rewrite.
//!
//! ## Waking
//!
//! Notify **and** a two-second poll (Inventory §9). The notify makes a capture start
//! assessing immediately; the poll is what makes the loop correct, because `Notify` has no
//! backlog and a job enqueued in the window between "claim returned None" and "await
//! notified" would otherwise sit until the next unrelated capture.
//!
//! It is also how a **parked** job resumes: nothing notifies when a `not_before` comes
//! due, so something has to look.
//!
//! ## Cancellation
//!
//! Polled at every boundary (R-BE-19), by re-reading the row rather than holding a flag.
//! The cancel route writes `cancelled` to the database, and the boundary check is the only
//! thing standing between that and the worker finishing the job as `done` a moment later —
//! overwriting the user's decision with a result they asked not to have.

mod assess;
mod dedupe;
mod retag;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use curio_core::ai::{Recovery, recover};
use curio_core::domain::{Job, JobStatus};
use curio_core::events::{Event, EventName};
use curio_db::jobs;
use tokio::sync::oneshot;

use crate::state::AppState;

/// How often the loop looks for work nobody announced (Inventory §9).
///
/// Also the resolution of a park timer: a job parked for 30 s resumes within 2 s of coming
/// due, which is close enough that a user pasting an API key sees the queue move.
const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// The worker's handle on the world.
#[derive(Clone)]
pub struct Worker {
    state: AppState,
    /// Raised at shutdown. Read at job boundaries so a long bulk run stops between items
    /// rather than being abandoned mid-write.
    stopping: Arc<AtomicBool>,
}

impl Worker {
    #[must_use]
    pub fn new(state: AppState) -> Self {
        Self {
            state,
            stopping: Arc::new(AtomicBool::new(false)),
        }
    }

    /// The shared state, for handlers.
    pub(crate) fn state(&self) -> &AppState {
        &self.state
    }

    /// Whether the process is shutting down. Checked at every job boundary.
    pub(crate) fn is_stopping(&self) -> bool {
        self.stopping.load(Ordering::Relaxed)
    }

    /// Whether this job should stop what it is doing.
    ///
    /// True when the user cancelled it **or** the process is quitting. Handlers call this
    /// between items and between polls; anything that loops without calling it is a job
    /// the Cancel button cannot stop.
    pub(crate) fn should_stop(&self, job_id: &str) -> bool {
        if self.is_stopping() {
            return true;
        }
        self.state
            .with_db(|db| jobs::get(db.conn(), job_id))
            .ok()
            .flatten()
            .is_some_and(|job| job.status == JobStatus::Cancelled)
    }

    /// Publish progress without moving the job's state (Inventory §10.30).
    pub(crate) fn report(&self, job_id: &str, result: &serde_json::Value) {
        if let Err(err) = self
            .state
            .with_db(|db| jobs::set_result(db.conn(), job_id, result))
        {
            tracing::warn!(%err, job = job_id, "could not record job progress");
            return;
        }
        // A partial, merged by id on the client (Inventory §10.30). Sending the whole job
        // on every tenth item of a 500-item retag would be 50 full payloads for one
        // changing number.
        self.state.publish(Event::new(
            EventName::JobUpdated,
            serde_json::json!({ "id": job_id, "result": result }),
        ));
    }

    fn announce(&self, job: &Job) {
        if let Ok(payload) = serde_json::to_value(job) {
            self.state
                .publish(Event::new(EventName::JobUpdated, payload));
        }
    }
}

/// The library's vocabulary, in the stable order the cached prompt block needs.
///
/// Shared by assessment and retag because both cache it, and two functions building the
/// same block in two orders would mean neither ever hits the cache.
///
/// The ordering comes from the database's `ORDER BY name COLLATE NOCASE`. Re-sorting here
/// would be a second source of truth for it, and the two drifting apart would silently
/// invalidate the breakpoint on every call.
pub(crate) fn assess_vocabulary(
    state: &AppState,
) -> curio_core::Result<curio_core::ai::Vocabulary> {
    use curio_core::domain::VocabularyKind;
    use curio_db::vocabulary;

    state.with_db(|db| {
        let families = vocabulary::list_families(db.conn())?
            .into_iter()
            .map(|family| (family.name, family.description))
            .collect();
        let design_types = vocabulary::list_terms(db.conn(), VocabularyKind::DesignType)?
            .into_iter()
            .map(|term| term.name)
            .collect();
        let tags = vocabulary::list_terms(db.conn(), VocabularyKind::Tag)?
            .into_iter()
            .map(|term| term.name)
            .collect();

        Ok(curio_core::ai::Vocabulary {
            families,
            design_types,
            tags,
        })
    })
}

/// Run until told to stop.
///
/// Drains everything claimable, then waits for a notify, the poll interval, or shutdown.
pub async fn run(state: AppState, mut shutdown: oneshot::Receiver<()>) {
    let worker = Worker::new(state);
    tracing::debug!("jobs worker started");

    loop {
        // Drain rather than take one: a burst of captures should not be spread across one
        // job per two-second tick.
        while !worker.is_stopping() {
            let claimed = worker.state.with_db(|db| jobs::claim_next(db.conn()));
            match claimed {
                Ok(Some(job)) => run_one(&worker, job).await,
                Ok(None) => break,
                Err(err) => {
                    tracing::error!(%err, "could not claim a job");
                    break;
                }
            }
        }

        tokio::select! {
            _ = &mut shutdown => break,
            () = worker.state.job_enqueued() => {}
            () = tokio::time::sleep(POLL_INTERVAL) => {}
        }
    }

    worker.stopping.store(true, Ordering::Relaxed);
    tracing::debug!("jobs worker stopped");
}

/// Handle one claimed job, including everything that can go wrong with it.
async fn run_one(worker: &Worker, job: Job) {
    worker.announce(&job);
    tracing::debug!(job = %job.id, kind = %job.kind, "running");

    let outcome = match job.kind.as_str() {
        "assess_item" => assess::run(worker, &job).await,
        "bulk_retag" => retag::run(worker, &job).await,
        "vocab_dedupe" => dedupe::run(worker, &job).await,
        // A kind this build does not know is a library written by a newer version, or a
        // hand-edited row. Failing it is honest; leaving it queued would make it a
        // permanent resident of the claim query, re-picked every two seconds forever.
        unknown => Err(curio_core::Error::invalid(format!(
            "this version of Curio does not know how to run a {unknown} job"
        ))),
    };

    // The boundary that matters most: the user pressed Cancel while the model was
    // thinking. Finishing here would overwrite their decision with a result they
    // explicitly asked not to have.
    if worker.should_stop(&job.id) && !worker.is_stopping() {
        tracing::debug!(job = %job.id, "cancelled while running; leaving it cancelled");
        if let Ok(Some(current)) = worker.state.with_db(|db| jobs::get(db.conn(), &job.id)) {
            worker.announce(&current);
        }
        return;
    }

    match outcome {
        Ok(result) => settle(worker, &job, JobStatus::Done, None, Some(&result)),
        Err(error) => fail(worker, &job, &error),
    }
}

/// Apply the failure policy (R-BE-17), which decides whether this costs an attempt.
fn fail(worker: &Worker, job: &Job, error: &curio_core::Error) {
    let message = error.to_string();

    match recover(job.attempts, error) {
        // Waiting, not failing. The attempt is refunded and the item stays as it was —
        // `processing`, not `assessment_failed` (FR-26).
        Recovery::Park { seconds } => {
            tracing::debug!(job = %job.id, seconds, reason = %message, "parked");
            let parked = worker.state.with_db(|db| {
                jobs::park(
                    db.conn(),
                    &job.id,
                    &curio_core::time::seconds_from_now(seconds),
                )
            });
            match parked {
                Ok(job) => worker.announce(&job),
                Err(err) => tracing::error!(%err, "could not park a job"),
            }
        }

        Recovery::Retry { seconds } => {
            tracing::info!(job = %job.id, seconds, error = %message, "retrying");
            let requeued = worker.state.with_db(|db| {
                jobs::requeue(
                    db.conn(),
                    &job.id,
                    &curio_core::time::seconds_from_now(seconds),
                    Some(&message),
                )
            });
            match requeued {
                Ok(job) => worker.announce(&job),
                Err(err) => tracing::error!(%err, "could not requeue a job"),
            }
        }

        Recovery::GiveUp => {
            tracing::warn!(job = %job.id, error = %message, "gave up");
            // The item's own failure state is the handler's business — only `assess_item`
            // has an item behind it, and only it knows which one.
            if job.kind == "assess_item" {
                assess::mark_failed(worker, job, &message);
            }
            settle(worker, job, JobStatus::Failed, Some(&message), None);
        }
    }
}

fn settle(
    worker: &Worker,
    job: &Job,
    status: JobStatus,
    error: Option<&str>,
    result: Option<&serde_json::Value>,
) {
    match worker
        .state
        .with_db(|db| jobs::finish(db.conn(), &job.id, status, error, result))
    {
        Ok(finished) => worker.announce(&finished),
        Err(err) => tracing::error!(%err, job = %job.id, "could not finish a job"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curio_core::domain::JobKind;

    fn worker() -> Worker {
        Worker::new(AppState::new(
            crate::security::RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            curio_core::config::Config::default(),
            curio_db::Db::open_in_memory().expect("in-memory library"),
        ))
    }

    #[test]
    fn a_cancelled_job_reports_that_it_should_stop() {
        // R-BE-19. Without this the worker finishes a job the user cancelled, writing a
        // result they explicitly declined.
        let worker = worker();
        let job = worker
            .state
            .with_db(|db| jobs::enqueue(db.conn(), JobKind::BulkRetag, &serde_json::json!({})))
            .expect("enqueue");

        assert!(!worker.should_stop(&job.id));

        worker
            .state
            .with_db(|db| jobs::cancel(db.conn(), &job.id))
            .expect("cancel");

        assert!(worker.should_stop(&job.id));
    }

    #[test]
    fn shutting_down_stops_every_job_not_just_cancelled_ones() {
        let worker = worker();
        worker.stopping.store(true, Ordering::Relaxed);

        assert!(worker.should_stop("a job that does not exist"));
    }

    #[test]
    fn a_vanished_job_does_not_read_as_cancelled() {
        // A job the row for which has gone is a bug, not a cancellation — reporting it as
        // cancelled would silently skip work rather than surfacing the problem.
        assert!(!worker().should_stop("01MISSING"));
    }

    #[test]
    fn progress_is_published_as_a_partial() {
        // Inventory §10.30: clients merge `job.updated` by id. A 500-item retag publishing
        // the whole job every ten items is fifty full payloads for one changing number.
        let worker = worker();
        let job = worker
            .state
            .with_db(|db| jobs::enqueue(db.conn(), JobKind::BulkRetag, &serde_json::json!({})))
            .expect("enqueue");
        let mut events = worker.state.subscribe();

        worker.report(&job.id, &serde_json::json!({ "total": 240, "done": 31 }));

        let event = events.try_recv().expect("an event");
        assert_eq!(event.name, EventName::JobUpdated);
        assert_eq!(event.payload["id"], job.id);
        assert_eq!(event.payload["result"]["done"], 31);
        assert!(
            event.payload.get("payload").is_none(),
            "a progress tick sent the whole job"
        );
    }
}
