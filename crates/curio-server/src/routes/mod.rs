//! The route surface.
//!
//! Route **order** is part of the contract, not an implementation detail (R-BE-8): `/mcp`
//! sits above the SPA catch-all, and the catch-all 404s the reserved prefixes rather than
//! answering with the shell. An agent that asks for a JSON-RPC frame and receives a page of
//! markup reports something incomprehensible to its user (Inventory §10.6).
//!
//! Middleware is applied per **group**, following ARCH-01's middleware map. The three groups
//! that matter:
//!
//! | Group | Identity | Paused | Credential |
//! |---|---|---|---|
//! | `/health`, SPA shell and assets | no | exempt | none |
//! | `/api/*` reads, SSE, `/ws`, both jails | yes | reads continue | cookie or bearer |
//! | `/api/*` mutations and ingest | yes | `503 + Retry-After` | cookie or bearer |
//!
//! `POST /api/auth/exchange` and `POST /api/pair/authorize` sit outside the credential
//! layer by design: the first authenticates by its single-use nonce, the second by a user's
//! explicit click behind the identity checks (R-SEC-2, R-SEC-3d).

pub mod api;
pub mod auth;
pub mod error;
pub mod events;
pub mod files;
pub mod health;
pub mod mcp;
pub mod switcher;
pub mod ws;

use axum::Router;
use axum::extract::{DefaultBodyLimit, State};
use axum::http::Uri;
use axum::middleware::{from_fn, from_fn_with_state};
use axum::response::Response;
use axum::routing::{delete, get, patch, post, put};

use crate::security::guard;
use crate::state::AppState;

/// The maximum request body (R-BE-13).
///
/// Sized to the capture pipeline, not to a round number: a full-page stitch is capped at
/// 20,000 device pixels (Inventory §10.31), and 64 MB is what that costs as PNG at a high
/// device pixel ratio. The two numbers move together.
const MAX_BODY_BYTES: usize = 64 * 1024 * 1024;

/// Build the router for a service bound to `port`.
///
/// `port` is threaded in rather than read back from the listener because `/health` reports
/// it, and the value must be the one we actually bound — with an ephemeral port there is no
/// configured value to fall back on (R-BE-6).
pub fn build(state: AppState, port: u16) -> Router {
    Router::new()
        // Unauthenticated by design (R-SEC-11): the extension's status dot and stale-
        // instance probing both need an answer before any credential exists.
        .route(
            "/health",
            get(move |state: State<AppState>| health::handler(state, port)),
        )
        .merge(open_routes(&state))
        .merge(read_routes(&state))
        .merge(mutating_routes(&state))
        // **Above the catch-all, and that ordering is the rule** (R-BE-8, Inventory §10.6).
        // Registered below it, a `GET /mcp` from a disabled or misconfigured client would
        // be answered with the dashboard's HTML — which an MCP client reports as a parse
        // error with no trace of the real cause.
        //
        // The service carries no credential layer: rmcp validates `Host` itself and the
        // surface is gated per request by `mcpEnabled` inside the handler (R-MCP-8), which
        // is also why the paused middleware does not wrap it — only the dispatcher can
        // tell an MCP read from an MCP write (R-MCP-10).
        .nest_service("/mcp", mcp::service(state.clone()))
        // Everything not claimed above is the dashboard. This must stay last.
        .fallback(spa_fallback)
        .with_state(state)
}

/// Routes that carry the identity checks but no credential.
fn open_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/auth/exchange", post(auth::exchange))
        .route("/api/auth/logout", post(auth::logout))
        .route("/api/pair/authorize", post(auth::pair_authorize))
        // `/ws` belongs here, not with the reads, and the distinction is not cosmetic: a
        // browser `WebSocket` can send neither headers nor cookies on the upgrade, so the
        // shared credential layer would 401 every client before the handler ever ran. Its
        // credential is the **first message** within 5 s (R-BE-32, D23). It is also exempt
        // from pause — the socket stays open and the paused state is announced over it,
        // because a disconnected extension cannot tell "paused" from "not running", which
        // is the one distinction FR-22 requires it to make.
        .route("/ws", get(ws::upgrade))
        // The quit token, and nothing else, authenticates this (R-SEC-8). It is exempt
        // from pause: a paused app must still be quittable.
        .route(
            "/api/system/quit",
            post(api::system::quit)
                .layer(from_fn_with_state(state.clone(), guard::authenticate_quit)),
        )
        .layer(from_fn(guard::identity))
}

