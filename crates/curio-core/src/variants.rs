//! The versions inside one project folder, and the file that names them.
//!
//! A Curio prompt asks for several design directions, each in its own folder — `v1/`, `v2/`,
//! `v3/` — and AI clients write exactly that. Curio catalogues the folder as one project, so
//! everything downstream needs the same two answers: which version to open first, and what
//! all of them are.
//!
//! Both answers live here rather than in the route that first needed one, because they
//! disagree on purpose and that disagreement has to be visible in a single file:
//!
//! - [`front_door`] picks the **newest** version. Someone opening a project means the latest
//!   attempt; showing them `v1` every time would show them their oldest.
//! - [`scan`] lists **ascending**, the order the prompt asked for the directions in and the
//!   order a reader compares them in.
//!
//! Read as two functions in two files those look like a bug, and the fix would be to make one
//! match the other. Both orderings are asserted below.
//!
//! This module touches the filesystem, which is why it is not in [`crate::domain`] — those are
//! serde types that decide nothing and read nothing.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// What an agent writes to name the directions it produced.
///
/// Deliberately **not** a dotfile. `.curio-project` is hidden because Curio mints it and it
/// holds an internal identity; this one is authored content — the names, families and tags a
/// user reads on screen and edits when the model gets one wrong. A file nobody can find in
/// Explorer is a file nobody can correct.
pub const MANIFEST_FILE_NAME: &str = "curio-variants.json";

/// One version folder that can actually be opened.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Variant {
    /// The folder name — `"v2"`, or empty for a project whose `index.html` is at the root.
    pub slug: String,
    /// The path to serve, relative to the project root.
    pub entry: String,
    /// The number in the folder name, which is what orders these. `None` for a root index.
    pub rank: Option<u64>,
}

/// Every version folder under `root`, oldest first.
///
/// A root `index.html` is a project with one version rather than a project with none — the
/// caller should not have to special-case the ordinary single-page case.
#[must_use]
pub fn scan(root: &Path) -> Vec<Variant> {
    if root.join("index.html").is_file() {
        return vec![Variant {
            slug: String::new(),
            entry: "index.html".to_owned(),
            rank: None,
        }];
    }

    let mut found: Vec<(u64, String)> = std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .filter(|entry| entry.path().join("index.html").is_file())
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            // `v3` and `3` both count; anything else is a folder that happens to hold an
            // index.html, which is not the same as a version of the project.
            let digits: String = name.chars().filter(char::is_ascii_digit).collect();
            let looks_numeric = !digits.is_empty()
                && name
                    .chars()
                    .all(|c| c.is_ascii_digit() || c.eq_ignore_ascii_case(&'v'));
            looks_numeric.then(|| digits.parse().ok().map(|number| (number, name)))?
        })
        .collect();

    // Numeric, not lexical: `v10` is newer than `v2`, and sorting as text says otherwise the
    // moment a user reaches double digits.
    found.sort_unstable();
    found
        .into_iter()
        .map(|(rank, name)| Variant {
            entry: format!("{name}/index.html"),
            slug: name,
            rank: Some(rank),
        })
        .collect()
}

/// Where a project's front door is (R-BE-22).
///
/// Root `index.html`, else the **newest** numeric subfolder containing one, else nothing —
/// and nothing is an honest answer. A project with no page to open should say so rather than
/// hand back a path that 404s.
#[must_use]
pub fn front_door(root: &Path) -> String {
    scan(root)
        .pop()
        .map_or_else(String::new, |variant| variant.entry)
}

/// The manifest an agent writes beside the version folders.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Manifest {
    /// The shape this file was written against. An unfamiliar number is read anyway — a
    /// best-effort read of names and tags cannot corrupt anything, and refusing the file
    /// would cost the user their labels over a number they never chose.
    #[serde(default = "manifest_version")]
    pub version: u32,
    #[serde(default)]
    pub variants: Vec<ManifestEntry>,
}

/// What an agent can say about one direction.
///
/// Everything but `folder` is optional, and unknown keys are kept rather than rejected: a
/// model that helpfully adds `"palette"` must not take the whole file down with it.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ManifestEntry {
    /// The version folder this describes, e.g. `"v1"`.
    pub folder: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub family: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub design_type: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

const fn manifest_version() -> u32 {
    1
}

/// What reading the manifest produced. Three outcomes, because the user has to be able to
/// tell "no manifest" from "a manifest with a typo in it" — the second is fixable and the
/// first is not a fault.
#[derive(Debug, Clone)]
pub enum ManifestOutcome {
    Absent,
    Malformed(String),
    Ok(Manifest),
}

/// Read `curio-variants.json` from a project root.
///
/// An unreadable or unparseable file is never an error the caller has to handle as a failure:
/// the version folders are on disk either way, and a switcher that vanished because of a
/// trailing comma would be a worse bug than the trailing comma.
#[must_use]
pub fn read_manifest(root: &Path) -> ManifestOutcome {
    let Ok(text) = std::fs::read_to_string(root.join(MANIFEST_FILE_NAME)) else {
        return ManifestOutcome::Absent;
    };

    match serde_json::from_str::<Manifest>(&text) {
        // An empty list is a file that says nothing, which is what `Absent` already means.
        // Reporting it as present would have callers render "described" chips for nothing.
        Ok(manifest) if manifest.variants.is_empty() => ManifestOutcome::Absent,
        Ok(manifest) => ManifestOutcome::Ok(manifest),
        Err(err) => ManifestOutcome::Malformed(err.to_string()),
    }
}

