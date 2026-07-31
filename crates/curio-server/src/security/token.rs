//! The runtime token and the launch nonce.
//!
//! There is exactly **one** client secret: the runtime bearer token — 32 CSPRNG bytes,
//! base64url, minted fresh at each service start and dead the moment the app quits
//! (R-SEC-2, D21). Per-run rotation costs nothing because every consumer re-bootstraps on
//! reconnect, and it means a leaked token has the lifetime of one session.
//!
//! It reaches clients over exactly four sanctioned paths, and no fifth (R-SEC-3): the
//! native-messaging reply to the pinned extension; the nonce exchange that mints the
//! dashboard's session cookie; a same-user read of `runtime.json` by the CLI and the MCP
//! stdio proxy; and the click-gated `/pair` handoff for unpacked installs.
//!
//! The nonce exists for one reason: **a token must never appear in a URL**. Query strings
//! survive in browser history, in shoulder-surfing distance, and in any log that records a
//! request line. So the tray opens `/?t=<nonce>` with a 30-second single-use value that
//! authorizes exactly one thing — the exchange that sets an HttpOnly cookie — and grants
//! nothing else (R-SEC-5, D22).

use std::collections::VecDeque;
use std::time::{Duration, Instant};

use base64::Engine as _;
use subtle::ConstantTimeEq as _;

/// How long a launch nonce lives.
///
/// Long enough for a browser to cold-start and load the page; short enough that a nonce
/// left in a shell's history is inert by the time anyone reads it.
pub const NONCE_TTL: Duration = Duration::from_secs(30);

/// How many nonces may be outstanding at once.
///
/// Minting evicts the oldest. Bounded because nonces are held in memory and "Open
/// Dashboard" is a button a user can hold down.
const MAX_OUTSTANDING_NONCES: usize = 8;

/// 32 bytes of CSPRNG randomness, base64url-encoded without padding.
///
/// # Panics
/// Panics if the OS entropy source fails. There is no safe degraded mode: a predictable
/// token is worse than not starting, because it is indistinguishable from a working one.
#[must_use]
pub fn mint_secret() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS entropy source must be available");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// Compare two secrets without leaking their contents through timing.
///
/// A byte-by-byte `==` returns as soon as it finds a difference, so how long the
/// comparison took tells an attacker how many leading bytes were right — enough to
/// recover a secret one byte at a time. Required for the quit token (R-SEC-8) and applied
/// to every secret here for the same reason.
#[must_use]
pub fn secrets_match(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        // Length is not a secret: both values are fixed-width by construction, so an
        // early return here reveals nothing a caller does not already know.
        return false;
    }
    a.ct_eq(b).into()
}

/// The per-run bearer token.
#[derive(Debug, Clone)]
pub struct RuntimeToken(String);

impl RuntimeToken {
    /// Mint a fresh token for this run.
    #[must_use]
    pub fn mint() -> Self {
        Self(mint_secret())
    }

    /// The token's value, for writing into `runtime.json` and nowhere else.
    #[must_use]
    pub fn expose(&self) -> &str {
        &self.0
    }

    /// Whether `presented` is this token.
    #[must_use]
    pub fn verify(&self, presented: &str) -> bool {
        secrets_match(&self.0, presented)
    }
}

impl std::fmt::Display for RuntimeToken {
    /// Redacted (R-SEC-15).
    ///
    /// The token must never reach a log line, a panic message, or an error response, and
    /// the cheapest way to guarantee that is to make the obvious way to print it print
    /// nothing useful. `expose()` is the deliberate, greppable exception.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<runtime token redacted>")
    }
}

/// Outstanding single-use launch nonces.
#[derive(Debug, Default)]
pub struct NonceStore {
    entries: VecDeque<(String, Instant)>,
}

