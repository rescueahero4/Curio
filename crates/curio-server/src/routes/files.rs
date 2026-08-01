//! The two serve jails: item media and project static hosting (R-SEC-9).
//!
//! Both follow the same three steps, in order:
//!
//! 1. resolve the requested path against the jail root, with `..` and symlinks collapsed;
//! 2. reject anything that escapes;
//! 3. for `/p/*`, apply `projectServeRefusal` to the **resolved** name.
//!
//! Judging the resolved target rather than the raw string is the whole lesson of the
//! Bun-era leak: a raw-string check passed while the resolved path landed on `curio.lock`
//! and served the quit token to any page that asked (Inventory §10.4). Run state has since
//! moved out of the data root entirely (R-DA-2), and the refusal list still names those
//! files — belt and braces, because the cost is a string comparison and the precedent is a
//! real exfiltration.

use std::path::{Component, Path, PathBuf};

use axum::extract::{Path as UrlPath, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};

use crate::state::AppState;

/// `GET /files/items/:id/:file` — an item's screenshot, thumbnail, or sidecar.
pub async fn item_file(
    State(state): State<AppState>,
    UrlPath((id, file)): UrlPath<(String, String)>,
) -> Response {
    let jail = curio_core::paths::item_dir(state.data_root(), &id);
    let Some(target) = resolve(&jail, &file) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    match std::fs::read(&target) {
        Ok(bytes) => serve_bytes(&target, bytes),
        Err(_) if is_derivative(&file) => {
            // The parity quirk, kept deliberately (Inventory §10.20): a missing derivative
            // falls back to the full screenshot, and **only** when the requested name says
            // which derivative it wanted. Image processing is optional, so a library
            // assessed without it has no thumbnails at all — and a grid of broken images
            // would look like data loss rather than a missing optional dependency.
            //
            // `detail` rides the same rule, which is what lets the item page ask for a
            // derivative that no row records and that older items never had.
            match std::fs::read(jail.join("screenshot.png")) {
                Ok(bytes) => serve_bytes(Path::new("screenshot.png"), bytes),
                Err(_) => StatusCode::NOT_FOUND.into_response(),
            }
        }
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// `GET /p/:id/*path` — a project's files, jailed and filtered (FR-18).
pub async fn project_file(
    State(state): State<AppState>,
    UrlPath((id, rest)): UrlPath<(String, String)>,
) -> Response {
    let project = state.with_db(|db| curio_db::projects::get(db.conn(), &id));
    let Ok(Some(project)) = project else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let jail = PathBuf::from(&project.path);
    let requested = if rest.is_empty() {
        "index.html"
    } else {
        rest.as_str()
    };
    let Some(target) = resolve(&jail, requested) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    // Step 3, on the resolved name. Every component is checked, not only the last: a
    // request for `assets/../.secrets.json` resolves to a refused name, and a request for
    // `skills/anything` walks a refused directory.
    if is_refused(&jail, &target) {
        return (StatusCode::FORBIDDEN, "that file is not served").into_response();
    }

    if target.is_dir() {
        let entry = if rest.is_empty() {
            "index.html".to_owned()
        } else {
            format!("{}/index.html", rest.trim_end_matches('/'))
        };
        return match std::fs::read(target.join("index.html")) {
            Ok(bytes) => serve_project_bytes(Path::new("index.html"), bytes, &id, &entry),
            Err(_) => (StatusCode::NOT_FOUND, "no index.html in that folder").into_response(),
        };
    }

    match std::fs::read(&target) {
        Ok(bytes) => serve_project_bytes(&target, bytes, &id, requested),
        Err(_) => StatusCode::NOT_FOUND.into_response(),
    }
}

/// Whether a requested name is a derived copy of the screenshot, and so may fall back to it.
///
/// Matched on the name rather than an exact filename because that is the contract the
/// ingest writes against, and it keeps the two ends from having to agree on an extension.
fn is_derivative(file: &str) -> bool {
    let lowered = file.to_ascii_lowercase();
    lowered.contains("thumb") || lowered.contains("detail")
}

/// Resolve `requested` inside `jail`, or `None` if it escapes.
///
/// The traversal is collapsed **lexically first** so a path that never existed still cannot
/// escape — `canonicalize` fails on a missing file, and falling back to the raw path at that
/// point is exactly how a jail springs a leak. Only then is the real path canonicalized, to
/// catch a symlink pointing outside.
fn resolve(jail: &Path, requested: &str) -> Option<PathBuf> {
    let mut resolved = jail.to_path_buf();
    for component in Path::new(requested).components() {
        match component {
            Component::Normal(part) => resolved.push(part),
            // Both are refused rather than normalised: a client has no legitimate reason to
            // send either, and silently collapsing them means the log never records the
            // attempt.
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
            Component::CurDir => {}
        }
    }

    let real_jail = jail.canonicalize().unwrap_or_else(|_| jail.to_path_buf());
    match resolved.canonicalize() {
        Ok(real) if real.starts_with(&real_jail) => Some(real),
        // The file does not exist yet, or cannot be stat'ed. The lexical walk above already
        // proved the path stays inside, so returning it lets the caller produce an honest
        // 404 rather than a confusing 403.
        Err(_) => Some(resolved),
        Ok(_) => None,
    }
}

/// Whether any component of the resolved target, **relative to the jail**, is a refused
/// name (Inventory §10.4).
///
/// Relative is load-bearing twice over. Judging the absolute path would refuse every
/// project living under a directory that happens to be dot-prefixed or named `items` —
/// and, worse, it would make the answer depend on where the user keeps their files.
///
/// The jail is canonicalized before the comparison because [`resolve`] canonicalized the
/// target; on Windows that adds a `\\?\` prefix, so an uncanonicalized jail never matches.
/// If the strip still fails the answer is **refuse**: a path this function cannot place
/// relative to the jail is one it cannot vouch for.
fn is_refused(jail: &Path, target: &Path) -> bool {
    let real_jail = jail.canonicalize().unwrap_or_else(|_| jail.to_path_buf());
    let Ok(relative) = target
        .strip_prefix(&real_jail)
        .or_else(|_| target.strip_prefix(jail))
    else {
        return true;
    };

    relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => part.to_str(),
            _ => None,
        })
        .any(curio_core::paths::is_refused_project_file)
}