/// What [`write_manifest`] did, in the terms the caller has to report back.
#[derive(Debug, Clone, Default)]
pub struct Written {
    pub written: usize,
    /// Entries that named nothing openable, so the file does not claim they exist.
    pub skipped: Vec<String>,
}

/// Write `curio-variants.json`, keeping only the entries that name a real version.
///
/// Two things are checked before anything is written, and both are about not trusting a
/// caller that is a language model:
///
/// 1. `folder` must be a **single plain path component**. `../` in a folder name is the one
///    way this function could write a file outside the project it was handed.
/// 2. The folder must exist and hold an `index.html`. A manifest naming a version that is
///    not there would offer the user a link to a 404, which is worse than a missing label.
///
/// Nothing valid means nothing written — an empty manifest is a file that says only that
/// somebody tried.
///
/// # Errors
/// Returns the underlying [`std::io::Error`] if the file cannot be written. Callers should
/// report that rather than treat it as a failure of whatever they were really doing.
pub fn write_manifest(root: &Path, entries: Vec<ManifestEntry>) -> std::io::Result<Written> {
    let (keep, skipped): (Vec<_>, Vec<_>) = entries
        .into_iter()
        .partition(|entry| is_plain_component(&entry.folder) && has_index(root, &entry.folder));

    let mut report = Written {
        written: keep.len(),
        skipped: skipped.into_iter().map(|entry| entry.folder).collect(),
    };

    if keep.is_empty() {
        report.written = 0;
        return Ok(report);
    }

    let manifest = Manifest {
        version: manifest_version(),
        variants: keep,
    };
    let body = serde_json::to_string_pretty(&manifest)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;
    std::fs::write(root.join(MANIFEST_FILE_NAME), body)?;
    Ok(report)
}

fn has_index(root: &Path, folder: &str) -> bool {
    root.join(folder).join("index.html").is_file()
}

