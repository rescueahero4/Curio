//! The work queue.
//!
//! One worker, one loop, one claim query (R-BE-16). The claim is "the oldest queued job
//! whose `not_before` is null or due", which is why **parking is not a fourth state**: a
//! parked job is `queued` with a timer, and one query serves ordinary and parked work
//! alike.
//!
//! FIFO ordering rests on monotonic ULIDs (R-DA-5). Jobs enqueued in the same millisecond
//! tie-break on id, and the id is the only thing that can order them — `created_at` is
//! second-precision, so a burst of captures shares one timestamp exactly.

use rusqlite::{Connection, OptionalExtension as _, Row};

use curio_core::domain::{Job, JobKind, JobStatus};

use crate::{Error, Result};

/// Enqueue a job.
///
/// # Errors
/// Propagates a storage failure.
pub fn enqueue(conn: &Connection, kind: JobKind, payload: &serde_json::Value) -> Result<Job> {
    let id = curio_core::ids::generate();
    let now = curio_core::time::now_iso();

    conn.execute(
        "INSERT INTO jobs (id, kind, payload, status, attempts, created_at, updated_at)
           VALUES (?1, ?2, ?3, 'queued', 0, ?4, ?4)",
        rusqlite::params![id, kind.as_str(), payload.to_string(), now],
    )?;
    require(conn, &id)
}

/// Read one job.
///
/// # Errors
/// Propagates a storage failure.
pub fn get(conn: &Connection, id: &str) -> Result<Option<Job>> {
    Ok(conn
        .query_row(
            &format!("SELECT {COLUMNS} FROM jobs WHERE id = ?1"),
            [id],
            map_row,
        )
        .optional()?)
}

/// Read one job or fail.
///
/// # Errors
/// Returns [`Error::NotFound`] if there is no such job.
pub fn require(conn: &Connection, id: &str) -> Result<Job> {
    get(conn, id)?.ok_or_else(|| Error::NotFound {
        kind: "job",
        id: id.to_owned(),
    })
}

/// Recent jobs, newest first.
///
/// # Errors
/// Propagates a storage failure.
pub fn recent(conn: &Connection, limit: usize) -> Result<Vec<Job>> {
    let mut statement = conn.prepare(&format!(
        "SELECT {COLUMNS} FROM jobs ORDER BY created_at DESC, id DESC LIMIT ?1"
    ))?;
    let rows = statement.query_map([i64::try_from(limit).unwrap_or(i64::MAX)], map_row)?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// How many jobs are waiting. Reported by `/health` (R-SEC-11).
///
/// Parked jobs count: from a user's point of view a job waiting on a batch result is still
/// outstanding work, and reporting zero while a 240-item retag is in flight would be a lie
/// the status dot tells.
///
/// # Errors
/// Propagates a storage failure.
pub fn queued_count(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status IN ('queued','running')",
        [],
        |row| row.get(0),
    )?)
}

/// Claim the next job, marking it running.
///
/// The `UPDATE … WHERE id = (SELECT …)` is one statement so the read and the mark cannot be
/// separated. There is only one worker today, but the claim is also what makes a second one
/// safe to add rather than a rewrite.
///
/// # Errors
/// Propagates a storage failure.
pub fn claim_next(conn: &Connection) -> Result<Option<Job>> {
    let now = curio_core::time::now_iso();
    let claimed: Option<String> = conn
        .query_row(
            "UPDATE jobs SET status = 'running', updated_at = ?1
              WHERE id = (SELECT id FROM jobs
                           WHERE status = 'queued' AND (not_before IS NULL OR not_before <= ?1)
                           ORDER BY created_at ASC, id ASC LIMIT 1)
            RETURNING id",
            [&now],
            |row| row.get(0),
        )
        .optional()?;

    match claimed {
        Some(id) => Ok(Some(require(conn, &id)?)),
        None => Ok(None),
    }
}

/// Return orphaned `running` jobs to the queue (R-BE-19).
///
/// Run at startup. A job left `running` by a crash is claimed by nobody and would sit there
/// forever — an item stuck at `processing` with no worker interested in it.
///
/// # Errors
/// Propagates a storage failure.
pub fn reclaim_orphans(conn: &Connection) -> Result<usize> {
    Ok(conn.execute(
        "UPDATE jobs SET status = 'queued', updated_at = ?1 WHERE status = 'running'",
        [curio_core::time::now_iso()],
    )?)
}

/// Publish progress without moving the job's state (Inventory §10.30).
///
/// # Errors
/// Propagates a storage failure.
pub fn set_result(conn: &Connection, id: &str, result: &serde_json::Value) -> Result<()> {
    conn.execute(
        "UPDATE jobs SET result = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, result.to_string(), curio_core::time::now_iso()],
    )?;
    Ok(())
}

