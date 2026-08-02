//! Telling the browser what the identity rules already permit.
//!
//! [`origin`](super::origin) decides whether an origin *may* call; this decides whether the
//! browser is *told* so. They are different jobs, and having only the first is why the
//! extension could complete a native-messaging handshake, show itself connected, and still
//! fail every capture with "Failed to fetch": the request was allowed and the answer was
//! unreadable.
//!
//! ## Two rules this must never break
//!
//! * **Never `*`.** The allowed set is a specific trio (R-SEC-7), and a wildcard on a
//!   loopback daemon holding a bearer token and an API key would let any page on the
//!   internet read the library.
//! * **Answer preflights before the credential check.** A CORS preflight carries no
//!   credentials — the spec forbids it — so a preflight that must authenticate can never
//!   succeed. This layer therefore sits outside [`authenticate`](super::guard::authenticate)
//!   and short-circuits `OPTIONS` itself. That is not a hole: a preflight reveals only which
//!   methods and headers are permitted, and the real request behind it still passes the full
//!   stack.

use axum::extract::Request;
use axum::http::{Method, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};

use super::guard::header_str;
use super::origin_is_allowed;

/// Headers a cross-origin caller may send (R-SEC-8, Inventory §1).
///
/// **`x-curio-quit-token` is absent, and must stay absent.** A browser refuses to send a
/// header the preflight did not allow, so leaving it out is what keeps a token-holding
/// cross-origin client away from the kill switch. Adding it here would hand every paired
/// client a way to stop the app — the exact escalation R-SEC-8 exists to prevent.
const ALLOWED_REQUEST_HEADERS: &str = "authorization, content-type";

/// Methods the API answers. Advisory to the browser; routing and the credential check remain
/// the enforcement, so naming a method here grants nothing the router would otherwise refuse.
const ALLOWED_METHODS: &str = "GET, POST, PATCH, DELETE, OPTIONS";

/// How long a browser may cache a preflight. Ten minutes turns the extension's per-capture
/// preflight into a once-per-session one without outliving a run's token.
const PREFLIGHT_MAX_AGE: &str = "600";

/// Grant a validated origin the permission the identity rules already imply.
pub async fn grant(request: Request, next: Next) -> Response {
    let headers = request.headers();
    let host = header_str(headers, header::HOST).map(str::to_owned);
    let origin = header_str(headers, header::ORIGIN).map(str::to_owned);

    // Re-validated here rather than trusted from `identity`, so this layer is correct
    // wherever it is mounted and cannot be made unsafe by a future reordering.
    let granted = origin.filter(|origin| {
        !origin.is_empty() && origin != "null" && origin_is_allowed(Some(origin), host.as_deref())
    });

    let is_preflight = request.method() == Method::OPTIONS;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::{EXTENSION_ORIGIN, RuntimeToken, guard};
    use crate::state::AppState;
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

    /// The real stack, in the real order: cors outside identity outside authenticate.
    fn app(state: AppState) -> Router {
        Router::new()
            .route("/read", get(|| async { "read" }))
            .route("/write", post(|| async { "written" }))
            .layer(axum::middleware::from_fn_with_state(
                state.clone(),
                guard::authenticate,
            ))
            .layer(axum::middleware::from_fn(guard::identity))
            .layer(axum::middleware::from_fn(grant))
            .with_state(state)
    }

    fn request(method: &str, path: &str) -> axum::http::request::Builder {
        HttpRequest::builder()
            .method(method)
            .uri(path)
            .header(header::HOST, "127.0.0.1:51234")
    }

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

    #[tokio::test]
    async fn a_preflight_is_answered_without_a_credential() {
        // The regression this exists for: `authenticate` used to 401 the preflight, and a
        // preflight carries no credentials by spec — so the extension could complete a
        // native-messaging handshake and still fail every capture with "Failed to fetch".
        let state = state();
        let response = respond(
            &state,
            request("OPTIONS", "/write")
                .header(header::ORIGIN, EXTENSION_ORIGIN)
                .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
                .header(header::ACCESS_CONTROL_REQUEST_HEADERS, "authorization")
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN).as_deref(),
            Some(EXTENSION_ORIGIN)
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
                .header(header::ORIGIN, EXTENSION_ORIGIN)
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

        let real = respond(
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

        assert_eq!(real.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn the_allowed_origin_is_echoed_never_a_wildcard() {
        // A wildcard on a loopback daemon holding a bearer token and an API key would let
        // any page on the internet read the library.
        let state = state();
        let response = respond(
            &state,
            request("GET", "/read")
                .header(header::ORIGIN, EXTENSION_ORIGIN)
                .header(
                    header::AUTHORIZATION,
                    format!("Bearer {}", state.token().expose()),
                )
                .body(Body::empty())
                .expect("request"),
        )
        .await;

        let allowed = header_of(&response, header::ACCESS_CONTROL_ALLOW_ORIGIN);
        assert_eq!(allowed.as_deref(), Some(EXTENSION_ORIGIN));
        assert_ne!(allowed.as_deref(), Some("*"));
        // Without Vary a shared cache could hand one origin's answer to another.
        assert_eq!(
            header_of(&response, header::VARY).as_deref(),
            Some(header::ORIGIN.as_str())
        );
    }
}
