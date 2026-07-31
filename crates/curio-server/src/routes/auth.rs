//! Getting a session: nonce in, cookie out (R-BE-11, R-SEC-5, D22).
//!
//! The tray opens `/?t=<nonce>` and the SPA trades that nonce for an HttpOnly cookie. The
//! whole mechanism exists because **a token must never appear in a URL**: query strings
//! survive in browser history, in screenshots, and in every log that records a request
//! line. A nonce is worth 30 seconds and authorizes exactly one thing.
//!
//! `POST /api/pair/authorize` is the fourth and last sanctioned distribution path
//! (R-SEC-3d): the click-gated `/pair` handoff for unpacked extension installs, where the
//! native-messaging bootstrap is unavailable. It has no credential of its own — the Host and
//! Origin checks, `Sec-Fetch-Site`, and an explicit user click stand in for one.

use axum::Json;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde::{Deserialize, Serialize};

use crate::security::session;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct ExchangeRequest {
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct NonceResponse {
    pub nonce: String,
}

#[derive(Debug, Serialize)]
pub struct SessionResponse {
    pub version: String,
    /// Whether mutations are currently refused. The dashboard renders this as a banner,
    /// never as an error (R-FE-8).
    pub paused: bool,
}

/// `POST /api/auth/nonce` — mint a launch nonce. Authenticated.
pub async fn mint(State(state): State<AppState>) -> Json<NonceResponse> {
    Json(NonceResponse {
        nonce: state.mint_nonce(),
    })
}

/// `POST /api/auth/exchange` — trade a nonce for the session cookie. Unauthenticated by
/// design: the nonce *is* the credential, and it is single-use and short-lived.
pub async fn exchange(
    State(state): State<AppState>,
    Json(body): Json<ExchangeRequest>,
) -> Response {
    if !state.consume_nonce(&body.nonce) {
        // Expired, already used, or invented. All three are the same answer to the client —
        // the SPA shows "Open Curio from the tray" — and distinguishing them would tell a
        // replay attempt which of its guesses was closest.
        return (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({
                "error": "nonce_rejected",
                "message": "That launch link has expired. Open the dashboard from the tray again.",
            })),
        )
            .into_response();
    }

    (
        [(
            header::SET_COOKIE,
            session::set_cookie(state.token().expose()),
        )],
        Json(SessionResponse {
            version: state.version().to_owned(),
            paused: state.is_paused(),
        }),
    )
        .into_response()
}

/// `GET /api/auth/session` — does the cookie we already hold still work?
///
/// The SPA calls this on load when there is no nonce in the URL. A 401 means the app
/// restarted (D21) and the answer is the no-session screen, not an error page.
pub async fn probe(State(state): State<AppState>) -> Json<SessionResponse> {
    Json(SessionResponse {
        version: state.version().to_owned(),
        paused: state.is_paused(),
    })
}

/// `POST /api/auth/logout` — drop the session cookie.
pub async fn logout() -> Response {
    (
        [(header::SET_COOKIE, session::clear_cookie())],
        StatusCode::NO_CONTENT,
    )
        .into_response()
}

#[derive(Debug, Serialize)]
pub struct PairResponse {
    pub token: String,
}

/// `POST /api/pair/authorize` — hand the per-run token to the `/pair` page (D11, R-SEC-3d).
///
/// POST only. The old implementation's `GET` returned 404 for the same reason this does:
/// a URL that hands out a token can be embedded in an `<img>` tag, and a GET is exactly what
/// a page can cause without the user doing anything.
pub async fn pair_authorize(State(state): State<AppState>, headers: HeaderMap) -> Response {
    // Belt and braces on top of the identity layer. `Sec-Fetch-Site: same-origin` is what a
    // real click on our own page produces; `none` is a top-level navigation, which cannot
    // be this. Absence is tolerated everywhere else (R-SEC-12) but not here, because this
    // is the one route with no credential at all.
    let site = headers
        .get("sec-fetch-site")
        .and_then(|value| value.to_str().ok());
    if site.is_some_and(|site| site != "same-origin") {
        return (
            StatusCode::FORBIDDEN,
            Json(serde_json::json!({
                "error": "forbidden",
                "message": "Pairing must be started from Curio's own pairing page.",
            })),
        )
            .into_response();
    }

    Json(PairResponse {
        token: state.token().expose().to_owned(),
    })
    .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::RuntimeToken;
    use curio_core::config::Config;

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

    fn set_cookie_of(response: &Response) -> Option<String> {
        response
            .headers()
            .get(header::SET_COOKIE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned)
    }

    #[tokio::test]
    async fn a_fresh_nonce_mints_a_session() {
        let state = state();
        let nonce = mint(State(state.clone())).await.0.nonce;

        let response = exchange(State(state.clone()), Json(ExchangeRequest { nonce })).await;

        assert_eq!(response.status(), StatusCode::OK);
        let cookie = set_cookie_of(&response).expect("a session cookie");
        assert!(cookie.contains(state.token().expose()));
        assert!(cookie.contains("HttpOnly"));
    }

    #[tokio::test]
    async fn a_replayed_nonce_is_rejected() {
        // R-SEC-5: consumption is atomic, so a replay races to a rejection rather than to a
        // second token handout.
        let state = state();
        let nonce = mint(State(state.clone())).await.0.nonce;

        exchange(
            State(state.clone()),
            Json(ExchangeRequest {
                nonce: nonce.clone(),
            }),
        )
        .await;
        let replay = exchange(State(state), Json(ExchangeRequest { nonce })).await;

        assert_eq!(replay.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn an_invented_nonce_is_rejected_without_setting_a_cookie() {
        let response = exchange(
            State(state()),
            Json(ExchangeRequest {
                nonce: "made-up".to_owned(),
            }),
        )
        .await;

        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert!(set_cookie_of(&response).is_none());
    }

    #[tokio::test]
    async fn the_session_probe_reports_the_paused_state() {
        // The dashboard needs this at load: paused is a banner, not an error (R-FE-8).
        let state = state();
        state.set_paused(true);

        assert!(probe(State(state)).await.0.paused);
    }

    #[tokio::test]
    async fn pairing_hands_over_the_runtime_token_on_a_same_origin_click() {
        let state = state();
        let mut headers = HeaderMap::new();
        headers.insert("sec-fetch-site", "same-origin".parse().expect("header"));

        let response = pair_authorize(State(state.clone()), headers).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn pairing_refuses_a_cross_site_caller() {
        // This is the one route with no credential at all, so the click gate and the
        // header are the whole of its authorization.
        let mut headers = HeaderMap::new();
        headers.insert("sec-fetch-site", "cross-site".parse().expect("header"));

        let response = pair_authorize(State(state()), headers).await;

        assert_eq!(response.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn pairing_still_answers_a_client_that_sends_no_sec_fetch_site() {
        // D0 has not yet confirmed the header is sent on loopback fetches from every
        // supported browser (ARCH-06 OQ-1). Until it is, absence must not break the
        // documented fallback for unpacked installs.
        let response = pair_authorize(State(state()), HeaderMap::new()).await;

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn logging_out_clears_the_cookie() {
        let response = logout().await;
        assert!(
            set_cookie_of(&response)
                .expect("cookie")
                .contains("Max-Age=0")
        );
    }
}