/// Reads. Never blocked by pause — browsing a paused library is the point of soft-disable
/// (D25).
fn read_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/auth/session", get(auth::probe))
        .route("/api/auth/nonce", post(auth::mint))
        .route("/api/events", get(events::stream))
        .route("/api/items", get(api::items::list))
        // Above `/api/items/{id}` for the same reason as `/api/prompts/template` below:
        // the literal has to win, or "count" is read as an item id.
        .route("/api/items/count", get(api::items::count))
        .route("/api/items/{id}", get(api::items::get))
        .route("/api/vocabulary", get(api::vocabulary::list))
        // Registered above `/api/prompts/{id}` so the literal wins: otherwise "template"
        // is read as an id and the editor asks for a prompt that does not exist.
        .route("/api/prompts/template", get(api::prompts::template))
        .route("/api/prompts", get(api::prompts::list))
        .route("/api/prompts/{id}", get(api::prompts::get))
        .route("/api/projects", get(api::projects::list))
        .route("/api/projects/{id}/variants", get(api::projects::variants))
        .route("/api/bulk/dedupe/latest", get(api::bulk::dedupe_latest))
        .route("/api/jobs", get(api::system::list_jobs))
        .route("/api/jobs/{id}", get(api::system::get_job))
        .route("/api/settings", get(api::settings::get))
        .route("/files/items/{id}/{file}", get(files::item_file))
        // Outside `/p/` on purpose. Served from inside the jail, a project folder that
        // happened to hold a file of the same name would shadow it — and the one thing this
        // script must be is the same script on every project.
        .route("/__curio/variant-switcher.js", get(switcher::script))
        .route("/p/{id}", get(project_root))
        .route("/p/{id}/{*rest}", get(files::project_file))
        .layer(from_fn_with_state(state.clone(), guard::authenticate))
        .layer(from_fn(guard::identity))
}

/// Mutations and ingest. Refused with `503 + Retry-After` while paused (R-BE-3).
fn mutating_routes(state: &AppState) -> Router<AppState> {
    Router::new()
        .route("/api/items", post(api::ingest::create))
        .route("/api/items/{id}", patch(api::items::patch))
        .route("/api/items/{id}", delete(api::items::delete))
        .route("/api/items/{id}/reassess", post(api::system::reassess))
        .route(
            "/api/items/{id}/resolve-grayzone",
            post(api::items::resolve_gray_zone),
        )
        .route("/api/bulk/edit", post(api::items::bulk_edit))
        .route("/api/bulk/retag", post(api::bulk::retag))
        .route("/api/bulk/dedupe", post(api::bulk::dedupe))
        .route("/api/vocabulary/{kind}", post(api::vocabulary::create))
        .route("/api/vocabulary/prune", post(api::vocabulary::prune))
        .route(
            "/api/vocabulary/{kind}/{id}",
            patch(api::vocabulary::update),
        )
        .route(
            "/api/vocabulary/{kind}/{id}",
            delete(api::vocabulary::delete),
        )
        .route(
            "/api/vocabulary/{kind}/{id}/merge",
            post(api::vocabulary::merge),
        )
        .route("/api/prompts", post(api::prompts::create))
        .route("/api/prompts/{id}", patch(api::prompts::update))
        .route("/api/prompts/{id}", delete(api::prompts::delete))
        .route("/api/prompts/{id}/serialize", post(api::prompts::serialize))
        .route("/api/prompts/{id}/sent", post(api::prompts::mark_sent))
        .route("/api/prompts/{id}/sent", delete(api::prompts::clear_sent))
        .route("/api/projects", post(api::projects::register))
        .route("/api/projects/{id}", patch(api::projects::update))
        .route("/api/projects/{id}", delete(api::projects::delete))
        .route("/api/projects/{id}/open", post(api::projects::open))
        .route("/api/jobs/{id}/cancel", post(api::system::cancel_job))
        .route("/api/settings", put(api::settings::put))
        .route(
            "/api/settings/api-key",
            delete(api::settings::clear_api_key),
        )
        .route("/api/settings/verify-key", post(api::settings::verify_key))
        .route("/api/system/reveal", post(api::system::reveal))
        .route(
            "/api/system/open-skill-file",
            post(api::system::open_skill_file),
        )
        .route(
            "/api/system/send-to-claude",
            post(api::system::send_to_claude),
        )
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(from_fn_with_state(state.clone(), guard::authenticate))
        .layer(from_fn_with_state(
            state.clone(),
            guard::refuse_while_paused,
        ))
        .layer(from_fn(guard::identity))
}

/// `GET /p/:id` — the project's front door, without a trailing path.
async fn project_root(
    state: State<AppState>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    files::project_file(state, axum::extract::Path((id, String::new()))).await
}

