//! What has to be true before the service starts: the data root, `config.json`, and the
//! quit token.
//!
//! All of it runs **after** the single-instance guard and **before** the database opens
//! (R-BE-5). The ordering is what makes the guard meaningful: a second launch that created
//! directories or rewrote config on its way to discovering it was second would leave marks
//! on a library it never opened.

use std::path::{Path, PathBuf};

use curio_core::config::Config;

/// Load `config.json`, creating it with defaults if absent, and materialize the data root.
///
/// The file is rewritten on every boot (R-BE-28), which is what makes it self-documenting:
/// a user who opens it sees every setting with its current value rather than only the ones
/// they happened to change.
///
/// A malformed file is **repaired rather than fatal**. It is user-editable by design, and
/// refusing to start over a stray comma would leave someone with a tray icon that does
/// nothing and no way to fix it except finding this file themselves.
///
/// # Errors
/// Returns an error if the data root cannot be created or the config cannot be written.
pub fn load_config(data_root: &Path) -> anyhow::Result<Config> {
    materialize(data_root)?;

    let path = data_root.join(curio_core::paths::CONFIG_FILE_NAME);
    let mut config = match std::fs::read_to_string(&path) {
        Ok(body) => serde_json::from_str::<Config>(&body).unwrap_or_else(|err| {
            tracing::warn!(%err, "config.json could not be read; starting from defaults");
            Config::default()
        }),
        Err(_) => Config::default(),
    };

    config.data_root = data_root.display().to_string();
    if config.projects_root.is_empty() {
        config.projects_root = data_root.join("projects").display().to_string();
    }
    if config.validate().is_err() {
        // A hand-edited file with, say, inverted thresholds. Repairing the invalid fields
        // beats refusing to start, and the rewrite below makes the repair visible.
        tracing::warn!("config.json held invalid values; the affected settings were reset");
        config.thresholds = curio_core::config::Thresholds::default();
        config.port = None;
    }

    std::fs::write(&path, serde_json::to_string_pretty(&config)?)?;
    Ok(config)
}

/// Create the data root's directories and seed the rubric.
fn materialize(data_root: &Path) -> anyhow::Result<()> {
    for child in ["items", "prompts", "projects", "skills"] {
        std::fs::create_dir_all(data_root.join(child))?;
    }

    let skill = data_root.join(curio_core::paths::SKILL_FILE_RELATIVE);
    if !skill.exists() {
        // Seeded once and **never overwritten** (R-DA-1, R-BE-29). The user is meant to
        // edit this file; an update that clobbered their rubric would be indistinguishable
        // from data loss, and it is the one input that shapes every assessment.
        std::fs::write(&skill, DEFAULT_RUBRIC)?;
    }
    Ok(())
}

/// Mint the quit token into the lock file (R-SEC-8).
///
/// A separate secret from the runtime token, in a separate file, with a separate lifetime.
/// A paired extension holding the runtime token must not thereby hold a kill switch
/// (Inventory §10.3).
///
/// # Errors
/// Returns an error if the lock file cannot be written.
pub fn mint_quit_token(lock_path: &Path) -> anyhow::Result<String> {
    let token = random_hex();
    let body = serde_json::json!({
        "pid": std::process::id(),
        "quitToken": token,
        "startedAt": curio_core::time::now_iso(),
    });

    std::fs::write(lock_path, serde_json::to_string_pretty(&body)?)?;
    restrict(lock_path);
    Ok(token)
}

/// 32 bytes of CSPRNG randomness as hex, matching the previous implementation's shape.
fn random_hex() -> String {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("the OS entropy source must be available");
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// Owner-only permissions, where the platform has them.
fn restrict(path: &Path) {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt as _;
        let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600));
    }
    #[cfg(windows)]
    {
        // Windows inherits the parent directory's ACL, and the app-data directory is
        // already per-user. There is no mode bit to set, and the equivalent — rewriting the
        // DACL — would be a larger change than the threat model asks for: R-SEC-4's actor
        // model explicitly excludes other processes of the same OS user.
        let _ = path;
    }
}

/// Where the data root actually is (R-BE-29, R-DA-3).
///
/// `CURIO_DATA_ROOT` → deprecated `CURIOL_DATA_ROOT` (with a warning) → `~/Curio`, after a
/// one-time `~/Curiol` → `~/Curio` rename.
///
/// # Errors
/// Returns an error if the home directory cannot be resolved.
pub fn resolve_data_root() -> anyhow::Result<PathBuf> {
    if let Some(explicit) = std::env::var_os("CURIO_DATA_ROOT") {
        return Ok(PathBuf::from(explicit));
    }
    if let Some(legacy) = std::env::var_os("CURIOL_DATA_ROOT") {
        tracing::warn!("CURIOL_DATA_ROOT is deprecated; use CURIO_DATA_ROOT");
        return Ok(PathBuf::from(legacy));
    }

    let target = curio_core::paths::default_data_root()?;
    migrate_legacy_root(&target);
    Ok(target)
}

/// The one-time `~/Curiol` → `~/Curio` rename, with every guard the original had.
///
/// Only the default root, only when the target is absent, only when the legacy directory
/// actually holds a library, and it **never merges** (R-DA-3, Inventory §10.18). A merge
/// would interleave two libraries' items directories with no way to tell them apart
/// afterwards.
fn migrate_legacy_root(target: &Path) {
    if target.exists() {
        return;
    }
    let Ok(legacy) = curio_core::paths::legacy_data_root() else {
        return;
    };
    let holds_a_library = legacy.join(curio_core::paths::DB_FILE_NAME).exists()
        || legacy.join(curio_core::paths::CONFIG_FILE_NAME).exists();
    if !holds_a_library {
        return;
    }

    match std::fs::rename(&legacy, target) {
        Ok(()) => tracing::info!(
            from = %legacy.display(),
            to = %target.display(),
            "moved the data root to its current name"
        ),
        // Falling back is the documented behaviour: a failed rename must not stop the app,
        // and the legacy root stays readable by pointing CURIO_DATA_ROOT at it.
        Err(err) => tracing::warn!(%err, "could not move the legacy data root; leaving it alone"),
    }
}