/// Move a job to a terminal state, clearing any park timer.
///
/// # Errors
/// Propagates a storage failure.
pub fn finish(
    conn: &Connection,
    id: &str,
    status: JobStatus,
    error: Option<&str>,
    result: Option<&serde_json::Value>,
) -> Result<Job> {
    conn.execute(
        "UPDATE jobs SET status = ?2, error = ?3, not_before = NULL,
                         result = COALESCE(?4, result), updated_at = ?5
           WHERE id = ?1",
        rusqlite::params![
            id,
            status.as_str(),
            error,
            result.map(std::string::ToString::to_string),
            curio_core::time::now_iso()
        ],
    )?;
    require(conn, id)
}

/// Requeue a job after a failure, consuming one attempt.
///
/// # Errors
/// Propagates a storage failure.
pub fn requeue(conn: &Connection, id: &str, retry_after: &str, error: Option<&str>) -> Result<Job> {
    conn.execute(
        "UPDATE jobs SET status = 'queued', attempts = attempts + 1, not_before = ?2,
                         error = ?3, updated_at = ?4
           WHERE id = ?1",
        rusqlite::params![id, retry_after, error, curio_core::time::now_iso()],
    )?;
    require(conn, id)
}

/// Park a job until `retry_after`, **refunding the attempt** (R-BE-17, Inventory §10.10).
///
/// Parking is not a failure. A job waiting on a batch result or a missing API key has not
/// gone wrong — it is waiting — and charging it an attempt would exhaust the retry budget of
/// a user who is merely offline, turning a queue that drains into a pile of failures.
///
/// # Errors
/// Propagates a storage failure.
pub fn park(conn: &Connection, id: &str, retry_after: &str) -> Result<Job> {
    conn.execute(
        "UPDATE jobs SET status = 'queued', not_before = ?2, updated_at = ?3 WHERE id = ?1",
        rusqlite::params![id, retry_after, curio_core::time::now_iso()],
    )?;
    require(conn, id)
}

/// Cancel a job, if it has not already finished.
///
/// # Errors
/// Returns [`Error::NotFound`] for an unknown job, or a storage failure.
pub fn cancel(conn: &Connection, id: &str) -> Result<Job> {
    let job = require(conn, id)?;
    if job.status.is_terminal() {
        // Not an error: the user pressed Cancel on a job that finished while they were
        // reading the screen, and telling them it failed would be worse than telling them
        // nothing happened.
        return Ok(job);
    }
    finish(conn, id, JobStatus::Cancelled, None, None)
}

const COLUMNS: &str =
    "id, kind, payload, status, attempts, error, result, not_before, created_at, updated_at";

fn map_row(row: &Row<'_>) -> rusqlite::Result<Job> {
    let payload: String = row.get(2)?;
    let result: Option<String> = row.get(6)?;
    Ok(Job {
        id: row.get(0)?,
        kind: row.get(1)?,
        // A payload that will not parse is a hand-edited row. Null is honest and keeps the
        // job listable; refusing the read would make the queue page unopenable.
        payload: serde_json::from_str(&payload).unwrap_or(serde_json::Value::Null),
        status: parse_status(&row.get::<_, String>(3)?),
        attempts: row.get(4)?,
        error: row.get(5)?,
        result: result.and_then(|raw| serde_json::from_str(&raw).ok()),
        not_before: row.get(7)?,
        created_at: row.get(8)?,
        updated_at: row.get(9)?,
    })
}

