//! Where things live on disk.
//!
//! Two roots, and the split between them is a security boundary rather than tidiness:
//!
//! * The **data root** (`~/Curio` by default) holds everything the user owns — the
//!   database, screenshots, sidecars, prompts, and the watched projects directory. It is
//!   shareable, backup-able, and partly served over HTTP.
//! * The **app-data directory** holds machine run state: `runtime.json`, the lock file
//!   with its quit token, and the encrypted-secrets fallback.
//!
//! The previous implementation kept run state inside the data root, and that is exactly
//! how the quit token leaked: `/p/<id>/curio.lock` served it through project static
//! hosting. Moving run state out is a deliberate break (R-DA-2, ARCH-08 break #8), and it
//! is why `projectServeRefusal` still lists these names defensively even though they no
//! longer live where it looks.

use std::path::{Path, PathBuf};

/// Files and directories inside the data root that must never be served.
///
/// Belt and braces: `runtime.json`, the lock, and `.secrets.json` moved to the app-data
/// directory (R-DA-2), so in principle nothing here can be reached through `/p/`. They
/// stay on the list because the cost is a string comparison and the precedent is a real
/// token exfiltration (Inventory §10.4). A future change that moves a file back would
/// otherwise silently re-open the hole.
pub const RESERVED_NAMES: &[&str] = &[
    "curio.lock",
    "curiol.lock",
    "library.db",
    "config.json",
    ".secrets.json",
    "runtime.json",
    "skills",
    "items",
    "prompts",
];

/// The database file, inside the data root.
pub const DB_FILE_NAME: &str = "library.db";

/// The user-editable config file, inside the data root.
pub const CONFIG_FILE_NAME: &str = "config.json";

/// The single-instance lock, inside the app-data directory. Carries the quit token.
pub const LOCK_FILE_NAME: &str = "curio.lock";

/// The assessment rubric, seeded once and **never overwritten** — the user is meant to
/// edit it, and an update that clobbered their edits would be indistinguishable from
/// data loss (R-DA-1, R-BE-29).
pub const SKILL_FILE_RELATIVE: &str = "skills/visual-assessment.md";

/// The default data root: `~/Curio`.
///
/// # Errors
/// Returns [`crate::Error::Invalid`] if the home directory cannot be determined.
pub fn default_data_root() -> crate::Result<PathBuf> {
    Ok(home_dir()?.join("Curio"))
}

/// The legacy data root the one-time migration moves from: `~/Curiol`.
///
/// The rename is guarded — it runs only for the default root, only when the target is
/// absent, only when the legacy directory actually holds a library, and it **never
/// merges** (R-DA-3, Inventory §10.18). Those guards are the boot path's job; this
/// function only names the directory.
///
/// # Errors
/// Returns [`crate::Error::Invalid`] if the home directory cannot be determined.
pub fn legacy_data_root() -> crate::Result<PathBuf> {
    Ok(home_dir()?.join("Curiol"))
}

/// The per-OS directory for run state and secrets.
///
/// Windows: `%LOCALAPPDATA%\Curio`. macOS: `~/Library/Application Support/Curio`.
/// Otherwise the XDG data directory. Implemented directly rather than through a
/// directories crate: it is three branches, and the idle-footprint budget makes every
/// avoidable dependency worth avoiding.
///
/// # Errors
/// Returns [`crate::Error::Invalid`] if neither the platform variable nor the home
/// directory can be resolved.
pub fn app_data_dir() -> crate::Result<PathBuf> {
    #[cfg(windows)]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return Ok(PathBuf::from(local).join("Curio"));
        }
        Ok(home_dir()?.join("AppData").join("Local").join("Curio"))
    }

    #[cfg(target_os = "macos")]
    {
        Ok(home_dir()?
            .join("Library")
            .join("Application Support")
            .join("Curio"))
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME") {
            return Ok(PathBuf::from(xdg).join("curio"));
        }
        Ok(home_dir()?.join(".local").join("share").join("curio"))
    }
}

/// Where `runtime.json` lives this run.
///
/// # Errors
/// Propagates [`app_data_dir`]'s failure.
pub fn runtime_file() -> crate::Result<PathBuf> {
    Ok(app_data_dir()?.join(curio_runtime_file_name()))
}

/// The directory holding one item's screenshot, thumbnail, and sidecar.
#[must_use]
pub fn item_dir(data_root: &Path, item_id: &str) -> PathBuf {
    data_root.join("items").join(item_id)
}

