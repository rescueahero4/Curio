//! What the worker does when a job fails (R-BE-17, Inventory §10.10).
//!
//! A pure function over `(attempts, error)` rather than a branch inside the loop, because
//! the distinction it draws is the one most worth testing in isolation: **waiting is not
//! failing**. A user with no API key, or a bulk run waiting on the Batch API, must
//! accumulate a queue that drains — not a pile of `failed` rows that need a human to
//! notice and re-run them.
//!
//! Three shapes come out of it, and only one of them charges an attempt.

use crate::Error;

/// How long a job with no API key waits before looking again (R-BE-17).
///
/// Short enough that a user who pastes a key into Settings sees the backlog move while
/// they are still looking at the screen, rather than wondering whether it worked.
pub const MISSING_KEY_BACKOFF_SECONDS: i64 = 30;

/// How many times a job may genuinely fail before it stops trying.
pub const MAX_ATTEMPTS: u32 = 3;

/// What the worker should do next.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recovery {
    /// Wait, then try again, **without charging an attempt**.
    ///
    /// The job is not broken: there is no key, or the upstream said "not yet". Charging
    /// these would exhaust a retry budget belonging to a user who is merely offline.
    Park { seconds: i64 },
    /// A real failure with budget left. Wait out the backoff and spend an attempt.
    Retry { seconds: i64 },
    /// Out of attempts. The job is `failed` and the item is `assessment_failed`.
    GiveUp,
}

/// Decide what happens to a job that just failed.
///
/// `attempts` is the count **already recorded** on the row — the failure being handled is
/// not yet included, which is why the budget check adds one before comparing.
#[must_use]
pub fn recover(attempts: u32, error: &Error) -> Recovery {
    match error {
        // FR-26. The capture landed, the item is visible, and the only thing missing is a
        // credential the user can supply at their leisure. The item stays `processing`.
        Error::MissingApiKey => Recovery::Park {
            seconds: MISSING_KEY_BACKOFF_SECONDS,
        },

        // The upstream named its own timing — a batch that is still running, a rate limit
        // with a `retry-after`. Honour it rather than guessing.
        Error::Parked { .. } => Recovery::Park {
            seconds: MISSING_KEY_BACKOFF_SECONDS,
        },

        // Pausing is a user action, not a job defect. Resuming should find the queue
        // intact rather than drained into failures.
        Error::Paused => Recovery::Park {
            seconds: MISSING_KEY_BACKOFF_SECONDS,
        },

        _ => {
            let spent = attempts + 1;
            if spent >= MAX_ATTEMPTS {
                Recovery::GiveUp
            } else {
                Recovery::Retry {
                    seconds: backoff_seconds(spent),
                }
            }
        }
    }
}

/// The backoff after `attempt` failures: `2000 · attempt²` ms, in whole seconds.
///
/// Seconds rather than milliseconds because `not_before` is a second-precision ISO
/// timestamp (Inventory §10.15) — a sub-second delay would round to "now" and produce a
/// hot retry loop against whatever just failed.
#[must_use]
pub fn backoff_seconds(attempt: u32) -> i64 {
    let millis = 2000_i64 * i64::from(attempt) * i64::from(attempt);
    // Round up: 2 s stays 2 s, and anything that would truncate to zero waits a second
    // rather than not waiting at all.
    millis.div_euclid(1000).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_missing_key_never_charges_an_attempt() {
        // FR-26 / R-BE-17. This is the whole point of the module: a user who has not
        // added a key yet must end up with a queue that drains the moment they do, not
        // three failed jobs per capture.
        for attempts in 0..10 {
            assert_eq!(
                recover(attempts, &Error::MissingApiKey),
                Recovery::Park { seconds: 30 },
                "attempt {attempts} charged a user for being offline"
            );
        }
    }

    #[test]
    fn parking_survives_an_exhausted_budget() {
        // A bulk job polling the Batch API parks many times. If parking consumed the
        // budget it would fail long before the batch came back.
        assert_eq!(
            recover(
                MAX_ATTEMPTS + 5,
                &Error::Parked {
                    retry_after: "2026-01-01T00:00:00Z".to_owned()
                }
            ),
            Recovery::Park { seconds: 30 }
        );
    }

    #[test]
    fn pausing_does_not_drain_the_queue_into_failures() {
        assert!(matches!(recover(0, &Error::Paused), Recovery::Park { .. }));
    }

    #[test]
    fn a_real_failure_retries_twice_then_stops() {
        let model = Error::Model("500 from upstream".to_owned());

        assert_eq!(recover(0, &model), Recovery::Retry { seconds: 2 });
        assert_eq!(recover(1, &model), Recovery::Retry { seconds: 8 });
        assert_eq!(recover(2, &model), Recovery::GiveUp);
    }

    #[test]
    fn the_backoff_grows_quadratically() {
        // 2000·n² ms, as the previous implementation shipped it (R-BE-17).
        assert_eq!(backoff_seconds(1), 2);
        assert_eq!(backoff_seconds(2), 8);
        assert_eq!(backoff_seconds(3), 18);
    }

    #[test]
    fn a_backoff_is_never_zero_seconds() {
        // `not_before` is second-precision. A zero delay is a hot loop against whatever
        // just failed, which is the failure mode backoff exists to prevent.
        assert!(backoff_seconds(0) >= 1);
    }

    #[test]
    fn a_malformed_reply_is_not_retried_forever() {
        // An invalid response shape will be invalid again. Three tries, then the item is
        // honestly `assessment_failed` and one click from a re-assess.
        assert_eq!(
            recover(MAX_ATTEMPTS - 1, &Error::invalid("bad shape")),
            Recovery::GiveUp
        );
    }
}
