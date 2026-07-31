//! `--register` / `--unregister`: telling browsers this host exists.
//!
//! Run by the installer (R-EXT-3, D29). It lives here rather than in the tray for one
//! reason: an installer needs to invoke *something* to do this, and this binary is already
//! on disk, already knows its own path, and drags in nothing — where invoking the main
//! app would mean starting the tray, the server, and the database during an install.
//!
//! ## What "registered" means differs by platform
//!
//! On macOS and Linux, a browser reads manifests from a directory it owns, so registration
//! is writing one file per browser. On Windows the manifest may live anywhere and the
//! browser finds it through a registry value — so registration is writing the file once,
//! beside the executable, and pointing four registry keys at it.
//!
//! ## Missing browsers are not failures
//!
//! Most users have one or two of the four. Registration reports what it did and carries on
//! past a browser that is not installed: failing an install because the user does not have
//! Brave would be absurd, and silently doing nothing would leave them with an extension
//! that cannot find the app and no way to find out why.

use std::path::{Path, PathBuf};

use curio_core::nm::{self, Browser};

/// What happened, per browser, in words an installer log can carry.
pub struct Report {
    pub registered: Vec<String>,
    pub skipped: Vec<String>,
}

impl Report {
    fn new() -> Self {
        Self {
            registered: Vec::new(),
            skipped: Vec::new(),
        }
    }

    /// Whether anything at all worked.
    ///
    /// False is worth reporting to the installer: it means no browser on this machine can
    /// spawn the host, so the extension will fall back to the probe ladder or manual
    /// pairing (R-EXT-8) rather than working out of the box.
    #[must_use]
    pub fn any(&self) -> bool {
        !self.registered.is_empty()
    }
}

/// Register this host with every Chromium-family browser present.
///
/// # Errors
/// Returns an error only if this executable's own path cannot be resolved — without it
/// there is nothing to point a browser at, and writing a manifest naming the wrong binary
/// would be worse than writing none.
pub fn register() -> Result<Report, String> {
    let host_path = std::env::current_exe()
        .map_err(|err| format!("could not resolve this executable's path: {err}"))?;
    let manifest = nm::manifest(&host_path);
    let body = serde_json::to_string_pretty(&manifest)
        .map_err(|err| format!("could not render the manifest: {err}"))?;

    let mut report = Report::new();

    #[cfg(windows)]
    {
        // One file, beside the binary, pointed at four times. Chrome reads the path out of
        // the registry, so the file's location is ours to choose — and next to the
        // executable is the one place an uninstaller is certain to remove.
        let manifest_path = windows_manifest_path(&host_path);
        if let Some(parent) = manifest_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&manifest_path, &body)
            .map_err(|err| format!("could not write {}: {err}", manifest_path.display()))?;

        for browser in Browser::all() {
            match write_registry_value(&browser.registry_key(), &manifest_path) {
                Ok(()) => report.registered.push(browser.label().to_owned()),
                Err(reason) => report
                    .skipped
                    .push(format!("{}: {reason}", browser.label())),
            }
        }
    }

    #[cfg(not(windows))]
    {
        for browser in Browser::all() {
            let Some(path) = browser.manifest_path() else {
                report
                    .skipped
                    .push(format!("{}: no manifest location", browser.label()));
                continue;
            };

            // The parent existing is the signal the browser is installed. Creating it
            // regardless would scatter directories for browsers the user does not have —
            // harmless, but it makes an uninstall's job ambiguous.
            let installed = path.parent().is_some_and(Path::exists)
                || path
                    .parent()
                    .and_then(Path::parent)
                    .is_some_and(Path::exists);
            if !installed {
                report
                    .skipped
                    .push(format!("{}: not installed", browser.label()));
                continue;
            }

            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            match std::fs::write(&path, &body) {
                Ok(()) => report.registered.push(browser.label().to_owned()),
                Err(err) => report.skipped.push(format!("{}: {err}", browser.label())),
            }
        }
    }

    Ok(report)
}

/// Undo [`register`]. Best effort by design.
///
/// An uninstall that fails because a registry key was already gone is an uninstall that
/// leaves the app half-removed. Every step here tolerates "it was not there".
pub fn unregister() -> Report {
    let mut report = Report::new();

    #[cfg(windows)]
    {
        for browser in Browser::all() {
            match delete_registry_key(&browser.registry_key()) {
                Ok(()) => report.registered.push(browser.label().to_owned()),
                Err(reason) => report
                    .skipped
                    .push(format!("{}: {reason}", browser.label())),
            }
        }
        if let Ok(host_path) = std::env::current_exe() {
            let _ = std::fs::remove_file(windows_manifest_path(&host_path));
        }
    }

    #[cfg(not(windows))]
    {
        for browser in Browser::all() {
            let Some(path) = browser.manifest_path() else {
                continue;
            };
            match std::fs::remove_file(&path) {
                Ok(()) => report.registered.push(browser.label().to_owned()),
                Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                Err(err) => report.skipped.push(format!("{}: {err}", browser.label())),
            }
        }
    }

    report
}

/// Where the single Windows manifest lives: beside the binary it names.
#[cfg(windows)]
#[must_use]
fn windows_manifest_path(host_path: &Path) -> PathBuf {
    host_path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!("{}.json", nm::HOST_NAME))
}

/// Point a registry key at the manifest.
///
/// Shelled out to `reg.exe` rather than linked against `windows-sys`, because this crate's
/// dependency list is its whole point: Chrome spawns it once per connection and its startup
/// time is user-visible latency in the popup's status dot (R-EXT-3, D27). A process spawn on
/// the `--register` path — which runs once, during an install — costs nothing that matters.
#[cfg(windows)]
fn write_registry_value(key: &str, manifest_path: &Path) -> Result<(), String> {
    let output = std::process::Command::new("reg")
        .args([
            "add",
            &format!(r"HKCU\{key}"),
            "/ve",
            "/t",
            "REG_SZ",
            "/d",
            &manifest_path.to_string_lossy(),
            "/f",
        ])
        .output()
        .map_err(|err| format!("could not run reg.exe: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(windows)]
fn delete_registry_key(key: &str) -> Result<(), String> {
    let output = std::process::Command::new("reg")
        .args(["delete", &format!(r"HKCU\{key}"), "/f"])
        .output()
        .map_err(|err| format!("could not run reg.exe: {err}"))?;

    if output.status.success() {
        Ok(())
    } else {
        // "The system was unable to find the specified registry key" is success for an
        // uninstall: the desired state is reached.
        Err(String::from_utf8_lossy(&output.stderr).trim().to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_report_with_nothing_registered_says_so() {
        // The installer branches on this: no browser registered means the extension will
        // fall back to the probe ladder rather than working out of the box.
        let report = Report::new();
        assert!(!report.any());
    }

    #[test]
    #[cfg(windows)]
    fn the_windows_manifest_sits_beside_the_binary_it_names() {
        // Anywhere else and an uninstaller has to know a second location to clean.
        let host = Path::new(r"C:\Program Files\Curio\curio-nmh.exe");
        let manifest = windows_manifest_path(host);

        assert_eq!(manifest.parent(), host.parent());
        assert_eq!(
            manifest.file_name().and_then(|n| n.to_str()),
            Some("com.curio.nmh.json")
        );
    }

    #[test]
    fn the_registry_keys_are_under_the_current_user() {
        // HKLM would need elevation, and a per-user extension does not warrant it.
        for browser in Browser::all() {
            let key = browser.registry_key();
            assert!(key.starts_with("Software\\"), "{key}");
            assert!(!key.contains("HKEY"), "{key}");
        }
    }
}