fn parse_status(raw: &str) -> JobStatus {
    JobStatus::all()
        .into_iter()
        .find(|status| status.as_str() == raw)
        .unwrap_or(JobStatus::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Db;

    fn payload(item: &str) -> serde_json::Value {
        serde_json::json!({ "item_id": item })
    }

    #[test]
    fn jobs_are_claimed_oldest_first() {
        let db = Db::open_in_memory().expect("open");
        let first = enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("a");
        enqueue(db.conn(), JobKind::AssessItem, &payload("01B")).expect("b");

        assert_eq!(
            claim_next(db.conn()).expect("claim").map(|j| j.id),
            Some(first.id)
        );
    }

    #[test]
    fn jobs_enqueued_in_the_same_millisecond_still_have_an_order() {
        // `created_at` is second-precision, so a burst shares one timestamp exactly and the
        // ULID is the only thing that can order them (R-DA-5).
        let db = Db::open_in_memory().expect("open");
        let ids: Vec<String> = (0..5)
            .map(|index| {
                enqueue(
                    db.conn(),
                    JobKind::AssessItem,
                    &payload(&format!("{index}")),
                )
                .expect("enqueue")
                .id
            })
            .collect();

        let mut claimed = Vec::new();
        while let Some(job) = claim_next(db.conn()).expect("claim") {
            claimed.push(job.id);
        }

        assert_eq!(claimed, ids);
    }

    #[test]
    fn claiming_marks_the_job_running_so_nobody_takes_it_twice() {
        let db = Db::open_in_memory().expect("open");
        enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("enqueue");

        let claimed = claim_next(db.conn()).expect("claim").expect("a job");
        assert_eq!(claimed.status, JobStatus::Running);
        assert!(claim_next(db.conn()).expect("claim").is_none());
    }

    #[test]
    fn a_parked_job_is_invisible_until_it_is_due() {
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::BulkRetag, &payload("01A")).expect("enqueue");
        park(
            db.conn(),
            &job.id,
            &curio_core::time::seconds_from_now(3600),
        )
        .expect("park");

        assert!(claim_next(db.conn()).expect("claim").is_none());
    }

    #[test]
    fn a_park_that_has_come_due_is_claimable_again() {
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::BulkRetag, &payload("01A")).expect("enqueue");
        park(db.conn(), &job.id, &curio_core::time::seconds_from_now(-1)).expect("park");

        assert_eq!(
            claim_next(db.conn()).expect("claim").map(|j| j.id),
            Some(job.id)
        );
    }

    #[test]
    fn parking_refunds_the_attempt() {
        // R-BE-17 / Inventory §10.10. A user who is merely offline must accumulate a queue
        // that drains, not a pile of failures.
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("enqueue");
        claim_next(db.conn()).expect("claim");

        let parked =
            park(db.conn(), &job.id, &curio_core::time::seconds_from_now(30)).expect("park");

        assert_eq!(parked.attempts, 0, "waiting is not failing");
        assert_eq!(parked.status, JobStatus::Queued);
    }

    #[test]
    fn requeueing_after_a_real_failure_does_consume_an_attempt() {
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("enqueue");

        let requeued = requeue(
            db.conn(),
            &job.id,
            &curio_core::time::seconds_from_now(2),
            Some("model call: 500"),
        )
        .expect("requeue");

        assert_eq!(requeued.attempts, 1);
        assert_eq!(requeued.error.as_deref(), Some("model call: 500"));
    }

    #[test]
    fn finishing_clears_the_park_timer() {
        // Otherwise a `done` job carries a future `not_before` that means nothing and reads
        // like scheduled work whenever anyone looks at the queue.
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::BulkRetag, &payload("01A")).expect("enqueue");
        park(db.conn(), &job.id, &curio_core::time::seconds_from_now(30)).expect("park");

        let done = finish(db.conn(), &job.id, JobStatus::Done, None, None).expect("finish");

        assert!(done.not_before.is_none());
    }

    #[test]
    fn progress_updates_do_not_move_the_state() {
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::BulkRetag, &payload("01A")).expect("enqueue");
        claim_next(db.conn()).expect("claim");

        set_result(
            db.conn(),
            &job.id,
            &serde_json::json!({"total": 240, "done": 31}),
        )
        .expect("progress");

        let running = require(db.conn(), &job.id).expect("read");
        assert_eq!(running.status, JobStatus::Running);
        assert_eq!(running.result.expect("result")["done"], 31);
    }

    #[test]
    fn a_crash_leaves_no_job_stranded() {
        // R-BE-19. A job left `running` is claimed by nobody, and the item behind it sits
        // at `processing` with no worker interested in it.
        let db = Db::open_in_memory().expect("open");
        enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("enqueue");
        claim_next(db.conn()).expect("claim");

        assert_eq!(reclaim_orphans(db.conn()).expect("reclaim"), 1);
        assert!(claim_next(db.conn()).expect("claim").is_some());
    }

    #[test]
    fn cancelling_a_finished_job_is_not_an_error() {
        // The user pressed Cancel on a job that completed while they were reading the
        // screen. Telling them it failed is worse than telling them nothing happened.
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("enqueue");
        finish(db.conn(), &job.id, JobStatus::Done, None, None).expect("finish");

        assert_eq!(
            cancel(db.conn(), &job.id).expect("cancel").status,
            JobStatus::Done
        );
    }

    #[test]
    fn the_queue_count_includes_work_in_flight() {
        let db = Db::open_in_memory().expect("open");
        enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("a");
        enqueue(db.conn(), JobKind::AssessItem, &payload("01B")).expect("b");
        claim_next(db.conn()).expect("claim");

        assert_eq!(queued_count(db.conn()).expect("count"), 2);
    }

    #[test]
    fn a_finished_job_leaves_the_queue_count() {
        let db = Db::open_in_memory().expect("open");
        let job = enqueue(db.conn(), JobKind::AssessItem, &payload("01A")).expect("enqueue");
        finish(db.conn(), &job.id, JobStatus::Done, None, None).expect("finish");

        assert_eq!(queued_count(db.conn()).expect("count"), 0);
    }
}
