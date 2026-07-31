//! `runtime.json` — the file that tells every client where the app is and how to talk to it.
//!
//! Curio binds an ephemeral port, so no client can guess its address (D10). Instead the
//! service writes this file once it is genuinely ready, and four kinds of consumer read
//! it: the native-messaging host (which relays it to the extension), the CLI, the
//! MCP-stdio proxy, and the app's own next launch performing its stale-instance check.
//!
//! Two rules make the file trustworthy, and both are enforced here rather than at the
//! call sites:
//!
//! * **It is written atomically.** A reader must never observe a half-written token.
//!   [`RuntimeFile::write_atomic`] writes a sibling temp file and renames it into place.
//! * **It is owner-only.** It carries the runtime bearer token, so it is created 0600 on
//!   Unix. On Windows the protection is the per-user app-data directory's inherited ACL —
//!   see [`write_atomic`](RuntimeFile::write_atomic) for why that is the boundary rather
//!   than a weaker one.
//!
//! It is written only *after* migrations and bind both succeed, and deleted at quit, so
//! its existence means "a healthy instance is listening" (R-BE-5, R-BE-33). The one thing
//! its presence does not prove is that the process still exists — a crash leaves it
//! behind — which is why [`RuntimeFile::pid_is_alive`] lives here too (R-BE-34).
//!
//! Owned by ARCH-01; this crate is its only home (R-OV-2).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The file's name inside the per-OS app-data directory.
///
/// Not the data root: the data root is user-shareable and project-servable, and run
/// state must be neither (R-DA-2; the Bun-era quit-token leak through `/p/<id>/curio.lock`
/// is the precedent).
pub const FILE_NAME: &str = "runtime.json";

/// Whether the instance is accepting mutations.
///
/// "Off" is a soft-disable, not a shutdown: the listener stays bound and reads keep
/// working, so this is a state a running server advertises rather than a reason the file
/// is absent (D2, D25).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum State {
    Running,
    Paused,
}

impl State {
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            State::Running => "running",
            State::Paused => "paused",
        }
    }
}

/// What a live instance advertises about itself.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuntimeFile {
    /// The OS-assigned port this run bound (R-BE-6).
    pub port: u16,
    /// The per-run bearer token: 32 CSPRNG bytes, base64url, minted at service start and
    /// invalidated at quit (R-SEC-2, D21). Never logged, never in a URL.
    pub token: String,
    /// The service process id, for the liveness check in [`Self::pid_is_alive`].
    pub pid: u32,
    /// The single stamped workspace version (R-DEL-12).
    pub version: String,
    pub state: State,
}

/// What a reader learned about the instance this file describes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Discovery {
    /// A healthy-looking instance: the file parsed and its pid is alive.
    Live(Box<RuntimeFile>),
    /// The file parsed but the process is gone — a crash left this behind.
    ///
    /// `curio-nmh` reports this to the extension as `{state: "stale"}`, and the extension
    /// treats it exactly as not-running (R-EXT-6). The server treats it as a lock to
    /// reclaim (R-BE-4).
    Stale,
    /// No file: nothing has run, or the last run shut down cleanly.
    Absent,
}

impl RuntimeFile {
    /// Read and classify the file at `path`.
    ///
    /// A malformed or unreadable file is [`Discovery::Stale`], not an error. A reader's
    /// only useful response to garbage is the same as to a dead pid — reclaim or report
    /// not-running — and `curio-nmh` in particular must answer Chrome with a message
    /// rather than an exit code (R-EXT-6).
    pub fn discover(path: &Path) -> Discovery {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Discovery::Absent,
            Err(_) => return Discovery::Stale,
        };
        match serde_json::from_str::<RuntimeFile>(&raw) {
            Ok(file) if file.pid_is_alive() => Discovery::Live(Box::new(file)),
            Ok(_) | Err(_) => Discovery::Stale,
        }
    }

    /// Write the file atomically with owner-only permissions.
    ///
    /// Temp-file-plus-rename because a reader must never see a partial token: `rename`
    /// within a directory is atomic on both target platforms, so a reader observes either
    /// the old file or the complete new one.
    ///
    /// On Unix the temp file is created 0600 *before* any content reaches it, so the token
    /// is never briefly world-readable. On Windows there is no equivalent mode bit; the
    /// protection is that the file lives in the per-user app-data directory, whose ACL the
    /// new file inherits. That is the same boundary the threat model already draws — other
    /// processes running as the same user are explicitly out of scope (R-SEC, §Threat
    /// model) — so this is the documented boundary, not a gap.
    pub fn write_atomic(&self, path: &Path) -> io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let temp = temp_sibling(path);
        let body = serde_json::to_vec_pretty(self).map_err(io::Error::other)?;

        write_owner_only(&temp, &body)?;

        // Windows rename fails if the destination exists; remove it first. The window
        // between remove and rename is a reader seeing Absent, which is a state every
        // consumer already handles (Discovery::Absent).
        match fs::rename(&temp, path) {
            Ok(()) => Ok(()),
            Err(_) if path.exists() => {
                fs::remove_file(path)?;
                fs::rename(&temp, path)
            }
            Err(err) => {
                let _ = fs::remove_file(&temp);
                Err(err)
            }
        }
    }

    /// Delete the file. Absence is success — quit must be idempotent (R-BE-7).
    pub fn remove(path: &Path) -> io::Result<()> {
        match fs::remove_file(path) {
            Ok(()) => Ok(()),
            Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(err) => Err(err),
        }
    }

    /// Whether the recorded process still exists.
    ///
    /// A PID check and nothing more. `curio-nmh` embeds no HTTP client, so this is the
    /// only staleness signal it has; the authenticated `/health` probe is a server-boot
    /// responsibility and lives there (R-BE-34).
    ///
    /// PID reuse can make a dead instance look alive. That is acceptable at this layer:
    /// the consequence is one failed request that the client's 401/connection-refused
    /// path already recovers from, and the server's boot check is the backstop.
    #[must_use]
    pub fn pid_is_alive(&self) -> bool {
        pid_is_alive(self.pid)
    }
}

