//! The route surface.
//!
//! Route **order** is part of the contract, not an implementation detail (R-BE-8):
//! `/mcp` sits above the SPA catch-all, and the catch-all 404s the reserved prefixes
//! rather than answering with the shell.
//!
//! At scaffold time only `/health` and the SPA exist. The rest of the surface — items,
//! bulk, vocabulary, prompts, projects, settings, system, the jails, SSE and `/ws` — is
//! carried over one-for-one from the previous implementation and lands with P1 and P2.
//! The middleware map in ARCH-01 says which guards apply to each group.

pub mod health;

use axum::Router;
use axum::extract::State;
use axum::http::Uri;
use axum::response::Response;
use axum::routing::get;

use crate::state::AppState;

/// Build the router for a service bound to `port`.
///
/// `port` is threaded in rather than read back from the listener because `/health`
/// reports it, and the value must be the one we actually bound — with an ephemeral port
/// there is no configured value to fall back on (R-BE-6).
pub fn build(state: AppState, port: u16) -> Router {
    Router::new()
        // Unauthenticated by design (R-SEC-11): the extension's status dot and stale-
        // instance probing both need an answer before any credential exists.
        .route(
            "/health",
            get(move |state: State<AppState>| health::handler(state, port)),
        )
        // Everything not claimed above is the dashboard. This must stay last, and
        // `/mcp` must be registered before it once the MCP surface mounts (P4).
        .fallback(spa_fallback)
        .with_state(state)
}

async fn spa_fallback(uri: Uri) -> Response {
    crate::spa::serve(uri.path())
}
