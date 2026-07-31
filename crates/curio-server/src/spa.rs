//! Serving the dashboard from inside the binary.
//!
//! The SPA is built by Vite and embedded into the executable, so there is **one origin**
//! and one file to ship: no CORS, no second web server, no bundled webview (D4, R-FE-4).
//! The browser is the screen.
//!
//! Two rules the fallback must obey (R-BE-8):
//!
//! * `/mcp` is routed **above** this catch-all. A disabled or unknown `/mcp` request must
//!   receive a JSON-RPC error, never SPA HTML — an agent that gets a page of markup where
//!   it expected a protocol frame reports something incomprehensible (Inventory §10.6).
//! * The fallback **404s** `/api`, `/files`, `/p/` and build-asset paths rather than
//!   serving `index.html`. Returning the shell for a missing asset turns a 404 into a
//!   silent, much harder-to-diagnose failure: the browser receives HTML where it asked for
//!   JavaScript.
//!
//! ## Not yet: the debug-mode Vite proxy
//!
//! R-DEL-3 also asks that debug builds proxy to the Vite dev server so frontend iteration
//! needs no Rust rebuild. That is **not implemented in this scaffold** — the dev loop
//! currently runs the SPA on its own Vite port. It lands with P3, when there is a
//! dashboard whose iteration speed the proxy would actually protect. Recorded here rather
//! than left to be discovered.

use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use rust_embed::Embed;

/// The built dashboard.
///
/// The directory is created by `build.rs` if it does not exist, so a fresh clone compiles
/// without having run `npm run build` first.
#[derive(Embed)]
#[folder = "../../web/spa/dist"]
struct Assets;

/// Path prefixes the SPA fallback must never answer for.
const NEVER_FALL_BACK: &[&str] = &["/api", "/files", "/p", "/mcp", "/ws", "/health", "/assets"];

/// Whether `path` should 404 rather than receive the SPA shell.
///
/// Matching is on **path segments**, not raw prefixes. A naive `starts_with("/api")` also
/// claims `/apiary`, and `/p` would swallow `/pair` — which is a real dashboard route
/// (R-FE-2), so the naive version would 404 the pairing page.
#[must_use]
pub fn is_reserved(path: &str) -> bool {
    let path = path.trim_end_matches('/');
    NEVER_FALL_BACK.iter().any(|reserved| {
        // `/assets` is in the list because a missing build asset must 404 rather than
        // receive HTML. The browser asked for JavaScript; handing it `<!doctype html>`
        // produces a parse error three layers away from the actual cause.
        path == *reserved || path.starts_with(&format!("{reserved}/"))
    })
}

/// Serve `path`, falling back to `index.html` for client-side routes.
#[must_use]
pub fn serve(path: &str) -> Response {
    let trimmed = path.trim_start_matches('/');

    if let Some(asset) = Assets::get(trimmed) {
        let mime = mime_guess::from_path(trimmed).first_or_octet_stream();
        return ([(header::CONTENT_TYPE, mime.as_ref())], asset.data).into_response();
    }

    if is_reserved(path) {
        return StatusCode::NOT_FOUND.into_response();
    }

    match Assets::get("index.html") {
        Some(shell) => (
            [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
            shell.data,
        )
            .into_response(),
        None => not_built(),
    }
}

/// Whether the dashboard was built into this binary.
#[must_use]
pub fn is_built() -> bool {
    Assets::get("index.html").is_some()
}

/// What to show when the binary carries no dashboard.
///
/// A plain, honest page rather than a 500. This is the state of a fresh clone before
/// `npm run build`, and a developer meeting it should be told what to run — not left
/// wondering whether the server is broken.
fn not_built() -> Response {
    (
        StatusCode::SERVICE_UNAVAILABLE,
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        concat!(
            "<!doctype html><meta charset=utf-8>",
            "<title>Curio — dashboard not built</title>",
            "<style>body{font:16px/1.6 system-ui;margin:6rem auto;max-width:34rem;padding:0 1.5rem}",
            "code{background:#0001;padding:.15em .4em;border-radius:.25em}</style>",
            "<h1>The dashboard has not been built</h1>",
            "<p>The server is running — this binary just does not carry the dashboard assets yet.</p>",
            "<p>Build them, then restart:</p>",
            "<pre><code>npm --prefix web/spa install\n",
            "npm --prefix web/spa run build\n",
            "cargo run</code></pre>",
            "<p><code>GET /health</code> works regardless.</p>",
        ),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn api_paths_never_receive_the_shell() {
        // Serving HTML here turns a routing mistake into a client-side parse error with
        // no trace of the real cause.
        for path in [
            "/api",
            "/api/items",
            "/api/events",
            "/files/items/01A/screenshot.png",
            "/p/01B/index.html",
            "/health",
        ] {
            assert!(is_reserved(path), "{path}");
        }
    }

    #[test]
    fn mcp_never_receives_the_shell() {
        // Inventory §10.6. An agent expecting a JSON-RPC frame and receiving markup
        // reports something incomprehensible to the user.
        assert!(is_reserved("/mcp"));
        assert!(is_reserved("/mcp/"));
    }

    #[test]
    fn missing_build_assets_404_rather_than_falling_back() {
        assert!(is_reserved("/assets/index-a1b2c3.js"));
        assert!(is_reserved("/assets/index-a1b2c3.css"));
    }

    #[test]
    fn client_side_routes_do_fall_back() {
        // These are real dashboard routes with no file behind them; the shell must answer
        // so the router can take over (R-FE-2).
        for path in [
            "/",
            "/items/01J000000000000000000000",
            "/projects",
            "/prompts",
            "/prompts/01J000000000000000000000",
            "/vocabulary",
            "/settings",
            "/pair",
        ] {
            assert!(!is_reserved(path), "{path}");
        }
    }

    #[test]
    fn a_path_merely_starting_with_the_same_letters_is_not_reserved() {
        // Matching must be on segments. `/pair` is the case that actually bites: it is a
        // real route (R-FE-2), and a raw `starts_with("/p")` would 404 the pairing page —
        // breaking the documented fallback for unpacked extension installs.
        assert!(!is_reserved("/pair"));
        assert!(!is_reserved("/apiary"));
        assert!(!is_reserved("/healthy-habits"));
        assert!(!is_reserved("/websocket-guide"));
    }

    #[test]
    fn an_unbuilt_dashboard_explains_itself() {
        if is_built() {
            return;
        }
        let response = serve("/");
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
}
