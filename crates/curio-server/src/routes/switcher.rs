//! The variant switcher: the script itself, and the one tag that puts it on a page.
//!
//! A project generated from a Curio prompt is several versions of one design in `v1/`, `v2/`,
//! `v3/`. Curio serves them through the jail in [`super::files`] and opens the newest — which
//! left the others reachable only through the file manager. This is what makes them reachable
//! from each other.
//!
//! It is injected into the **response** and never written to disk, and that is the whole
//! design rather than an implementation detail. It means a folder an agent wrote before any of
//! this existed gets the switcher without being regenerated; it means there is one copy of the
//! component instead of one pasted into every page; and it means nothing Curio does here can
//! be left behind in a user's project.

use std::path::Path;

use axum::http::header;
use axum::response::{IntoResponse, Response};

/// `GET /__curio/variant-switcher.js`.
///
/// A plain file rather than an SPA entry point: the SPA's build would give it a hashed name
/// the Rust side then has to discover, and would pull a framework into four kilobytes of
/// vanilla JavaScript whose whole job is to leave no trace on a stranger's page.
pub async fn script() -> Response {
    (
        [
            (header::CONTENT_TYPE, "text/javascript; charset=utf-8"),
            // Deliberately not the sixty seconds `serve_bytes` gives project files. This
            // script ships with Curio, so a cached copy from the previous version is a bar
            // that disagrees with the endpoint it calls. Revalidate; the ETag makes that a
            // 304 in the ordinary case.
            (header::CACHE_CONTROL, "private, max-age=0, must-revalidate"),
        ],
        [(header::ETAG, format!("\"{}\"", curio_core::VERSION))],
        include_str!("../../assets/variant-switcher.js"),
    )
        .into_response()
}

/// Append the switcher's `<script>` to an HTML response, or hand the bytes back unchanged.
///
/// Three gates, in order, and each one declines rather than guesses:
///
/// 1. **The extension.** `.html`/`.htm` only. Not `mime_guess`, which would sweep in `.xhtml`
///    — parsed as strict XML, where a stray tag is a fatal error rather than a warning.
/// 2. **UTF-8.** A page in another encoding is served as it was. Appending ASCII to any
///    ASCII-superset encoding is safe, so this is really a guard against UTF-16.
/// 3. **A closing tag.** Before `</body>`, else before `</html>`, else appended — a trailing
///    script is reparented into the body by every browser, so the last case is correct rather
///    than merely tolerable.
///
/// `rfind`, not `find`: a page with a literal `</body>` inside a `<pre>` block would otherwise
/// take the script into the middle of its own displayed source.
pub fn inject(path: &Path, bytes: Vec<u8>, id: &str, entry: &str) -> Vec<u8> {
    let is_page = path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            extension.eq_ignore_ascii_case("html") || extension.eq_ignore_ascii_case("htm")
        });
    if !is_page {
        return bytes;
    }

    let Ok(text) = std::str::from_utf8(&bytes) else {
        return bytes;
    };

    let tag = format!(
        "<script defer src=\"/__curio/variant-switcher.js\" data-curio-project=\"{}\" data-curio-entry=\"{}\"></script>",
        escape_attribute(id),
        escape_attribute(entry),
    );

    // Lowercased with `to_ascii_lowercase`, which is byte-length preserving — so an index
    // found in the copy is valid in the original. A Unicode-aware lowercase is not.
    let haystack = text.to_ascii_lowercase();
    let at = haystack
        .rfind("</body>")
        .or_else(|| haystack.rfind("</html>"))
        .unwrap_or(text.len());

    let mut out = Vec::with_capacity(bytes.len() + tag.len());
    out.extend_from_slice(&bytes[..at]);
    out.extend_from_slice(tag.as_bytes());
    out.extend_from_slice(&bytes[at..]);
    out
}