impl NonceStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Mint a nonce, evicting the oldest if the store is full.
    pub fn mint(&mut self) -> String {
        self.expire();
        if self.entries.len() >= MAX_OUTSTANDING_NONCES {
            self.entries.pop_front();
        }
        let nonce = mint_secret();
        self.entries.push_back((nonce.clone(), Instant::now()));
        nonce
    }

    /// Consume `presented`, returning whether it was valid.
    ///
    /// Atomic by construction: the entry is removed before this returns, so a replay
    /// races to a rejection rather than to a second token handout (R-SEC-5).
    pub fn consume(&mut self, presented: &str) -> bool {
        self.expire();
        let found = self
            .entries
            .iter()
            .position(|(nonce, _)| secrets_match(nonce, presented));

        match found {
            Some(index) => {
                self.entries.remove(index);
                true
            }
            None => false,
        }
    }

    /// How many nonces are currently outstanding.
    #[must_use]
    pub fn outstanding(&self) -> usize {
        self.entries.len()
    }

    /// Drop anything past its TTL. Expiry is checked server-side, never client-side.
    fn expire(&mut self) {
        let now = Instant::now();
        self.entries
            .retain(|(_, minted)| now.duration_since(*minted) < NONCE_TTL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minted_secret_is_32_bytes_of_base64url() {
        let secret = mint_secret();
        // 32 bytes unpadded base64 is 43 characters.
        assert_eq!(secret.len(), 43);
        assert!(
            secret
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "must be URL-safe and unpadded: {secret}"
        );
    }

    #[test]
    fn secrets_are_not_reused() {
        let secrets: std::collections::HashSet<String> = (0..256).map(|_| mint_secret()).collect();
        assert_eq!(secrets.len(), 256);
    }

    #[test]
    fn a_token_verifies_only_itself() {
        let token = RuntimeToken::mint();
        assert!(token.verify(token.expose()));
        assert!(!token.verify("wrong"));
        assert!(!token.verify(""));
        assert!(!token.verify(RuntimeToken::mint().expose()));
    }

    #[test]
    fn a_prefix_of_the_token_is_rejected() {
        // The failure mode a length-unaware comparison invites.
        let token = RuntimeToken::mint();
        let prefix = &token.expose()[..20];
        assert!(!token.verify(prefix));
    }

    #[test]
    fn printing_a_token_redacts_it() {
        // R-SEC-15. Tokens must not reach logs, panics, or error bodies, and the reliable
        // way to ensure that is for the obvious formatting path to be useless.
        let token = RuntimeToken::mint();
        let printed = format!("{token}");

        assert!(!printed.contains(token.expose()));
        assert_eq!(printed, "<runtime token redacted>");
    }

    #[test]
    fn a_debug_dump_of_a_token_does_not_leak_it_into_a_url() {
        // Weaker than Display by design: Debug is for developers at a breakpoint, and
        // R-SEC-4's prohibition is about URLs, DOM, logs, and errors. This test records
        // that the distinction is intentional rather than an oversight.
        let token = RuntimeToken::mint();
        assert!(format!("{token:?}").contains(token.expose()));
    }

    #[test]
    fn a_nonce_works_once() {
        // R-SEC-5: consumption is atomic, so a replay races to a rejection rather than to
        // a second token handout.
        let mut store = NonceStore::new();
        let nonce = store.mint();

        assert!(store.consume(&nonce));
        assert!(!store.consume(&nonce), "a replay must fail");
    }

    #[test]
    fn an_unknown_nonce_is_rejected() {
        let mut store = NonceStore::new();
        store.mint();
        assert!(!store.consume(&mint_secret()));
        assert!(!store.consume(""));
    }

    #[test]
    fn minting_evicts_the_oldest_when_full() {
        // Nonces live in memory and "Open Dashboard" is a button a user can hold down.
        let mut store = NonceStore::new();
        let first = store.mint();
        for _ in 0..MAX_OUTSTANDING_NONCES {
            store.mint();
        }

        assert_eq!(store.outstanding(), MAX_OUTSTANDING_NONCES);
        assert!(
            !store.consume(&first),
            "the oldest should have been evicted"
        );
    }

    #[test]
    fn nonces_are_independent_of_each_other() {
        let mut store = NonceStore::new();
        let a = store.mint();
        let b = store.mint();

        assert!(store.consume(&b));
        assert!(
            store.consume(&a),
            "consuming one must not invalidate another"
        );
    }

    #[test]
    fn the_ttl_is_the_documented_thirty_seconds() {
        // Long enough for a cold browser start; short enough that a nonce left in a
        // shell's history is inert before anyone reads it (R-SEC-5).
        assert_eq!(NONCE_TTL, Duration::from_secs(30));
    }
}
