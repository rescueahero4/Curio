//! The middleware stack, in the one order that is correct.
//!
//! ```text
//! Host → Origin → (Sec-Fetch-Site) → soft-disable 503 → credential → handler
//! ```
//!
//! Identity before availability before credentials (ARCH-06 §Middleware order). The
//! ordering is what makes a DNS-rebinding attempt learn *nothing* — not even whether the
//! app is paused — while a legitimate paused client still gets a clean 503 on a write
//! without needing to refresh a credential first.
//!
//! Every rejection is a 403 rather than a 404 or a 401. A 401 invites a retry with
//! credentials, which is exactly the wrong advice for a caller whose Host header disqualified
//! it before credentials were considered.

use axum::extract::{Request, State};
use axum::http::{HeaderMap, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use crate::security::{host_is_loopback, origin_is_allowed, sec_fetch_site_is_hostile, session};
use crate::state::AppState;

/// How long a paused client should wait before trying again.
///
/// Resuming is a menu click, so this is short. It exists to give an automated client a
/// number rather than a guess (R-BE-3).
const RETRY_AFTER_SECONDS: &str = "5";

/// What a route group is allowed to do while paused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Reads: never blocked by pause. Browsing a paused library is the whole point of
    /// soft-disable (D25).
    Read,
    /// Mutations and ingest: refused with `503 + Retry-After` while paused.
    Mutation,
}

/// Headers a cross-origin caller may send (R-SEC-8, Inventory §1).
///
/// **`x-curio-quit-token` is absent, and must stay absent.** A browser refuses to send a
/// header the preflight did not allow, so leaving it out is what stops a token-holding
/// cross-origin client from reaching the kill switch. Adding it here would hand every
/// paired client a way to stop the app — the exact escalation R-SEC-8 exists to prevent.
const ALLOWED_REQUEST_HEADERS: &str = "authorization, content-type";

/// Methods the API answers. Advisory to the browser; routing and `authenticate` remain the
/// enforcement, so naming a method here grants nothing the router would otherwise refuse.
const ALLOWED_METHODS: &str = "GET, POST, PATCH, DELETE, OPTIONS";

/// How long a browser may cache a preflight. Ten minutes turns the extension's per-capture
/// preflight into a once-per-session one without outliving a run's token.
const PREFLIGHT_MAX_AGE: &str = "600";

/// Tell the browser what the identity rules already permit.
///
/// `identity` decides whether an origin *may* call; this decides whether the browser is
/// *told* so. They are different jobs, and having only the first is why the extension could
/// complete a native-messaging handshake and still fail every `fetch` with "Failed to
/// fetch": the request was allowed and the answer was unreadable.
///
/// Two rules this must never break:
///
/// * **Never `*`.** The allowed set is a specific trio (R-SEC-7), and a wildcard on a
///   loopback daemon holding a bearer token and an API key would let any page on the
///   internet read the library.
/// * **Answer preflights before `authenticate`.** A CORS preflight carries no credentials —
///   the spec forbids it — so a preflight that must authenticate can never succeed. This
///   layer therefore sits outside the credential check and short-circuits `OPTIONS` itself.
///   That is not a hole: a preflight reveals only which methods and headers are permitted,
///   and the real request behind it still passes the full stack.
pub async fn cors(request: Request, next: Next) -> Response {
    let headers = request.headers();
    let host = header_str(headers, header::HOST).map(str::to_owned);
    let origin = header_str(headers, header::ORIGIN).map(str::to_owned);

    // Re-validated here rather than trusted from `identity`, so this layer is correct
    // wherever it is mounted and cannot be made unsafe by a future reordering.
    let granted = origin.filter(|origin| {
        !origin.is_empty() && origin != "null" && origin_is_allowed(Some(origin), host.as_deref())
    });

    let is_preflight = request.method() == axum::http::Method::OPTIONS;
    let mut response = if is_preflight {
        StatusCode::NO_CONTENT.into_response()
    } else {
        next.run(request).await
    };

    let Some(granted) = granted else {
        return response;
    };
    let Ok(value) = granted.parse() else {
        return response;
    };

    let out = response.headers_mut();
    out.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    // Without this a shared cache could hand one origin's permissive answer to another.
    out.insert(
        header::VARY,
        header::ORIGIN.as_str().parse().expect("header"),
    );

    if is_preflight {
        out.insert(
            header::ACCESS_CONTROL_ALLOW_METHODS,
            ALLOWED_METHODS.parse().expect("header"),
        );
        out.insert(
            header::ACCESS_CONTROL_ALLOW_HEADERS,
            ALLOWED_REQUEST_HEADERS.parse().expect("header"),
        );
        out.insert(
            header::ACCESS_CONTROL_MAX_AGE,
            PREFLIGHT_MAX_AGE.parse().expect("header"),
        );
    }

    response
}