/// Whether a resolved path stays inside `jail`.
///
/// The check is on the **resolved** target, not the raw string, because that is the
/// distinction the previous implementation got wrong: a raw-string check passes while the
/// resolved path lands on the lock file (Inventory §10.4). Callers must canonicalize both
/// sides before asking — this function assumes that has happened and only compares.
#[must_use]
pub fn is_inside(jail: &Path, resolved: &Path) -> bool {
    resolved.starts_with(jail)
}

/// Whether a resolved file name is refused by project static serving.
///
/// Dotfiles are refused wholesale; the reserved names are matched case-insensitively,
/// because Windows would otherwise serve `LIBRARY.DB` past a case-sensitive comparison
/// (Inventory §10.4).
#[must_use]
pub fn is_refused_project_file(name: &str) -> bool {
    if name.starts_with('.') {
        return true;
    }
    let lowered = name.to_ascii_lowercase();
    RESERVED_NAMES.iter().any(|reserved| {
        let reserved = reserved.to_ascii_lowercase();
        // The database prefix covers `library.db-wal` and `library.db-shm` too: the WAL
        // holds committed data that has not yet been checkpointed, so serving it is
        // serving the library.
        lowered == reserved || lowered.starts_with(&format!("{reserved}-"))
    })
}

fn curio_runtime_file_name() -> &'static str {
    // Kept as a function rather than a re-export so this crate does not depend on
    // curio-runtime; the name is part of two contracts and matches by test, not by type.
    "runtime.json"
}

fn home_dir() -> crate::Result<PathBuf> {
    #[cfg(windows)]
    let candidates = ["USERPROFILE", "HOME"];
    #[cfg(unix)]
    let candidates = ["HOME"];

    for key in candidates {
        // An empty value is treated as unset: some launchers export the variable with no
        // content, and taking it literally would put the library at the filesystem root.
        if let Some(value) = std::env::var_os(key).filter(|value| !value.is_empty()) {
            return Ok(PathBuf::from(value));
        }
    }
    Err(crate::Error::invalid(
        "cannot determine the home directory; set CURIO_DATA_ROOT explicitly",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dotfiles_are_refused() {
        assert!(is_refused_project_file(".env"));
        assert!(is_refused_project_file(".secrets.json"));
        assert!(is_refused_project_file(".git"));
    }

    #[test]
    fn the_lock_file_is_refused() {
        // The precedent this whole list exists for: the quit token was exfiltrated
        // through `/p/<id>/curio.lock` (Inventory §10.4).
        assert!(is_refused_project_file("curio.lock"));
        assert!(is_refused_project_file("curiol.lock"));
    }

    #[test]
    fn the_database_and_its_sidecar_files_are_refused() {
        // The WAL holds committed data awaiting checkpoint. Serving it is serving the
        // library, so the prefix match is not paranoia.
        assert!(is_refused_project_file("library.db"));
        assert!(is_refused_project_file("library.db-wal"));
        assert!(is_refused_project_file("library.db-shm"));
    }

    #[test]
    fn refusal_is_case_insensitive() {
        // Windows would otherwise serve this straight past a case-sensitive comparison.
        assert!(is_refused_project_file("LIBRARY.DB"));
        assert!(is_refused_project_file("Config.json"));
        assert!(is_refused_project_file("Curio.Lock"));
    }

    #[test]
    fn ordinary_project_files_are_served() {
        assert!(!is_refused_project_file("index.html"));
        assert!(!is_refused_project_file("app.js"));
        assert!(!is_refused_project_file("library.js"));
        assert!(!is_refused_project_file("config.ts"));
    }

    #[test]
    fn the_jail_check_rejects_escapes() {
        let jail = Path::new("/data/items/01J");
        assert!(is_inside(jail, Path::new("/data/items/01J/screenshot.png")));
        assert!(!is_inside(
            jail,
            Path::new("/data/items/01K/screenshot.png")
        ));
        assert!(!is_inside(jail, Path::new("/data/library.db")));
    }

    #[test]
    fn app_data_is_not_inside_the_data_root() {
        // R-DA-2 / ARCH-08 break #8. Run state and secrets must not sit anywhere the
        // project static server can reach.
        let Ok(app_data) = app_data_dir() else {
            return;
        };
        let Ok(data_root) = default_data_root() else {
            return;
        };
        assert!(
            !app_data.starts_with(&data_root),
            "{} must not be inside {}",
            app_data.display(),
            data_root.display()
        );
    }

    #[test]
    fn item_directories_are_per_item() {
        let root = Path::new("/data");
        assert_eq!(
            item_dir(root, "01J"),
            Path::new("/data").join("items").join("01J")
        );
    }
}
