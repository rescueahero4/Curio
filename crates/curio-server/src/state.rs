//! What the service knows while it is running.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use curio_core::config::Config;
use curio_core::events::Event;
use curio_db::Db;
use curio_runtime::State;
use tokio::sync::broadcast;

use crate::security::{NonceStore, RuntimeToken};

/// How many events a slow subscriber may fall behind before it is dropped.
///
/// A dashboard tab that has been asleep in a background window is the normal case. Dropping
/// it is correct rather than unfortunate: the client's reconnect refetches, so a lagged
/// subscriber recovers with the truth, while an unbounded buffer would hold the whole
/// backlog of a bulk retag in memory against a 25 MB budget (R-BE-31).
const EVENT_BUFFER: usize = 256;

/// Shared service state, cloned into every handler.
///
/// Everything mutable is behind an `Arc` so the MCP service factory — which runs **per
/// request** (R-MCP-4) — can clone handles rather than rebuild pools.
#[derive(Clone)]
pub struct AppState {
    inner: Arc<Inner>,
}

struct Inner {
    /// Whether mutations are accepted.
    ///
    /// An atomic rather than a lock because the soft-disable middleware reads it on every
    /// request and writes are rare: pausing is a menu click, not a hot path.
    paused: AtomicBool,
    token: RuntimeToken,
    /// The separate secret for `POST /api/system/quit` (R-SEC-8).
    ///
    /// Never the runtime token, and never in any CORS allow-headers list: a paired
    /// extension holding the runtime token must not thereby hold a kill switch
    /// (Inventory §10.3).
    quit_token: String,
    nonces: Mutex<NonceStore>,
    version: String,
    /// The port actually bound this run.
    ///
    /// Carried in the state rather than threaded to each handler because with an ephemeral
    /// port there is no configured value to fall back on, and two surfaces need it —
    /// `/health` and the MCP snippet in Settings (R-BE-6).
    port: u16,
    data_root: PathBuf,
    config: Mutex<Config>,
    events: broadcast::Sender<Event>,
    /// Raised by `POST /api/system/quit`, awaited by the tray.
    ///
    /// A notification rather than a direct shutdown call because R-BE-7 has exactly one
    /// shutdown sequence: the quit route and the tray's Quit menu item converge here, so
    /// neither can drift into a second ordering that skips the WAL checkpoint.
    quit_requested: tokio::sync::Notify,
    /// Raised whenever a job is enqueued, awaited by the worker.
    ///
    /// The worker also polls every two seconds (Inventory §9), so this is a latency
    /// optimisation rather than the correctness mechanism: a capture should start
    /// assessing immediately, but a job that arrives while the worker is mid-claim must
    /// still be picked up. Notify has no backlog, which is exactly why the poll stays.
    job_enqueued: tokio::sync::Notify,
    /// The single write connection (R-DA-8).
    ///
    /// A mutex rather than a pool because there is exactly one writer by design. Handlers
    /// run on the same current-thread runtime as the worker, so this is uncontended in
    /// practice — it exists to make the single-writer invariant a type-level fact rather
    /// than a convention someone can forget.
    db: Mutex<Db>,
}

impl AppState {
    #[must_use]
    pub fn new(
        token: RuntimeToken,
        quit_token: impl Into<String>,
        version: impl Into<String>,
        port: u16,
        data_root: impl Into<PathBuf>,
        config: Config,
        db: Db,
    ) -> Self {
        let (events, _) = broadcast::channel(EVENT_BUFFER);
        Self {
            inner: Arc::new(Inner {
                paused: AtomicBool::new(false),
                token,
                quit_token: quit_token.into(),
                nonces: Mutex::new(NonceStore::new()),
                version: version.into(),
                port,
                data_root: data_root.into(),
                config: Mutex::new(config),
                events,
                quit_requested: tokio::sync::Notify::new(),
                job_enqueued: tokio::sync::Notify::new(),
                db: Mutex::new(db),
            }),
        }
    }

    /// Run `f` against the library.
    pub fn with_db<T>(&self, f: impl FnOnce(&Db) -> T) -> T {
        f(&self.lock_db())
    }

    /// Run `f` against the library with write access.
    pub fn with_db_mut<T>(&self, f: impl FnOnce(&mut Db) -> T) -> T {
        f(&mut self.lock_db())
    }

    /// Whether the app is currently refusing mutations.
    #[must_use]
    pub fn is_paused(&self) -> bool {
        self.inner.paused.load(Ordering::Relaxed)
    }

    /// Pause or resume.
    ///
    /// Soft-disable: the listener stays bound, reads keep working, and only mutations are
    /// refused (D2, D25). Never unbind and never exit — clients get a clean 503 they can
    /// explain to the user, and resuming is instant.
    pub fn set_paused(&self, paused: bool) {
        self.inner.paused.store(paused, Ordering::Relaxed);
    }

    /// The state as `runtime.json` and the `/ws` handshake report it.
    #[must_use]
    pub fn run_state(&self) -> State {
        if self.is_paused() {
            State::Paused
        } else {
            State::Running
        }
    }

    #[must_use]
    pub fn token(&self) -> &RuntimeToken {
        &self.inner.token
    }

    /// Whether `presented` is the quit token (R-SEC-8). Timing-safe.
    #[must_use]
    pub fn quit_token_matches(&self, presented: &str) -> bool {
        crate::security::secrets_match(&self.inner.quit_token, presented)
    }

    #[must_use]
    pub fn version(&self) -> &str {
        &self.inner.version
    }

