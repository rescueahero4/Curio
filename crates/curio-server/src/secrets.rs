//! The Anthropic API key: keychain first, and never anywhere else (R-SEC-10).
//!
//! Read order is `ANTHROPIC_API_KEY` → the OS keychain. The environment variable comes
//! first because it is how CI, a dev shell, and a user who already manages secrets
//! elsewhere expect to override — and because a key supplied that way was never ours to
//! store.
//!
//! The key MUST never reach the database, a sidecar, `runtime.json`, a log line, or a
//! settings response. The settings API is write-only for it: accepted on PUT, surfaced only
//! as a boolean and a mask.
//!
//! ## What is deliberately missing, and why it is a refusal rather than a fallback
//!
//! ARCH-06 R-SEC-10 also specifies an AES-256-GCM encrypted-file fallback with scrypt
//! parameters matching the previous implementation, so an in-place upgrade can read an
//! existing `.secrets.json`. Those parameters are **unverified** — it is D0 row 4
//! (ARCH-06 OQ-3), still outstanding — and guessing them would produce a file the old app
//! wrote and this one silently cannot read.
//!
//! So on a platform with no keychain backend, storing a key **fails loudly** instead of
//! writing one to disk in the clear. A refusal a user can act on beats a plaintext secret
//! they never agreed to, and the environment variable remains available on every platform.

use crate::routes::error::ApiError;

/// The environment variable that overrides everything.
pub const ENV_VAR: &str = "ANTHROPIC_API_KEY";

/// The keychain service name. Shared with the previous implementation so an existing
/// macOS keychain entry is found rather than duplicated.
///
/// Only the macOS backend reads it — DPAPI keys off the Windows account rather than a
/// service name — but it is asserted by a test on every platform, because drifting it is a
/// silent upgrade failure rather than a compile error.
#[cfg_attr(
    not(target_os = "macos"),
    allow(dead_code, reason = "asserted by test on all platforms")
)]
const SERVICE: &str = "curio-anthropic-api-key";

/// The key, if one is configured.
#[must_use]
pub fn api_key() -> Option<String> {
    if let Ok(value) = std::env::var(ENV_VAR)
        && !value.trim().is_empty()
    {
        return Some(value);
    }
    backend::read()
}

/// Whether a key is configured. What `/health` reports (R-SEC-11) — a boolean, never a
/// prefix.
#[must_use]
pub fn is_configured() -> bool {
    api_key().is_some()
}

/// The previous implementation's secrets file, if it is still sitting in the data root.
///
/// D31 retired the encrypted-file backend rather than guess at its scrypt parameters, which
/// means an upgrading user's stored key does not carry over. That is a fair trade for a
/// credential they can re-obtain in seconds — but only if they are **told**. Silently
/// starting with no key would look like the app forgetting something, and the honest
/// version of that is one line of copy in Settings.
///
/// Reports presence only. The file is never opened, never parsed, and never moved: we
/// cannot read it, and pretending otherwise is what this decision avoided.
#[must_use]
pub fn legacy_secrets_present(data_root: &std::path::Path) -> bool {
    data_root.join(".secrets.json").is_file()
}

/// Store a key in the OS keychain.
///
/// # Errors
/// Returns an error if the key is empty, or if this platform has no keychain backend — see
/// the module note on why that is a refusal rather than a plaintext write.
pub fn store_api_key(key: &str) -> Result<(), ApiError> {
    let key = key.trim();
    if key.is_empty() {
        return Err(ApiError(curio_core::Error::invalid(
            "an API key cannot be empty — use Clear to remove the one you have",
        )));
    }
    backend::write(key).map_err(|message| ApiError(curio_core::Error::invalid(message)))
}

/// Remove the stored key.
///
/// # Errors
/// Returns an error only if the backend fails in a way the user should know about. A key
/// that was not there is not one of them: Clear is idempotent by design, because a user
/// pressing it twice means the same thing both times.
pub fn clear_api_key() -> Result<(), ApiError> {
    backend::clear().map_err(|message| ApiError(curio_core::Error::invalid(message)))
}

#[cfg(windows)]
mod backend {
    //! DPAPI, via `CryptProtectData`. The ciphertext is bound to the Windows user account,
    //! so a copy of the file is useless on another machine or under another login.
    //!
    //! Called through `windows-sys` rather than a wrapper crate: it is two functions, and
    //! the idle-footprint budget makes every avoidable dependency worth avoiding
    //! (R-BE-31).

    use std::path::PathBuf;

    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{
        CRYPT_INTEGER_BLOB, CryptProtectData, CryptUnprotectData,
    };

    fn vault() -> Option<PathBuf> {
        curio_core::paths::app_data_dir()
            .ok()
            .map(|dir| dir.join("secrets.dpapi"))
    }

    pub(super) fn read() -> Option<String> {
        let bytes = std::fs::read(vault()?).ok()?;
        unprotect(&bytes)
    }

    pub(super) fn write(key: &str) -> Result<(), String> {
        let path = vault().ok_or("could not resolve the application data directory")?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let protected = protect(key.as_bytes()).ok_or("Windows refused to protect the key")?;
        std::fs::write(&path, protected).map_err(|err| err.to_string())
    }

