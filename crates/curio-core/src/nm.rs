//! Native-messaging registration: the manifest, and where each browser looks for it.
//!
//! Chrome will not spawn a native-messaging host it has not been told about. The telling is
//! a small JSON manifest in a per-browser directory (and, on Windows, a registry value
//! pointing at it), written by the installer (R-EXT-3, D29).
//!
//! This module is the **pure half**: what the manifest says and where it goes. The writing
//! lives in `curio-nmh`, which is the binary an installer can invoke without dragging the
//! whole app in.
//!
//! ## `allowed_origins` is the security boundary
//!
//! It is the only thing standing between this host and *any* extension the user installs.
//! Chrome refuses to connect an extension whose id is not listed, so the pinned origin here
//! is what makes "the token rides the native-messaging reply" safe: a hostile extension
//! that guesses the host name still cannot ask it anything.
//!
//! That origin is derived from the extension's manifest key and appears in exactly two
//! places — here and the server's CORS check — which is [`EXTENSION_ORIGIN`]'s reason for
//! living in this crate rather than in either of them (R-OV-2, Inventory §10.1).

use std::path::PathBuf;

/// The host name Chrome knows this by. Matches the extension's `connectNative` argument.
pub const HOST_NAME: &str = "com.curio.nmh";

/// The extension's origin, derived from the `key` in its `manifest.json`.
///
/// One fact, two readers: this pins `allowed_origins` in the native-messaging manifest, and
/// the server's origin check pins the same value for CORS (Inventory §10.1). A test in
/// `curio-server` derives it from the manifest key so the two cannot drift apart silently.
pub const EXTENSION_ORIGIN: &str = "chrome-extension://fclkgbgdifbddhlhnlhcfnijhcepdhke";

/// A Chromium-family browser that might have the extension installed.
///
/// All four are listed because a user who installed Curio's extension in Edge and not in
/// Chrome should not have to discover that registration is per-browser. Writing a manifest
/// for a browser that is not installed is harmless — the directory simply is not there, and
/// registration skips it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Browser {
    Chrome,
    Edge,
    Brave,
    Chromium,
}

impl Browser {
    #[must_use]
    pub const fn all() -> [Browser; 4] {
        [
            Browser::Chrome,
            Browser::Edge,
            Browser::Brave,
            Browser::Chromium,
        ]
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Browser::Chrome => "Chrome",
            Browser::Edge => "Edge",
            Browser::Brave => "Brave",
            Browser::Chromium => "Chromium",
        }
    }

    /// The registry path Windows looks under, without the `HKCU\` prefix.
    ///
    /// Windows keeps the manifest itself anywhere on disk and stores a *pointer* to it
    /// here; the other platforms put the manifest in a known directory instead.
    #[must_use]
    pub fn registry_key(self) -> String {
        let vendor = match self {
            Browser::Chrome => r"Google\Chrome",
            Browser::Edge => r"Microsoft\Edge",
            Browser::Brave => r"BraveSoftware\Brave-Browser",
            Browser::Chromium => r"Chromium",
        };
        format!(r"Software\{vendor}\NativeMessagingHosts\{HOST_NAME}")
    }

    /// Where this browser reads native-messaging manifests from, on platforms that use a
    /// directory. `None` when the location cannot be resolved — a missing `HOME`, say.
    ///
    /// Windows returns `None` for every browser: it uses the registry instead, and
    /// pretending otherwise would have registration silently write files nothing reads.
    #[must_use]
    pub fn manifest_dir(self) -> Option<PathBuf> {
        #[cfg(target_os = "macos")]
        {
            let vendor = match self {
                Browser::Chrome => "Google/Chrome",
                Browser::Edge => "Microsoft Edge",
                Browser::Brave => "BraveSoftware/Brave-Browser",
                Browser::Chromium => "Chromium",
            };
            let home = std::env::var_os("HOME")?;
            Some(
                PathBuf::from(home)
                    .join("Library/Application Support")
                    .join(vendor)
                    .join("NativeMessagingHosts"),
            )
        }

        #[cfg(target_os = "linux")]
        {
            let vendor = match self {
                Browser::Chrome => "google-chrome",
                Browser::Edge => "microsoft-edge",
                Browser::Brave => "BraveSoftware/Brave-Browser",
                Browser::Chromium => "chromium",
            };
            let home = std::env::var_os("HOME")?;
            Some(
                PathBuf::from(home)
                    .join(".config")
                    .join(vendor)
                    .join("NativeMessagingHosts"),
            )
        }

        #[cfg(windows)]
        {
            let _ = self;
            None
        }
    }

    /// The manifest file for this browser, on platforms that use a directory.
    #[must_use]
    pub fn manifest_path(self) -> Option<PathBuf> {
        self.manifest_dir()
            .map(|dir| dir.join(format!("{HOST_NAME}.json")))
    }
}