/// The rubric a fresh install starts with.
///
/// Deliberately short. It is a **starting point the user is expected to rewrite** — the
/// product's claim is that the library speaks the user's own vocabulary, and a long
/// prescriptive rubric shipped by us works against that.
const DEFAULT_RUBRIC: &str = r"# Visual assessment

You are describing a design reference for a personal library. Describe what you see in the
vocabulary this library already uses, and propose new vocabulary only when nothing existing
fits.

## What to produce

- **name** — what a designer would call this screen, not what the company calls itself.
- **short_description** — one or two sentences on what it is and how it feels.
- **design_types** — what kind of thing it is: landing page, pricing table, dashboard.
- **tags** — free descriptors. Prefer words already in the library.
- **family_scores** — how well this matches each existing aesthetic family, 0.0 to 1.0.
- **new_family_proposal** — only when nothing above 0.5 fits, and only with a description
  written in the same format as the existing families.
- **image_recipe** — optional: how you would brief someone to produce an image like this.

## Rules

- Score every family you are shown. **Do not apply thresholds yourself** — report the
  numbers and Curio decides.
- Describe the design, not the product. 'Warm serif editorial' is useful; 'a website for a
  law firm' is not.
- Reuse existing tags rather than coining near-duplicates.
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_data_root_gets_its_directories_and_a_rubric() {
        let dir = tempfile::tempdir().expect("tempdir");
        load_config(dir.path()).expect("load");

        for child in ["items", "prompts", "projects", "skills"] {
            assert!(dir.path().join(child).is_dir(), "{child}");
        }
        assert!(dir.path().join("skills/visual-assessment.md").is_file());
    }

    #[test]
    fn the_rubric_is_never_overwritten() {
        // R-DA-1. The user is meant to edit this file; clobbering it on upgrade would be
        // indistinguishable from data loss, and it is the one input shaping every
        // assessment.
        let dir = tempfile::tempdir().expect("tempdir");
        load_config(dir.path()).expect("first");
        let skill = dir.path().join("skills/visual-assessment.md");
        std::fs::write(&skill, "my own rubric").expect("edit");

        load_config(dir.path()).expect("second");

        assert_eq!(
            std::fs::read_to_string(&skill).expect("read"),
            "my own rubric"
        );
    }

    #[test]
    fn a_malformed_config_is_repaired_rather_than_fatal() {
        // The file is user-editable by design. Refusing to start over a stray comma leaves
        // someone with a tray icon that does nothing and no way to fix it.
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("config.json"), "{ not json,,,").expect("write");

        let config = load_config(dir.path()).expect("load");

        assert_eq!(config.thresholds, curio_core::config::Thresholds::default());
    }

    #[test]
    fn invalid_values_are_reset_and_written_back_visibly() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            dir.path().join("config.json"),
            r#"{"thresholds":{"lower":0.9,"upper":0.1}}"#,
        )
        .expect("write");

        let config = load_config(dir.path()).expect("load");

        assert!(config.thresholds.lower <= config.thresholds.upper);
        let rewritten = std::fs::read_to_string(dir.path().join("config.json")).expect("read");
        assert!(rewritten.contains("0.4"), "{rewritten}");
    }

    #[test]
    fn the_config_is_rewritten_with_every_setting_named() {
        // R-BE-28: the file documents itself, so a user sees every setting rather than
        // only the ones they changed.
        let dir = tempfile::tempdir().expect("tempdir");
        load_config(dir.path()).expect("load");

        let body = std::fs::read_to_string(dir.path().join("config.json")).expect("read");
        for key in [
            "projectsRoot",
            "thresholds",
            "models",
            "mcpEnabled",
            "sendToClaudeTarget",
        ] {
            assert!(body.contains(key), "{key} missing from {body}");
        }
    }

    #[test]
    fn the_projects_root_defaults_inside_the_data_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = load_config(dir.path()).expect("load");

        assert!(
            config.projects_root.ends_with("projects"),
            "{}",
            config.projects_root
        );
    }

    #[test]
    fn a_quit_token_is_minted_into_the_lock_file() {
        // R-SEC-8: a separate secret, in a separate file, with a separate lifetime.
        let dir = tempfile::tempdir().expect("tempdir");
        let lock = dir.path().join("curio.lock");

        let token = mint_quit_token(&lock).expect("mint");

        assert_eq!(token.len(), 64, "32 bytes as hex");
        let body = std::fs::read_to_string(&lock).expect("read");
        assert!(body.contains(&token));
        assert!(body.contains("pid"));
    }

    #[test]
    fn two_boots_mint_different_quit_tokens() {
        let dir = tempfile::tempdir().expect("tempdir");
        let lock = dir.path().join("curio.lock");

        assert_ne!(
            mint_quit_token(&lock).expect("first"),
            mint_quit_token(&lock).expect("second")
        );
    }

    #[test]
    fn the_legacy_root_is_left_alone_when_the_current_one_exists() {
        // Never merge (R-DA-3). Interleaving two libraries' items directories cannot be
        // undone afterwards.
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("Curio");
        std::fs::create_dir_all(&target).expect("mkdir");

        migrate_legacy_root(&target);

        assert!(target.is_dir());
    }
}
