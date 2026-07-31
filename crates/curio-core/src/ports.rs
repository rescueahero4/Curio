//! The seams: traits the domain defines and the outer layers implement.
//!
//! This is the mechanism behind R-DEL-2. The domain names what it needs — somewhere to
//! store things, somewhere to announce things, a clock, an embedder — and `curio-db` and
//! `curio-server` supply them. Nothing points inward except data.
//!
//! The traits are small on purpose. Each one exists because a specific seam has to stay
//! cuttable: storage so the database can change without touching domain rules, the event
//! sink so the tray/service split can become a process boundary later, the clock so
//! time-dependent rules are testable without sleeping, and the embedder so the daemon
//! never links a model runtime.

use crate::events::Event;

/// Somewhere to announce that something happened.
///
/// Implemented by the in-process bus in `curio-server`, which fans out to SSE and the
/// WebSocket. The domain neither knows nor cares that there are two transports.
///
/// Publishing must not fail the operation that triggered it: an item was created whether
/// or not anyone was listening, so implementations swallow delivery problems rather than
/// propagating them.
pub trait EventSink: Send + Sync {
    fn publish(&self, event: Event);
}

/// A no-op sink, for tests and for code paths that run before the bus exists.
#[derive(Debug, Clone, Copy, Default)]
pub struct NullEventSink;

impl EventSink for NullEventSink {
    fn publish(&self, _event: Event) {}
}

/// The current time, as the domain sees it.
///
/// A trait rather than a direct call to [`crate::time`] so that rules with a time
/// component — job backoff, parking, the six-hour prompt claim window — can be tested by
/// moving the clock instead of sleeping through it.
pub trait Clock: Send + Sync {
    /// ISO-8601 UTC, second precision.
    fn now_iso(&self) -> String;

    /// ISO-8601 UTC, millisecond precision. `prompts.sent_at` only (R-DA-6).
    fn now_iso_millis(&self) -> String;
}

/// The real clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_iso(&self) -> String {
        crate::time::now_iso()
    }

    fn now_iso_millis(&self) -> String {
        crate::time::now_iso_millis()
    }
}

/// Turns text into a vector.
///
/// **Post-v1.** Vector search is designed in and deferred (D7); nothing implements this
/// in v1 and no embedding is ever generated. The trait exists now because its whole
/// purpose is to be a link-time boundary: the daemon must not link an ML runtime, so the
/// default implementation when this activates is a remote API behind the user's own key,
/// and a local model is a later `impl` rather than a dependency anyone inherits (D9,
/// R-BE-27).
///
/// What gets embedded is the **AI's own description** of an item — name, short
/// description, tags, families — not the image. The assessment already paid a frontier
/// vision model to compress the visual into the library's vocabulary; embedding that text
/// reuses the signal, keeps the vectors cheap, and means semantic search speaks the same
/// language the user browses in (R-DA-12).
pub trait Embedder: Send + Sync {
    /// A stable identifier for this embedder, stored alongside the vectors it produced.
    ///
    /// Changing embedder or dimension is a migration that truncates and rebuilds, so the
    /// database has to be able to tell which one wrote what.
    fn id(&self) -> &str;

    /// The dimension of the vectors this embedder produces.
    fn dimensions(&self) -> usize;

    /// Embed one document.
    ///
    /// # Errors
    /// Returns an error if the embedder is unconfigured or the call fails. Callers must
    /// treat that as "not searchable yet" and queue, never as data loss (R-DA-13).
    fn embed(&self, text: &str) -> crate::Result<Vec<f32>>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{Event, EventName};

    #[test]
    fn the_null_sink_accepts_anything() {
        // Used wherever the bus does not exist yet — boot, migrations, tests. Publishing
        // must never be the thing that fails an operation.
        NullEventSink.publish(Event::new(
            EventName::ItemUpdated,
            serde_json::json!({"id": "x"}),
        ));
    }

    #[test]
    fn the_system_clock_agrees_with_the_time_module() {
        // Same precision contract as R-DA-6: seconds everywhere, milliseconds for sent_at.
        assert_eq!(SystemClock.now_iso().len(), 20);
        assert_eq!(SystemClock.now_iso_millis().len(), 24);
    }
}