    /// The port this run bound.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.inner.port
    }

    #[must_use]
    pub fn data_root(&self) -> &Path {
        &self.inner.data_root
    }

    /// A copy of the current configuration.
    ///
    /// Cloned rather than borrowed because `mcpEnabled` is read **per request**
    /// (R-MCP-8) and thresholds are re-read per job — holding a lock across either would
    /// serialise the whole server behind a settings save.
    #[must_use]
    pub fn config(&self) -> Config {
        self.lock(&self.inner.config).clone()
    }

    pub fn set_config(&self, config: Config) {
        *self.lock(&self.inner.config) = config;
    }

    /// Announce something. Never fails: an item was created whether or not anyone was
    /// listening (see [`curio_core::ports::EventSink`]).
    pub fn publish(&self, event: Event) {
        let _ = self.inner.events.send(event);
    }

    /// Subscribe to the push stream. One receiver per SSE or WebSocket connection.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<Event> {
        self.inner.events.subscribe()
    }

    /// Ask the process to shut down (R-BE-7).
    pub fn request_quit(&self) {
        self.inner.quit_requested.notify_waiters();
    }

    /// Wait for a quit request. Awaited by the owner of the shutdown sequence.
    pub async fn quit_requested(&self) {
        self.inner.quit_requested.notified().await;
    }

    /// Tell the worker there is something to do.
    ///
    /// Call this after every enqueue. Missing one costs latency, not correctness — the
    /// worker's two-second poll finds the job anyway (Inventory §9) — but "the card sat
    /// at processing for two seconds after I pressed capture" is the kind of slowness a
    /// user reads as brokenness.
    pub fn wake_worker(&self) {
        self.inner.job_enqueued.notify_one();
    }

    /// Wait for a job to be enqueued.
    pub async fn job_enqueued(&self) {
        self.inner.job_enqueued.notified().await;
    }

    /// Mint a single-use launch nonce (R-SEC-5).
    pub fn mint_nonce(&self) -> String {
        self.lock(&self.inner.nonces).mint()
    }

    /// Consume a launch nonce, returning whether it was valid.
    pub fn consume_nonce(&self, presented: &str) -> bool {
        self.lock(&self.inner.nonces).consume(presented)
    }

    fn lock_db(&self) -> std::sync::MutexGuard<'_, Db> {
        self.lock(&self.inner.db)
    }

    /// Recover from a poisoned lock rather than propagating the panic.
    ///
    /// Every value guarded here is either a plain snapshot (config, the connection) or a
    /// bounded queue of short-lived values (nonces). None has an invariant that can be left
    /// half-updated, so refusing to serve for the rest of the run would be strictly worse
    /// than carrying on.
    fn lock<'a, T>(&self, target: &'a Mutex<T>) -> std::sync::MutexGuard<'a, T> {
        target
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl curio_core::ports::EventSink for AppState {
    fn publish(&self, event: Event) {
        AppState::publish(self, event);
    }
}

impl std::fmt::Debug for AppState {
    /// Redacted (R-SEC-15): this struct holds the runtime token, and `{state:?}` in a
    /// tracing call is exactly how a secret reaches a log file.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("paused", &self.is_paused())
            .field("version", &self.inner.version)
            .finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use curio_core::events::EventName;

    fn state() -> AppState {
        AppState::new(
            RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            Config::default(),
            Db::open_in_memory().expect("in-memory library"),
        )
    }

    #[test]
    fn a_fresh_service_is_running() {
        assert!(!state().is_paused());
        assert_eq!(state().run_state(), State::Running);
    }

    #[test]
    fn pausing_is_reversible_and_reported() {
        let state = state();

        state.set_paused(true);
        assert!(state.is_paused());
        assert_eq!(state.run_state(), State::Paused);

        state.set_paused(false);
        assert!(!state.is_paused());
        assert_eq!(state.run_state(), State::Running);
    }

    #[test]
    fn clones_share_one_state() {
        // Every handler holds a clone. If pausing only affected the clone that received
        // the menu click, the tray would say paused while the server kept accepting
        // writes.
        let state = state();
        let clone = state.clone();

        state.set_paused(true);
        assert!(clone.is_paused());
    }

    #[test]
    fn nonces_round_trip_through_the_state() {
        let state = state();
        let nonce = state.mint_nonce();

        assert!(state.consume_nonce(&nonce));
        assert!(!state.consume_nonce(&nonce));
    }

    #[test]
    fn the_quit_token_is_not_the_runtime_token() {
        // R-SEC-8 / Inventory §10.3. A paired client holding the runtime token must not
        // thereby hold a kill switch.
        let state = state();

        assert!(state.quit_token_matches("quit-secret"));
        assert!(!state.quit_token_matches(state.token().expose()));
    }

    #[test]
    fn a_subscriber_receives_what_is_published() {
        let state = state();
        let mut stream = state.subscribe();

        state.publish(Event::item_deleted("01A"));

        let received = stream.try_recv().expect("an event");
        assert_eq!(received.name, EventName::ItemDeleted);
    }

    #[test]
    fn publishing_with_nobody_listening_is_fine() {
        // An item was created whether or not a dashboard was open, and a failed publish
        // must never fail the operation that triggered it.
        state().publish(Event::vocabulary_updated());
    }

    #[test]
    fn debugging_the_state_does_not_print_the_token() {
        // R-SEC-15. `tracing::debug!(?state)` must not be the thing that writes a bearer
        // token to disk.
        let state = state();
        let printed = format!("{state:?}");

        assert!(!printed.contains(state.token().expose()));
        assert!(printed.contains("paused"));
    }
}