    pub(super) fn clear() -> Result<(), String> {
        let Some(path) = vault() else {
            return Ok(());
        };
        match std::fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err.to_string()),
        }
    }

    fn protect(plain: &[u8]) -> Option<Vec<u8>> {
        // The secret is passed as a buffer, never as a command-line argument — argv is
        // world-readable to every process on the machine (Inventory §5).
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(plain.len()).ok()?,
            pbData: plain.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        // SAFETY: both blobs are initialised and live for the call; `pbData` on the output
        // is allocated by Windows and freed with `LocalFree` immediately after copying.
        let ok = unsafe {
            CryptProtectData(
                &raw mut input,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                &raw mut output,
            )
        };
        if ok == 0 {
            return None;
        }

        let copied =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe { LocalFree(output.pbData.cast()) };
        Some(copied)
    }

    fn unprotect(sealed: &[u8]) -> Option<String> {
        let mut input = CRYPT_INTEGER_BLOB {
            cbData: u32::try_from(sealed.len()).ok()?,
            pbData: sealed.as_ptr().cast_mut(),
        };
        let mut output = CRYPT_INTEGER_BLOB {
            cbData: 0,
            pbData: std::ptr::null_mut(),
        };

        // SAFETY: as above.
        let ok = unsafe {
            CryptUnprotectData(
                &raw mut input,
                std::ptr::null_mut(),
                std::ptr::null(),
                std::ptr::null_mut(),
                std::ptr::null(),
                0,
                &raw mut output,
            )
        };
        if ok == 0 {
            return None;
        }

        let copied =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec() };
        unsafe { LocalFree(output.pbData.cast()) };
        String::from_utf8(copied).ok()
    }
}

#[cfg(target_os = "macos")]
mod backend {
    //! The macOS keychain, through the `security` CLI — the same service name the previous
    //! implementation used, so an existing entry is found rather than duplicated.
    //!
    //! The secret is written to the tool's **stdin**, never passed as an argument: argv is
    //! visible to every process on the machine (Inventory §5).

    use std::io::Write as _;
    use std::process::{Command, Stdio};

    pub(super) fn read() -> Option<String> {
        let output = Command::new("security")
            .args(["find-generic-password", "-s", super::SERVICE, "-w"])
            .output()
            .ok()?;
        if !output.status.success() {
            return None;
        }
        let value = String::from_utf8(output.stdout).ok()?.trim().to_owned();
        (!value.is_empty()).then_some(value)
    }

    pub(super) fn write(key: &str) -> Result<(), String> {
        let mut child = Command::new("security")
            .args([
                "add-generic-password",
                "-s",
                super::SERVICE,
                "-a",
                super::SERVICE,
                "-U",
                "-w",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|err| format!("could not reach the macOS keychain: {err}"))?;

        child
            .stdin
            .as_mut()
            .ok_or("could not write to the keychain helper")?
            .write_all(key.as_bytes())
            .map_err(|err| err.to_string())?;

        let status = child.wait().map_err(|err| err.to_string())?;
        if status.success() {
            Ok(())
        } else {
            Err("the macOS keychain refused to store the key".to_owned())
        }
    }

    pub(super) fn clear() -> Result<(), String> {
        // A non-zero exit means "no such entry", which is the state Clear is trying to
        // reach — so it is success, not failure.
        let _ = Command::new("security")
            .args(["delete-generic-password", "-s", super::SERVICE])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        Ok(())
    }
}

#[cfg(not(any(windows, target_os = "macos")))]
mod backend {
    //! No keychain backend on this platform. See the module note: the encrypted-file
    //! fallback is blocked on D0 row 4 (ARCH-06 OQ-3), and a plaintext file is not an
    //! acceptable stand-in for it.

    pub(super) fn read() -> Option<String> {
        None
    }

    pub(super) fn write(_key: &str) -> Result<(), String> {
        Err(format!(
            "Curio has no secure key store on this platform yet. \
             Set the {} environment variable instead.",
            super::ENV_VAR
        ))
    }

    pub(super) fn clear() -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_empty_key_is_refused_with_a_pointer_to_clear() {
        // Saving "" would otherwise look like a way to remove a key, and leave the old one
        // in place while telling the user it saved.
        let error = store_api_key("   ").expect_err("refused");
        assert!(error.0.to_string().contains("Clear"), "{}", error.0);
    }

    #[test]
    fn clearing_is_idempotent() {
        // A user pressing Clear twice means the same thing both times.
        clear_api_key().expect("first");
        clear_api_key().expect("second");
    }

    #[test]
    fn the_service_name_matches_the_previous_implementation() {
        // An upgrade must find the existing keychain entry rather than sit beside it
        // asking for a key the user already provided (Inventory §5).
        assert_eq!(SERVICE, "curio-anthropic-api-key");
    }

    #[test]
    fn the_environment_variable_is_the_documented_one() {
        assert_eq!(ENV_VAR, "ANTHROPIC_API_KEY");
    }
}
