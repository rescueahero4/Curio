//! What the service knows while it is running.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use curio_db::Db;
use curio_runtime::State;

use crate::security::{NonceStore, RuntimeToken};

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
    nonces: Mutex<NonceStore>,
    version: String,
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
    pub fn new(token: RuntimeToken, version: impl Into<String>, db: Db) -> Self {
        Self {
            inner: Arc::new(Inner {
                paused: AtomicBool::new(false),
                token,
                nonces: Mutex::new(NonceStore::new()),
                version: version.into(),
                db: Mutex::new(db),
            }),
        }
    }

    /// Run `f` against the library.
    pub fn with_db<T>(&self, f: impl FnOnce(&Db) -> T) -> T {
        let guard = self
            .inner
            .db
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        f(&guard)
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

    #[must_use]
    pub fn version(&self) -> &str {
        &self.inner.version
    }

    /// Mint a single-use launch nonce (R-SEC-5).
    pub fn mint_nonce(&self) -> String {
        self.lock_nonces().mint()
    }

    /// Consume a launch nonce, returning whether it was valid.
    pub fn consume_nonce(&self, presented: &str) -> bool {
        self.lock_nonces().consume(presented)
    }

    fn lock_nonces(&self) -> std::sync::MutexGuard<'_, NonceStore> {
        self.inner.nonces.lock().unwrap_or_else(|poisoned| {
            // The guarded state is a bounded queue of short-lived values. Recovering is
            // strictly better than refusing to authenticate for the rest of the run.
            poisoned.into_inner()
        })
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

    fn state() -> AppState {
        AppState::new(
            RuntimeToken::mint(),
            "0.1.0",
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
    fn debugging_the_state_does_not_print_the_token() {
        // R-SEC-15. `tracing::debug!(?state)` must not be the thing that writes a bearer
        // token to disk.
        let state = state();
        let printed = format!("{state:?}");

        assert!(!printed.contains(state.token().expose()));
        assert!(printed.contains("paused"));
    }
}