/// Whether `name` is one ordinary path segment — no separators, no `..`, no drive letter.
fn is_plain_component(name: &str) -> bool {
    let mut components = Path::new(name).components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree(folders: &[&str], files: &[&str]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("tempdir");
        for folder in folders {
            std::fs::create_dir_all(dir.path().join(folder)).expect("mkdir");
        }
        for file in files {
            std::fs::write(dir.path().join(file), "<h1>hi</h1>").expect("write");
        }
        dir
    }

    #[test]
    fn a_root_index_is_the_front_door() {
        let dir = tree(&[], &["index.html"]);
        assert_eq!(front_door(dir.path()), "index.html");
    }

    #[test]
    fn the_newest_numeric_folder_wins() {
        // AI tools write v1, v2, v3 and the user means the latest. Sorting as text would
        // put v10 before v2 and show them the wrong one as soon as they got to double
        // digits.
        let dir = tree(
            &["v1", "v2", "v10"],
            &["v1/index.html", "v2/index.html", "v10/index.html"],
        );
        assert_eq!(front_door(dir.path()), "v10/index.html");
    }

    #[test]
    fn a_root_index_beats_a_versioned_one() {
        let dir = tree(&["v2"], &["index.html", "v2/index.html"]);
        assert_eq!(front_door(dir.path()), "index.html");
    }

    #[test]
    fn a_non_numeric_folder_is_not_a_version() {
        // `docs/index.html` is documentation, not a version of the project.
        let dir = tree(&["docs"], &["docs/index.html"]);
        assert_eq!(front_door(dir.path()), "");
    }

    #[test]
    fn a_project_with_nothing_to_open_says_so_rather_than_guessing() {
        let dir = tree(&["src"], &["src/main.rs"]);
        assert_eq!(front_door(dir.path()), "");
    }

    #[test]
    fn the_scan_lists_every_version_oldest_first() {
        // The other ordering, and the reason both live in this file: `front_door` opens the
        // newest, the switcher lists them in the order the prompt asked for the directions.
        let dir = tree(
            &["v1", "v2", "v10"],
            &["v1/index.html", "v2/index.html", "v10/index.html"],
        );

        let slugs: Vec<String> = scan(dir.path()).into_iter().map(|v| v.slug).collect();
        assert_eq!(slugs, ["v1", "v2", "v10"]);
    }

    #[test]
    fn a_single_page_project_is_one_variant_not_none() {
        let dir = tree(&[], &["index.html"]);
        let found = scan(dir.path());

        assert_eq!(found.len(), 1);
        assert_eq!(found[0].slug, "");
        assert_eq!(found[0].entry, "index.html");
    }

    #[test]
    fn a_folder_without_an_index_is_not_a_version() {
        let dir = tree(&["v1", "v2"], &["v1/index.html", "v2/styles.css"]);
        let slugs: Vec<String> = scan(dir.path()).into_iter().map(|v| v.slug).collect();

        assert_eq!(slugs, ["v1"]);
    }

    fn with_manifest(body: &str) -> tempfile::TempDir {
        let dir = tree(&["v1"], &["v1/index.html"]);
        std::fs::write(dir.path().join(MANIFEST_FILE_NAME), body).expect("write");
        dir
    }

    #[test]
    fn a_manifest_keeps_what_it_says_and_defaults_the_rest() {
        let dir = with_manifest(
            r#"{"version":1,"variants":[{"folder":"v1","name":"Print-tech","tags":["risograph"]}]}"#,
        );

        let ManifestOutcome::Ok(manifest) = read_manifest(dir.path()) else {
            panic!("expected a manifest");
        };
        let entry = &manifest.variants[0];

        assert_eq!(entry.folder, "v1");
        assert_eq!(entry.name.as_deref(), Some("Print-tech"));
        assert_eq!(entry.tags, ["risograph"]);
        assert_eq!(entry.summary, None);
        assert_eq!(entry.family, None);
    }

    #[test]
    fn a_key_nobody_planned_for_does_not_take_the_file_down() {
        // A model that adds a field it thought was helpful must not cost the user every
        // label in the file.
        let dir = with_manifest(r##"{"variants":[{"folder":"v1","palette":["#fff"]}]}"##);

        let ManifestOutcome::Ok(manifest) = read_manifest(dir.path()) else {
            panic!("unknown keys must be ignored, not rejected");
        };
        assert_eq!(manifest.version, 1, "an absent version reads as version 1");
        assert_eq!(manifest.variants[0].folder, "v1");
    }

    #[test]
    fn a_broken_manifest_is_reported_rather_than_thrown() {
        let dir = with_manifest(r#"{"variants":[{"folder":"v1",}]}"#);

        let ManifestOutcome::Malformed(reason) = read_manifest(dir.path()) else {
            panic!("expected the parse error to survive as a message");
        };
        assert!(!reason.is_empty(), "the user needs to know what to fix");
    }

    #[test]
    fn no_manifest_and_an_empty_one_read_the_same() {
        let none = tree(&["v1"], &["v1/index.html"]);
        let empty = with_manifest(r#"{"version":1,"variants":[]}"#);

        assert!(matches!(
            read_manifest(none.path()),
            ManifestOutcome::Absent
        ));
        assert!(matches!(
            read_manifest(empty.path()),
            ManifestOutcome::Absent
        ));
    }

    fn entry(folder: &str) -> ManifestEntry {
        ManifestEntry {
            folder: folder.to_owned(),
            name: Some(folder.to_uppercase()),
            ..ManifestEntry::default()
        }
    }

    #[test]
    fn writing_a_manifest_keeps_only_versions_that_are_really_there() {
        let dir = tree(&["v1", "v2"], &["v1/index.html", "v2/index.html"]);

        let report =
            write_manifest(dir.path(), vec![entry("v1"), entry("v2"), entry("v9")]).expect("write");

        assert_eq!(report.written, 2);
        assert_eq!(
            report.skipped,
            ["v9"],
            "a version that is not there is reported"
        );

        let ManifestOutcome::Ok(read) = read_manifest(dir.path()) else {
            panic!("expected the file back");
        };
        assert_eq!(read.variants.len(), 2);
    }

    #[test]
    fn a_folder_name_cannot_climb_out_of_the_project() {
        // The one way this function could write outside the directory it was handed. The
        // caller is a language model, so the check is not optional.
        let dir = tree(&["v1"], &["v1/index.html"]);

        let report = write_manifest(
            dir.path(),
            vec![entry("../elsewhere"), entry("nested/v1"), entry("v1")],
        )
        .expect("write");

        assert_eq!(report.written, 1);
        assert_eq!(report.skipped.len(), 2);
    }

    #[test]
    fn nothing_valid_writes_no_file_at_all() {
        // A manifest listing nothing is a file that says only that somebody tried.
        let dir = tree(&["v1"], &["v1/index.html"]);
        let report = write_manifest(dir.path(), vec![entry("v9")]).expect("write");

        assert_eq!(report.written, 0);
        assert!(!dir.path().join(MANIFEST_FILE_NAME).exists());
    }

    #[test]
    fn a_manifest_round_trips() {
        // Stage 3 writes this file from the same types an agent's JSON is read into.
        let manifest = Manifest {
            version: 1,
            variants: vec![ManifestEntry {
                folder: "v1".to_owned(),
                name: Some("Print-tech".to_owned()),
                tags: vec!["risograph".to_owned()],
                ..ManifestEntry::default()
            }],
        };

        let text = serde_json::to_string(&manifest).expect("serialize");
        let read: Manifest = serde_json::from_str(&text).expect("deserialize");

        assert_eq!(read.variants[0].name.as_deref(), Some("Print-tech"));
        // Absent optionals are not written back out as nulls — the file stays readable.
        assert!(!text.contains("null"), "{text}");
    }
}