/// A project file, with the variant switcher appended if it is a page.
///
/// Separate from [`serve_bytes`] so that item media — a screenshot, a sidecar — can never be
/// rewritten by a rule written for project pages. What gets appended, and when, is
/// [`super::switcher::inject`].
fn serve_project_bytes(path: &Path, bytes: Vec<u8>, id: &str, entry: &str) -> Response {
    serve_bytes(path, super::switcher::inject(path, bytes, id, entry))
}

fn serve_bytes(path: &Path, bytes: Vec<u8>) -> Response {
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    (
        [
            (header::CONTENT_TYPE, mime.as_ref()),
            // Private, because the response is a user's own screenshot behind a session
            // cookie; sixty seconds, because a re-assessment can replace the file under a
            // URL that does not change.
            (header::CACHE_CONTROL, "private, max-age=60"),
        ],
        bytes,
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn jail() -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let jail = dir.path().join("project");
        std::fs::create_dir_all(jail.join("assets")).expect("mkdir");
        std::fs::write(jail.join("index.html"), "<h1>hi</h1>").expect("write");
        std::fs::write(jail.join("assets").join("app.js"), "console.log(1)").expect("write");
        std::fs::write(dir.path().join("library.db"), "secret").expect("write");
        (dir, jail)
    }

    #[test]
    fn an_ordinary_file_resolves() {
        let (_dir, jail) = jail();
        let resolved = resolve(&jail, "assets/app.js").expect("resolved");

        assert!(resolved.ends_with("app.js"));
        assert!(!is_refused(&jail, &resolved));
    }

    #[test]
    fn a_traversal_escapes_nothing() {
        // The leak precedent: a raw-string check passes while the resolved path lands on
        // the lock file (Inventory §10.4).
        let (_dir, jail) = jail();

        assert!(resolve(&jail, "../library.db").is_none());
        assert!(resolve(&jail, "assets/../../library.db").is_none());
        assert!(resolve(&jail, "../../etc/passwd").is_none());
    }

    #[test]
    fn an_absolute_path_is_refused() {
        let (_dir, jail) = jail();

        assert!(resolve(&jail, "/etc/passwd").is_none());
        assert!(resolve(&jail, "C:\\Windows\\System32\\config\\SAM").is_none());
    }

    #[test]
    fn a_path_that_does_not_exist_still_stays_inside() {
        // `canonicalize` fails on a missing file, and falling back to the raw path at that
        // point is exactly how a jail springs a leak — so the lexical walk has to be the
        // thing that decides.
        let (_dir, jail) = jail();
        let resolved = resolve(&jail, "never/written.txt").expect("inside");

        assert!(resolved.starts_with(&jail));
        assert!(resolve(&jail, "../never-written.txt").is_none());
    }

    #[test]
    fn reserved_names_are_refused_wherever_they_appear() {
        let (_dir, jail) = jail();

        for requested in [
            "library.db",
            "library.db-wal",
            "config.json",
            ".secrets.json",
            "curio.lock",
            "runtime.json",
            "skills/visual-assessment.md",
            "items/01A/screenshot.png",
            "prompts/01P.md",
        ] {
            let resolved = resolve(&jail, requested).expect("inside the jail");
            assert!(
                is_refused(&jail, &resolved),
                "{requested} must not be served"
            );
        }
    }

    #[test]
    fn refusal_survives_a_traversal_that_stays_inside() {
        // `assets/../library.db` never leaves the jail, so the traversal check does not
        // catch it — the refusal list, applied to the resolved name, is what does.
        let (_dir, jail) = jail();

        assert!(
            resolve(&jail, "assets/../library.db")
                .is_none_or(|resolved| is_refused(&jail, &resolved))
        );
    }

    #[test]
    fn a_jail_inside_a_dot_prefixed_directory_still_serves_its_own_files() {
        // The bug this test exists for: judging the absolute path made the answer depend
        // on where the user keeps their files. A project under `~/.local/work/site` would
        // have refused every file it contained.
        let dir = tempfile::tempdir().expect("tempdir");
        let jail = dir.path().join(".hidden").join("site");
        std::fs::create_dir_all(&jail).expect("mkdir");
        std::fs::write(jail.join("index.html"), "<h1>hi</h1>").expect("write");

        let resolved = resolve(&jail, "index.html").expect("inside");
        assert!(!is_refused(&jail, &resolved));
    }

    #[test]
    fn a_target_that_cannot_be_placed_relative_to_the_jail_is_refused() {
        // Fail closed. A path this function cannot locate is one it cannot vouch for.
        let (_dir, jail) = jail();
        assert!(is_refused(&jail, Path::new("/somewhere/else/app.js")));
    }

    #[test]
    fn dotfiles_are_refused() {
        let (_dir, jail) = jail();
        let resolved = resolve(&jail, ".env").expect("inside");

        assert!(is_refused(&jail, &resolved));
    }

    #[test]
    fn a_project_file_that_merely_shares_a_prefix_is_served() {
        // `library.js` is a perfectly ordinary file in a project, and refusing it would
        // break the site being previewed.
        let (_dir, jail) = jail();
        let resolved = resolve(&jail, "library.js").expect("inside");

        assert!(!is_refused(&jail, &resolved));
    }

    #[test]
    fn only_derived_copies_fall_back_to_the_screenshot() {
        // The fallback is what lets the item page request `detail.jpg` for an item captured
        // before that derivative existed. It must stay narrow: an arbitrary missing file
        // answering with the screenshot would hide real 404s.
        assert!(is_derivative("thumb.jpg"));
        assert!(is_derivative("detail.jpg"));
        assert!(is_derivative("DETAIL.JPG"));

        assert!(!is_derivative("screenshot.png"));
        assert!(!is_derivative("item.md"));
        assert!(!is_derivative("anything-else.png"));
    }

    #[test]
    fn a_current_directory_segment_is_harmless() {
        let (_dir, jail) = jail();
        let resolved = resolve(&jail, "./assets/./app.js").expect("inside");

        assert!(resolved.ends_with("app.js"));
    }
}
