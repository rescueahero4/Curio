//! The dashboard's session cookie (D22, R-SEC-5).
//!
//! The nonce exchange sets this and nothing else ever does. Its four attributes each remove
//! a specific failure the previous design had:
//!
//! * **`HttpOnly`** — the token never reaches SPA JavaScript, so an XSS bug in the
//!   dashboard cannot exfiltrate a credential the dashboard does not hold.
//! * **`SameSite=Strict`** — no other site can cause an authenticated request, which is the
//!   browser-level half of the Origin check.
//! * **Session-scoped** (no `Max-Age`) — the token is per-run (D21); persisting the cookie
//!   past the browser's life would only produce 401s that look like breakage.
//! * **`Path=/`** — SSE, the API, and both user-content jails are all same-origin fetches
//!   that must carry it, `<img>` tags included.
//!
//! Deliberately **not** `Secure`: the origin is `http://127.0.0.1`, and a `Secure` cookie
//! would simply never be sent.

use axum::http::{HeaderMap, header};

/// The cookie's name.
pub const COOKIE_NAME: &str = "curio_session";

/// The `Set-Cookie` value the exchange returns.
#[must_use]
pub fn set_cookie(token: &str) -> String {
    format!("{COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite=Strict")
}

/// The value that clears the session.
#[must_use]
pub fn clear_cookie() -> String {
    format!("{COOKIE_NAME}=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0")
}

/// Pull our cookie out of a request's `Cookie` header.
///
/// Parsed by hand rather than with a cookie crate: this is one header, one name, and a
/// split on `;` — and the idle-footprint budget makes every avoidable dependency worth
/// avoiding (R-BE-31).
#[must_use]
pub fn from_cookies(headers: &HeaderMap) -> Option<String> {
    headers
        .get_all(header::COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(';'))
        .filter_map(|pair| pair.trim().split_once('='))
        .find(|(name, _)| *name == COOKIE_NAME)
        .map(|(_, value)| value.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers(cookie: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, cookie.parse().expect("header"));
        headers
    }

    #[test]
    fn the_cookie_is_http_only_and_same_site_strict() {
        // Both are load-bearing: HttpOnly keeps the token out of JavaScript's reach, and
        // SameSite=Strict is the browser-level half of the Origin check.
        let value = set_cookie("secret");

        assert!(value.contains("HttpOnly"), "{value}");
        assert!(value.contains("SameSite=Strict"), "{value}");
    }

    #[test]
    fn the_cookie_does_not_outlive_the_browser_session() {
        // The token is per-run (D21). A persisted cookie would only produce 401s that look
        // like breakage the next morning.
        let value = set_cookie("secret");

        assert!(!value.contains("Max-Age"), "{value}");
        assert!(!value.contains("Expires"), "{value}");
    }

    #[test]
    fn the_cookie_is_not_marked_secure() {
        // The origin is http://127.0.0.1. A Secure cookie would simply never be sent, and
        // the dashboard would authenticate nothing.
        assert!(
            !set_cookie("secret").contains("Secure"),
            "{}",
            set_cookie("secret")
        );
    }

    #[test]
    fn the_cookie_covers_the_whole_origin() {
        // SSE, /api, /files and /p are all same-origin requests that must carry it —
        // including <img src="/files/…">, which cannot add a header.
        assert!(set_cookie("secret").contains("Path=/"));
    }

    #[test]
    fn our_cookie_is_found_among_others() {
        let found = from_cookies(&headers(&format!("other=1; {COOKIE_NAME}=secret; third=3")));
        assert_eq!(found.as_deref(), Some("secret"));
    }

    #[test]
    fn a_cookie_header_without_ours_yields_nothing() {
        assert!(from_cookies(&headers("other=1; third=3")).is_none());
        assert!(from_cookies(&HeaderMap::new()).is_none());
    }

    #[test]
    fn a_similarly_named_cookie_is_not_mistaken_for_ours() {
        // `curio_session_backup` shares our prefix; a `starts_with` check would take it.
        assert!(from_cookies(&headers("curio_session_backup=nope")).is_none());
    }

    #[test]
    fn clearing_expires_the_cookie_immediately() {
        assert!(clear_cookie().contains("Max-Age=0"));
    }
}