/// Host, Origin, and `Sec-Fetch-Site` — the identity checks every guarded route shares.
///
/// Applied to `/api/*`, `/mcp`, `/ws`, and both jails. Deliberately **not** applied to
/// `GET /health`, which is open by contract (R-SEC-11).
pub async fn identity(request: Request, next: Next) -> Response {
    let headers = request.headers().clone();
    let host = header_str(&headers, header::HOST);

    if !host.is_some_and(host_is_loopback) {
        // Logged with the offending value because that is what makes a rebinding attempt
        // diagnosable — and it is not a credential, so R-SEC-15 permits it.
        tracing::warn!(
            host = host.unwrap_or("<absent>"),
            "refused: Host is not loopback"
        );
        return forbidden("this server answers only to a loopback Host");
    }

    if !origin_is_allowed(header_str(&headers, header::ORIGIN), host) {
        tracing::warn!(
            origin = header_str(&headers, header::ORIGIN).unwrap_or("<absent>"),
            "refused: Origin is not allowed"
        );
        return forbidden("that origin is not allowed to talk to Curio");
    }

    if sec_fetch_site_is_hostile(header_str(&headers, "sec-fetch-site")) {
        return forbidden("cross-site requests are refused");
    }

    next.run(request).await
}

/// Soft-disable for mutating routes (R-BE-3, D25).
pub async fn refuse_while_paused(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    if state.is_paused() {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            [(header::RETRY_AFTER, RETRY_AFTER_SECONDS)],
            axum::Json(serde_json::json!({
                "error": "paused",
                // The copy the dashboard shows verbatim. "Paused" is a state the user
                // chose, so the message says what to do rather than apologising.
                "message": "Curio is paused. Resume it from the tray to make changes.",
            })),
        )
            .into_response();
    }
    next.run(request).await
}

/// The credential check: session cookie, or bearer token (R-SEC-2, D22).
///
/// Both are accepted everywhere. The cookie exists because `EventSource` cannot send
/// headers; the bearer header exists because the extension, the CLI, and the MCP proxy are
/// not browsers. Neither is a fallback for the other — they are two client classes.
pub async fn authenticate(State(state): State<AppState>, request: Request, next: Next) -> Response {
    let headers = request.headers();
    let presented = bearer(headers).or_else(|| session::from_cookies(headers));

    match presented {
        Some(value) if state.token().verify(&value) => next.run(request).await,
        // A 401 here genuinely means "re-bootstrap": the token is per-run, so a session
        // minted by a previous run authenticates nothing (D21, R-SEC-17). Clients are
        // required to treat it as "the app restarted", never as a user-facing failure.
        _ => (
            StatusCode::UNAUTHORIZED,
            axum::Json(serde_json::json!({
                "error": "unauthenticated",
                "message": "Open Curio's dashboard from the tray to start a session.",
            })),
        )
            .into_response(),
    }
}

/// The quit token, and only the quit token (R-SEC-8).
pub async fn authenticate_quit(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let presented = request
        .headers()
        .get("x-curio-quit-token")
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default()
        .to_owned();

    if state.quit_token_matches(&presented) {
        next.run(request).await
    } else {
        (StatusCode::UNAUTHORIZED, "the quit token is required").into_response()
    }
}