async fn spa_fallback(uri: Uri) -> Response {
    crate::spa::serve(uri.path())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RuntimeToken;
    use axum::body::Body;
    use axum::http::{Request, StatusCode, header};
    use curio_core::config::Config;
    use tower::ServiceExt as _;

    fn app() -> (Router, String) {
        let state = AppState::new(
            RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            Config::default(),
            curio_db::Db::open_in_memory().expect("db"),
        );
        let token = state.token().expose().to_owned();
        (build(state, 51234), token)
    }

    async fn get(path: &str, token: Option<&str>) -> StatusCode {
        let (app, minted) = app();
        let mut request = Request::builder()
            .uri(path)
            .header(header::HOST, "127.0.0.1:51234");
        if token.is_some() {
            request = request.header(header::AUTHORIZATION, format!("Bearer {minted}"));
        }
        app.oneshot(request.body(Body::empty()).expect("request"))
            .await
            .expect("response")
            .status()
    }

    #[tokio::test]
    async fn health_answers_without_a_credential() {
        assert_eq!(get("/health", None).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_library_needs_one() {
        assert_eq!(get("/api/items", None).await, StatusCode::UNAUTHORIZED);
        assert_eq!(get("/api/items", Some("yes")).await, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_variant_switcher_is_served_and_is_not_public() {
        // It is a sub-resource of a page that already carried a credential, so it sits with
        // the reads rather than beside `/health` — a script that anyone can fetch is a
        // wider surface than it needs to be for no gain.
        assert_eq!(
            get("/__curio/variant-switcher.js", Some("yes")).await,
            StatusCode::OK
        );
        assert_eq!(
            get("/__curio/variant-switcher.js", None).await,
            StatusCode::UNAUTHORIZED
        );
    }

    #[tokio::test]
    async fn a_typo_under_the_curio_prefix_is_a_404_not_the_dashboard() {
        // The shell arriving where JavaScript was asked for puts a syntax error in the
        // console of the *user's* prototype, three layers from the cause.
        assert_eq!(
            get("/__curio/nothing-here.js", Some("yes")).await,
            StatusCode::NOT_FOUND
        );
    }

    #[tokio::test]
    async fn the_template_route_is_not_read_as_a_prompt_id() {
        // Registered above `/api/prompts/{id}`. If the parameter route won, the editor
        // would ask for a prompt called "template" and get a 404 on every new prompt.
        assert_eq!(
            get("/api/prompts/template", Some("yes")).await,
            StatusCode::OK
        );
    }

    #[tokio::test]
    async fn the_websocket_upgrade_is_not_behind_the_credential_layer() {
        // R-BE-32: `/ws` authenticates by first message, because a browser `WebSocket` can
        // send neither headers nor cookies on the upgrade. It was originally grouped with
        // the reads, which put the bearer/cookie layer in front of it — every client got a
        // 401 before the handler ran, and nothing caught it because the ws tests only
        // asserted constants. Found by driving a real socket.
        //
        // A credential-less upgrade must therefore NOT be 401. It is rejected as a bad
        // request instead, since this is a plain GET with no upgrade headers.
        let (app, _token) = app();
        let status = app
            .oneshot(
                Request::builder()
                    .uri("/ws")
                    .header(header::HOST, "127.0.0.1:51234")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .status();

        assert_ne!(
            status,
            StatusCode::UNAUTHORIZED,
            "the credential layer must not sit in front of the first-message handshake"
        );
    }

    #[tokio::test]
    async fn prune_is_not_read_as_a_vocabulary_kind() {
        // The same shape as the template route: `/api/vocabulary/{kind}` would otherwise
        // swallow `/api/vocabulary/prune` and try to create a family called "prune".
        let (app, token) = app();
        let status = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/vocabulary/prune")
                    .header(header::HOST, "127.0.0.1:51234")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .status();

        // A create would have needed a JSON body and answered 415 or 422; prune takes none.
        assert_eq!(status, StatusCode::OK);
    }

    #[tokio::test]
    async fn an_api_path_that_does_not_exist_404s_rather_than_serving_the_shell() {
        // R-BE-8. Serving HTML here turns a routing mistake into a client-side parse error
        // with no trace of the real cause.
        let status = get("/api/nothing-here", Some("yes")).await;
        assert!(
            status == StatusCode::NOT_FOUND || status == StatusCode::METHOD_NOT_ALLOWED,
            "{status}"
        );
    }

    #[tokio::test]
    async fn a_client_side_route_falls_back_to_the_shell() {
        // These have no file behind them; the shell must answer so the router can take
        // over (R-FE-2). Without built assets the fallback reports 503, which is still not
        // a 404 — the route was matched.
        let status = get("/settings", None).await;
        assert!(
            status == StatusCode::OK || status == StatusCode::SERVICE_UNAVAILABLE,
            "{status}"
        );
    }

    #[tokio::test]
    async fn quitting_needs_the_quit_token_not_the_runtime_token() {
        // R-SEC-8 / Inventory §10.3: a paired client holding the runtime token must not
        // thereby hold a kill switch.
        let (app, runtime) = app();
        let status = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/system/quit")
                    .header(header::HOST, "127.0.0.1:51234")
                    .header(header::AUTHORIZATION, format!("Bearer {runtime}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response")
            .status();

        assert_eq!(status, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_paused_read_still_answers_but_a_paused_write_does_not() {
        // D25 / ARCH-08 break #10, end to end through the real router.
        let state = AppState::new(
            RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            Config::default(),
            curio_db::Db::open_in_memory().expect("db"),
        );
        let token = state.token().expose().to_owned();
        state.set_paused(true);

        let read = build(state.clone(), 51234)
            .oneshot(
                Request::builder()
                    .uri("/api/items")
                    .header(header::HOST, "127.0.0.1:51234")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(read.status(), StatusCode::OK);

        let write = build(state, 51234)
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/prompts")
                    .header(header::HOST, "127.0.0.1:51234")
                    .header(header::AUTHORIZATION, format!("Bearer {token}"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from("{}"))
                    .expect("request"),
            )
            .await
            .expect("response");
        assert_eq!(write.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(write.headers().contains_key(header::RETRY_AFTER));
    }
}
