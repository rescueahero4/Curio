//! `GET /health` — open by design, and deliberately boring.
//!
//! This is the one endpoint that answers without a credential, and that is a **contract,
//! not an oversight** (R-SEC-11, R-BE-12, Inventory §10.29). Two consumers need it before
//! they hold anything:
//!
//! * the extension's popup, which must render a green or gray status dot on open;
//! * boot-time stale-instance probing, which must tell "a live server" from "a dead port".
//!
//! Since it cannot be authenticated, it is minimised instead. The response is exactly six
//! fields and **nothing more** — no paths, no tokens, no OS or user information, no
//! library contents. `port` merely echoes what the caller already used to reach us, and
//! `api_key_configured` is a boolean rather than a prefix. Counts are the most a
//! cross-origin reader learns, and that trade is accepted explicitly.
//!
//! Adding a field here is a security decision, which is why the shape is asserted by test.

use axum::Json;
use axum::extract::State;
use serde::Serialize;

use crate::state::AppState;

/// The whole of the health response. Six fields, fixed.
#[derive(Debug, Serialize)]
pub struct Health {
    pub status: &'static str,
    pub version: String,
    pub port: u16,
    pub items: i64,
    pub queue: i64,
    pub api_key_configured: bool,
}

/// Build the response body.
///
/// Separated from the handler so the field set can be asserted without a running server.
#[must_use]
pub fn snapshot(state: &AppState, port: u16) -> Health {
    let (items, queue) = state.with_db(|db| {
        let items = db
            .conn()
            .query_row("SELECT COUNT(*) FROM items", [], |row| row.get::<_, i64>(0))
            .unwrap_or(0);
        let queue = db
            .conn()
            .query_row(
                "SELECT COUNT(*) FROM jobs WHERE status IN ('queued','running')",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
        (items, queue)
    });

    Health {
        // Reports "ok" while paused too. Paused is a soft-disable — the server is healthy
        // and reachable, it is only refusing mutations (D25) — and the extension learns
        // about pausing from the `/ws` handshake, which is authenticated. Leaking the
        // pause state to any origin that can reach loopback would tell a hostile page
        // more about the user's session than it needs to know.
        status: "ok",
        version: state.version().to_owned(),
        port,
        items,
        queue,
        api_key_configured: api_key_is_configured(),
    }
}

/// Handler for `GET /health`.
pub async fn handler(State(state): State<AppState>, port: u16) -> Json<Health> {
    Json(snapshot(&state, port))
}

/// Whether an API key is available.
///
/// Reads the environment variable only. The keychain backends — DPAPI on Windows, the
/// macOS Keychain, and the AES-256-GCM encrypted-file fallback — land in P1 with the rest
/// of the secrets layer (R-SEC-10); until then this reports the one source that already
/// works, rather than reporting `false` and making a configured user look unconfigured.
fn api_key_is_configured() -> bool {
    std::env::var_os("ANTHROPIC_API_KEY").is_some_and(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RuntimeToken;
    use curio_db::Db;

    fn state() -> AppState {
        AppState::new(
            RuntimeToken::mint(),
            "0.1.0",
            Db::open_in_memory().expect("library"),
        )
    }

    #[test]
    fn the_response_has_exactly_the_six_contracted_fields() {
        // R-SEC-11. This endpoint is readable by any origin that can reach loopback, so
        // every field is a deliberate disclosure. A seventh field must be a decision, not
        // a diff — this test is where that decision gets made.
        let json = serde_json::to_value(snapshot(&state(), 51_234)).expect("serialize");
        let object = json.as_object().expect("object");

        let mut keys: Vec<&str> = object.keys().map(String::as_str).collect();
        keys.sort_unstable();

        assert_eq!(
            keys,
            [
                "api_key_configured",
                "items",
                "port",
                "queue",
                "status",
                "version"
            ]
        );
    }

    #[test]
    fn the_response_carries_no_secret() {
        let state = state();
        let json = serde_json::to_string(&snapshot(&state, 51_234)).expect("serialize");

        assert!(!json.contains(state.token().expose()));
    }

    #[test]
    fn api_key_configured_is_a_boolean_not_a_prefix() {
        // Inventory §10.29 is explicit. A masked prefix would tell a cross-origin reader
        // something about the key itself.
        let json = serde_json::to_value(snapshot(&state(), 51_234)).expect("serialize");
        assert!(json["api_key_configured"].is_boolean());
    }

    #[test]
    fn counts_come_from_the_library() {
        let state = state();
        state.with_db(|db| {
            db.conn()
                .execute_batch(
                    "INSERT INTO items (id, name, screenshot_path, created_at, updated_at)
                       VALUES ('01A', 'x', 'p', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                     INSERT INTO jobs (id, kind, payload, status, created_at, updated_at)
                       VALUES ('01J', 'assess_item', '{}', 'queued', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');
                     INSERT INTO jobs (id, kind, payload, status, created_at, updated_at)
                       VALUES ('01K', 'assess_item', '{}', 'done', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z');",
                )
                .expect("seed");
        });

        let health = snapshot(&state, 51_234);

        assert_eq!(health.items, 1);
        assert_eq!(health.queue, 1, "finished jobs are not queue depth");
    }

    #[test]
    fn status_stays_ok_while_paused() {
        // Paused is a soft-disable, not ill health — and this endpoint is unauthenticated,
        // so it is not where the pause state should be published.
        let state = state();
        state.set_paused(true);

        assert_eq!(snapshot(&state, 51_234).status, "ok");
    }
}