fn bearer(headers: &HeaderMap) -> Option<String> {
    headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .map(str::to_owned)
}

fn header_str(headers: &HeaderMap, name: impl axum::http::header::AsHeaderName) -> Option<&str> {
    headers.get(name).and_then(|value| value.to_str().ok())
}

/// A refusal that names the reason without naming a secret (R-SEC-15).
fn forbidden(message: &'static str) -> Response {
    (
        StatusCode::FORBIDDEN,
        axum::Json(serde_json::json!({ "error": "forbidden", "message": message })),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RuntimeToken;
    use axum::Router;
    use axum::body::Body;
    use axum::http::Request as HttpRequest;
    use axum::routing::{get, post};
    use curio_core::config::Config;
    use tower::ServiceExt as _;

    fn state() -> AppState {
        AppState::new(
            RuntimeToken::mint(),
            "quit-secret",
            "0.1.0",
            51_234,
            std::env::temp_dir(),
            Config::default(),
            curio_db::Db::open_in_memory().expect("db"),
        )
    }

    fn app(state: AppState) -> Router {
        Router::new()
            .route("/read", get(|| async { "read" }))
            .route(
                "/write",
                post(|| async { "written" }).layer(axum::middleware::from_fn_with_state(
                    state.clone(),
                    refuse_while_paused,
                )),
            )
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                authenticate,
            ))
            .layer(axum::middleware::from_fn(identity))
            .layer(axum::middleware::from_fn(cors))
            .with_state(state)
    }

    /// The full response, for the tests that care about headers rather than status.
    async fn respond(state: &AppState, request: HttpRequest<Body>) -> Response {
        app(state.clone()).oneshot(request).await.expect("response")
    }

    fn header_of(response: &Response, name: header::HeaderName) -> Option<String> {
        response
            .headers()
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    fn request(method: &str, path: &str) -> axum::http::request::Builder {
        HttpRequest::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "127.0.0.1:51234")
    }

    async fn send(state: &AppState, request: HttpRequest<Body>) -> StatusCode {
        app(state.clone())
            .oneshot(request)
            .await
            .expect("response")
            .status()
    }

    #[tokio::test]
    async fn a_bearer_token_authenticates() {
        let state = state();
        let response = send(
            &state,
            request("GET", "/read")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::OK);
    }

    #[tokio::test]
    async fn the_session_cookie_authenticates_too() {
        // The reason the session is a cookie at all: EventSource cannot send headers, so
        // SSE would be unauthenticatable without this (D22).
        let state = state();
        let response = send(
            &state,
            request("GET", "/read")
                .header(
                    header::COOKIE,
                    format!("{}={}", session::COOKIE_NAME, state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_preflight_is_answered_without_a_credential() {
        // The regression this exists for: `authenticate` used to 401 the preflight, and a
        // CORS preflight carries no credentials by spec — so the extension could complete a
        // native-messaging handshake and still fail every capture with "Failed to fetch".
        let state = state();
        let response = respond(
            &state,
            request("OPTIONS", "/write")
                .header(header::ORIGIN, crate::security::EXTENSION_ORIGIN)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
            Some(crate::security::EXTENSION_ORIGIN)
        );
    }

    #[tokio::test]
    async fn the_quit_token_header_is_never_allowed_cross_origin() {
        // R-SEC-8. A browser refuses to send a header the preflight did not name, so this
        // list is what keeps a token-holding cross-origin client away from the kill switch.
        let state = state();
        let response = respond(
            &state,
            request("OPTIONS", "/write")
                .header(header::ORIGIN, crate::security::EXTENSION_ORIGIN)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "x-curio-quit-token")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        let allowed = header_of(&response, header::ACCESS_CONTROL_ALLOW_HEADERS)
            .expect("preflight names its allowed headers");
        assert!(!allowed.to_ascii_lowercase().contains("quit"), "{allowed}");
    }

    #[tokio::test]
    async fn a_hostile_origin_is_never_granted_access() {
        // The preflight may answer, but it must not carry permission — and the real request
        // behind it still dies on the identity check rather than on CORS alone.
        let state = state();
        let preflight = respond(
            &state,
            request("OPTIONS", "/read")
                .header(header::ORIGIN, "https://evil.example")
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(
            header_of(&preflight, header::ACCESS_CONTROL_ALLOW_ORIGIN),
            None
        );

        let real = send(
            &state,
            request("GET", "/read")
                .header(header::ORIGIN, "https://evil.example")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(real, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn the_allowed_origin_is_echoed_never_a_wildcard() {
        // A wildcard on a loopback daemon holding a bearer token and an API key would let
        // any page on the internet read the library.
        let state = state();
        let response = respond(
            &state,
            request("GET", "/read")
                .header(header::ORIGIN, crate::security::EXTENSION_ORIGIN)
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        let allowed = header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN);
        assert_eq!(allowed.as_deref(), Some(crate::security::EXTENSION_ORIGIN));
        assert_ne!(allowed.as_deref(), Some("*"));
        // Without Vary a shared cache could hand one origin's answer to another.
        assert_eq!(
            header_of(&response, header::VARY).as_deref(),
            Some(header::ORIGIN.as_str())
        );
    }

    #[tokio::test]
    async fn no_credential_is_a_401_that_means_re_bootstrap() {
        let state = state();
        let response = send(
            &state,
            request("GET", "/read")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn a_rebound_host_is_refused_before_anything_else() {
        // R-SEC-6. `evil.example` resolves to 127.0.0.1, but the browser still sends the
        // attacker's name in Host — and the refusal must happen before the credential
        // check, so a rebinding attempt cannot even learn whether its token was wrong.
        let state = state();
        let response = send(
            &state,
            HttpRequest::builder()
                .method("GET")
                .uri("/read")
                .header(header::HOST, "evil.example")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_hostile_origin_is_refused() {
        let state = state();
        let response = send(
            &state,
            request("GET", "/read")
                .header(header::ORIGIN, "https://evil.example")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_cross_site_fetch_is_refused() {
        let state = state();
        let response = send(
            &state,
            request("GET", "/read")
                .header("sec-fetch-site", "cross-site")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_client_that_sends_no_sec_fetch_site_is_not_punished() {
        // R-SEC-12 is defence in depth. curl, the MCP stdio proxy, and every CLI consumer
        // omit the header, and treating absence as hostile would break all of them.
        let state = state();
        let response = send(
            &state,
            request("GET", "/read")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::OK);
    }

    #[tokio::test]
    async fn pausing_refuses_a_write_and_says_when_to_retry() {
        let state = state();
        state.set_paused(true);

        let response = app(state.clone())
            .oneshot(
                request("POST", "/write")
                    .header(
                        header::AUTHORIZATION,
                        format!("Bearer {}", state.token().expose()),
                    )
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(response.headers().contains_key(header::RETRY_AFTER));
    }

    #[tokio::test]
    async fn pausing_leaves_reading_alone() {
        // D25 / ARCH-08 break #10. A paused library you can still browse is strictly
        // better than a dead one, and the blanket short-circuit was never implemented.
        let state = state();
        state.set_paused(true);

        let response = send(
            &state,
            request("GET", "/read")
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response, StatusCode::OK);
    }

    #[tokio::test]
    async fn a_paused_write_from_a_rebound_host_learns_nothing_about_pausing() {
        // The ordering test. Identity is checked first, so a hostile caller receives 403 —
        // not the 503 that would tell it the app exists and is merely paused.
        let state = state();
        state.set_paused(true);

        let response = app(state.clone())
            .oneshot(
                HttpRequest::builder()
                    .method("POST")
                    .uri("/write")
                    .header(header::HOST, "evil.example")
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("response");

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }
}