fn temp_sibling(path: &Path) -> PathBuf {
    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(".tmp");
    path.with_file_name(name)
}

#[cfg(unix)]
fn write_owner_only(path: &Path, body: &[u8]) -> io::Result<()> {
    use std::io::Write as _;
    use std::os::unix::fs::OpenOptionsExt as _;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)?;
    file.write_all(body)?;
    file.sync_all()
}

#[cfg(windows)]
fn write_owner_only(path: &Path, body: &[u8]) -> io::Result<()> {
    use std::io::Write as _;

    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(path)?;
    file.write_all(body)?;
    file.sync_all()
}

#[cfg(unix)]
fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    // Signal 0 performs the existence and permission checks without delivering anything.
    // EPERM means the process exists but belongs to someone else — which cannot be our
    // service, so it is not a live instance for our purposes.
    // SAFETY: `kill` with signal 0 has no side effects and takes only scalars.
    unsafe { libc::kill(pid as libc::pid_t, 0) == 0 }
}

#[cfg(windows)]
fn pid_is_alive(pid: u32) -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, STILL_ACTIVE};
    use windows_sys::Win32::System::Threading::{
        GetExitCodeProcess, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION,
    };

    if pid == 0 {
        return false;
    }

    // SAFETY: OpenProcess takes scalars and returns a handle we close on every path.
    // A null handle means the process is gone (or unqueryable), which we treat as dead.
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return false;
        }
        let mut code: u32 = 0;
        // A handle alone is not proof of life: Windows keeps the handle openable while a
        // terminated process's exit code is still readable. STILL_ACTIVE is the check.
        let queried = GetExitCodeProcess(handle, &raw mut code);
        CloseHandle(handle);
        queried != 0 && code == STILL_ACTIVE as u32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> RuntimeFile {
        RuntimeFile {
            port: 51_234,
            token: "not-a-real-token".to_owned(),
            pid: std::process::id(),
            version: "0.1.0".to_owned(),
            state: State::Running,
        }
    }

    #[test]
    fn round_trips_through_disk() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);
        let written = sample();

        written.write_atomic(&path).expect("write");

        match RuntimeFile::discover(&path) {
            Discovery::Live(read) => assert_eq!(*read, written),
            other => panic!("expected Live, got {other:?}"),
        }
    }

    #[test]
    fn overwrites_an_existing_file() {
        // Every boot rewrites this file with a fresh token (D21). If the second write
        // failed because the first existed, a restarted app would advertise a dead token.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);

        sample().write_atomic(&path).expect("first write");
        let mut second = sample();
        second.port = 51_235;
        second.write_atomic(&path).expect("second write");

        match RuntimeFile::discover(&path) {
            Discovery::Live(read) => assert_eq!(read.port, 51_235),
            other => panic!("expected Live, got {other:?}"),
        }
    }

    #[test]
    fn a_dead_pid_reads_as_stale() {
        // The crash case: the file survives the process. Every consumer must treat this
        // as not-running rather than trusting the recorded port (R-EXT-6, R-BE-4).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);
        let mut file = sample();
        // PID 0 is never a live user process on either platform.
        file.pid = 0;
        file.write_atomic(&path).expect("write");

        assert_eq!(RuntimeFile::discover(&path), Discovery::Stale);
    }

    #[test]
    fn garbage_reads_as_stale_not_as_an_error() {
        // curio-nmh must answer Chrome with a message, never an exit code (R-EXT-6).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);
        fs::write(&path, b"{ truncated").expect("write");

        assert_eq!(RuntimeFile::discover(&path), Discovery::Stale);
    }

    #[test]
    fn absence_is_distinct_from_staleness() {
        let dir = tempfile::tempdir().expect("tempdir");
        assert_eq!(
            RuntimeFile::discover(&dir.path().join(FILE_NAME)),
            Discovery::Absent
        );
    }

    #[test]
    fn remove_is_idempotent() {
        // Quit runs it; a crash-recovered boot runs it again (R-BE-7).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);
        sample().write_atomic(&path).expect("write");

        RuntimeFile::remove(&path).expect("first remove");
        RuntimeFile::remove(&path).expect("second remove");
    }

    #[cfg(unix)]
    #[test]
    fn is_owner_only_on_unix() {
        use std::os::unix::fs::PermissionsExt as _;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(FILE_NAME);
        sample().write_atomic(&path).expect("write");

        let mode = fs::metadata(&path).expect("metadata").permissions().mode();
        assert_eq!(mode & 0o777, 0o600, "runtime.json carries the bearer token");
    }

    #[test]
    fn state_serializes_lowercase() {
        // The extension and the NM host read this string; ARCH-01 spells it "running" |
        // "paused" and clients compare it literally.
        let json = serde_json::to_string(&State::Paused).expect("serialize");
        assert_eq!(json, "\"paused\"");
    }
}