/// Escape a value going into a double-quoted HTML attribute.
///
/// The id is a ULID and the entry is a jail-resolved relative path, so neither *should* carry
/// anything interesting — but the entry ends in a filename the user chose, and a filename is
/// not a place to start trusting input.
fn escape_attribute(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn html(body: &str) -> String {
        let out = inject(
            Path::new("index.html"),
            body.as_bytes().to_vec(),
            "01J",
            "v2",
        );
        String::from_utf8(out).expect("still utf-8")
    }

    #[test]
    fn the_switcher_lands_just_inside_the_body() {
        let out = html("<html><body><h1>hi</h1></body></html>");

        assert!(out.contains("variant-switcher.js"));
        assert!(
            out.find("variant-switcher.js") < out.find("</body>"),
            "{out}"
        );
    }

    #[test]
    fn a_shouting_closing_tag_counts() {
        let out = html("<HTML><BODY>hi</BODY></HTML>");
        assert!(
            out.find("variant-switcher.js") < out.find("</BODY>"),
            "{out}"
        );
    }

    #[test]
    fn a_page_with_no_body_tag_still_gets_it() {
        // Both fallbacks. A trailing script is reparented into the body by every browser, so
        // the last case is correct rather than merely tolerated.
        assert!(html("<html><h1>hi</h1></html>").contains("variant-switcher.js"));
        assert!(html("<h1>hi</h1>").contains("variant-switcher.js"));
    }

    #[test]
    fn a_page_that_prints_its_own_source_is_not_cut_in_half() {
        // The reason this uses `rfind`: a tutorial page showing `</body>` inside a `<pre>`
        // would otherwise take the script into the middle of its own displayed source.
        let out = html("<body><pre>&lt;/body&gt;</pre><p>after</p></body>");

        assert_eq!(out.matches("variant-switcher.js").count(), 1);
        assert!(
            out.find("<p>after</p>") < out.find("variant-switcher.js"),
            "{out}"
        );
    }

    #[test]
    fn only_pages_are_rewritten() {
        // A stylesheet, a script, an image and an XHTML document — the last because it is
        // parsed as strict XML, where being nearly right is a fatal error.
        for name in ["styles.css", "app.js", "shot.png", "page.xhtml"] {
            let body = b"<body></body>".to_vec();
            let out = inject(Path::new(name), body.clone(), "01J", "v2");
            assert_eq!(out, body, "{name} must be served untouched");
        }
    }

    #[test]
    fn bytes_that_are_not_utf8_are_served_as_they_are() {
        // A UTF-16 page. Guessing at an encoding to inject a toolbar is not worth corrupting
        // someone's document over.
        let body = vec![0xff, 0xfe, b'<', 0x00, b'h', 0x00];
        let out = inject(Path::new("index.html"), body.clone(), "01J", "v2");

        assert_eq!(out, body);
    }

    #[test]
    fn what_goes_into_the_attributes_is_escaped() {
        // The entry ends in a filename the user chose, and a filename is not a place to
        // start trusting input.
        let out = inject(
            Path::new("index.html"),
            b"<body></body>".to_vec(),
            "01J",
            "v2/\"><script>alert(1)</script>.html",
        );
        let out = String::from_utf8(out).expect("utf-8");

        assert!(!out.contains("alert(1)</script>"), "{out}");
        assert!(out.contains("&quot;&gt;&lt;script&gt;"), "{out}");
    }

    #[test]
    fn styles_and_switcher_agree_on_the_palette() {
        // The switcher runs inside a page that has no Curio custom properties to `var()`
        // against, so it is the one file in the codebase that restates the palette by value.
        // Two copies of a design language drift silently — a bar that is *almost* Curio's
        // grey reads as a rendering bug — so the copies are held together by this test
        // rather than by anyone remembering.
        const SWITCHER: &str = include_str!("../../assets/variant-switcher.js");
        const STYLES: &str = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../web/spa/src/styles.css"
        ));

        let mut checked = 0;
        for (at, _) in SWITCHER.match_indices('#') {
            let hex: String = SWITCHER[at + 1..]
                .chars()
                .take(6)
                .take_while(char::is_ascii_hexdigit)
                .collect();
            if hex.len() != 6 {
                continue;
            }
            checked += 1;
            assert!(
                STYLES.contains(&format!("#{hex}")),
                "#{hex} is in the switcher but not in styles.css — one of the two moved"
            );
        }

        assert!(
            checked > 5,
            "the palette scan found almost nothing to check"
        );
    }
}