/// The manifest, pointing at `host_path`.
///
/// `path` must be **absolute**. Chrome resolves it relative to nothing useful, and a
/// relative path produces a host that silently never starts — which surfaces to the user as
/// an extension that cannot find Curio, several layers away from the cause.
#[must_use]
pub fn manifest(host_path: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "name": HOST_NAME,
        "description": "Curio native messaging host",
        "path": host_path.to_string_lossy(),
        "type": "stdio",
        // The whole security boundary. Chrome refuses any extension not named here.
        "allowed_origins": [format!("{EXTENSION_ORIGIN}/")],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_manifest_allows_exactly_one_extension() {
        // R-EXT-3. This list is what stops any other installed extension from asking the
        // host for a live token.
        let rendered = manifest(std::path::Path::new("/opt/curio/curio-nmh"));
        let origins = rendered["allowed_origins"].as_array().expect("origins");

        assert_eq!(origins.len(), 1);
        assert_eq!(origins[0], format!("{EXTENSION_ORIGIN}/"));
    }

    #[test]
    fn the_allowed_origin_carries_the_trailing_slash_chrome_requires() {
        // Chrome matches the origin pattern literally. Without the slash it never matches,
        // and the failure is a host that refuses to start with no diagnostic anywhere.
        let rendered = manifest(std::path::Path::new("/opt/curio/curio-nmh"));
        let origin = rendered["allowed_origins"][0].as_str().expect("origin");

        assert!(origin.ends_with('/'), "{origin}");
    }

    #[test]
    fn the_manifest_names_the_stdio_transport() {
        let rendered = manifest(std::path::Path::new("/opt/curio/curio-nmh"));

        assert_eq!(rendered["type"], "stdio");
        assert_eq!(rendered["name"], HOST_NAME);
    }

    #[test]
    fn the_host_path_is_carried_verbatim() {
        // Including on Windows, where a backslash path must survive JSON encoding intact.
        let path = std::path::Path::new(r"C:\Program Files\Curio\curio-nmh.exe");
        let rendered = manifest(path);

        assert_eq!(
            rendered["path"].as_str().expect("path"),
            r"C:\Program Files\Curio\curio-nmh.exe"
        );
    }

    #[test]
    fn every_browser_has_a_distinct_registry_key() {
        // Registering all four under one key would mean the last one written wins and the
        // other three silently do not work.
        let keys: std::collections::HashSet<String> =
            Browser::all().iter().map(|b| b.registry_key()).collect();

        assert_eq!(keys.len(), Browser::all().len());
        for key in &keys {
            assert!(key.ends_with(HOST_NAME), "{key}");
        }
    }

    #[test]
    #[cfg(not(windows))]
    fn every_browser_has_a_distinct_manifest_path() {
        // SAFETY: single-threaded test; nothing else reads HOME concurrently.
        unsafe { std::env::set_var("HOME", "/home/test") };

        let paths: std::collections::HashSet<PathBuf> = Browser::all()
            .iter()
            .filter_map(|b| b.manifest_path())
            .collect();

        assert_eq!(paths.len(), Browser::all().len());
    }

    #[test]
    #[cfg(windows)]
    fn windows_registers_through_the_registry_rather_than_a_directory() {
        // Writing manifest files on Windows would produce files no browser reads, and a
        // registration that reports success while doing nothing.
        for browser in Browser::all() {
            assert!(browser.manifest_dir().is_none());
        }
    }
}
