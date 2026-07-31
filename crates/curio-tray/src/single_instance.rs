//! One Curio at a time.
//!
//! Two instances sharing one library would each hold a write connection, which breaks the
//! single-writer invariant everything else rests on (R-DA-8). So the guard runs **before
//! any other side effect** (R-BE-4): before the config is read, before the database is
//! touched, before anything is written.
//!
//! The mechanism differs per platform because the right primitive does:
//!
//! * **Windows** — a named mutex. The kernel releases it when the process dies, however it
//!   dies, so a crash cannot leave a lock behind.
//! * **Unix** — `flock` on a file. Also released by the kernel on process exit, which is
//!   what makes it safe against a crash where a plain "is this file present" check is not.
//!
//! Both mean the same thing: the guard is held by a **live process**, not by a file that
//! happens to exist. That is why a stale `runtime.json` can be reclaimed confidently —
//! if the lock is free, whoever wrote that file is gone.

use std::path::Path;

/// A held single-instance lock. Released when dropped or when the process dies.
#[derive(Debug)]
pub struct InstanceGuard {
    #[cfg(windows)]
    handle: windows_sys::Win32::Foundation::HANDLE,
    #[cfg(unix)]
    file: std::fs::File,
}

// SAFETY: the Windows HANDLE is owned exclusively by this guard and only closed in Drop.
#[cfg(windows)]
unsafe impl Send for InstanceGuard {}

/// Why the guard could not be taken.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// Another Curio is already running.
    ///
    /// Not an error condition for the user: the second launch's job is to open the
    /// dashboard on the instance that already exists and exit quietly (R-BE-4).
    #[error("another Curio is already running")]
    AlreadyRunning,

    #[error("could not take the single-instance lock: {0}")]
    Io(#[from] std::io::Error),
}

impl InstanceGuard {
    /// Take the lock, or report that someone else holds it.
    ///
    /// # Errors
    /// Returns [`Error::AlreadyRunning`] if another instance holds it, or [`Error::Io`] if
    /// the lock could not be created.
    pub fn acquire(lock_path: &Path) -> Result<Self, Error> {
        if let Some(parent) = lock_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        Self::platform_acquire(lock_path)
    }

    #[cfg(windows)]
    fn platform_acquire(lock_path: &Path) -> Result<Self, Error> {
        use windows_sys::Win32::Foundation::{CloseHandle, ERROR_ALREADY_EXISTS, GetLastError};
        use windows_sys::Win32::System::Threading::CreateMutexW;

        // Local\ rather than Global\: the scope is this user's session, which matches the
        // trust boundary — Curio's data root is per-user, so two users may each run their
        // own instance without interfering.
        let name: Vec<u16> = format!("Local\\{}\0", mutex_name(lock_path))
            .encode_utf16()
            .collect();

        // SAFETY: `name` is a valid NUL-terminated UTF-16 string that outlives the call.
        let handle = unsafe { CreateMutexW(std::ptr::null(), 1, name.as_ptr()) };
        if handle.is_null() {
            return Err(Error::Io(std::io::Error::last_os_error()));
        }

        // SAFETY: called immediately after CreateMutexW on this thread.
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            // SAFETY: `handle` is a valid handle we own and are giving up.
            unsafe { CloseHandle(handle) };
            return Err(Error::AlreadyRunning);
        }

        Ok(Self { handle })
    }

    #[cfg(unix)]
    fn platform_acquire(lock_path: &Path) -> Result<Self, Error> {
        use std::os::unix::io::AsRawFd as _;

        let file = std::fs::OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(lock_path)?;

        // LOCK_NB so a second launch learns immediately rather than blocking behind the
        // first — it has somewhere else to be (opening the dashboard).
        // SAFETY: `flock` takes a valid descriptor this function owns.
        let locked = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
        if locked != 0 {
            let err = std::io::Error::last_os_error();
            return match err.raw_os_error() {
                Some(libc::EWOULDBLOCK) => Err(Error::AlreadyRunning),
                _ => Err(Error::Io(err)),
            };
        }

        Ok(Self { file })
    }
}

/// The mutex name for a given lock path.
///
/// Derived from the path rather than fixed, because the invariant is **one instance per
/// library**, not one per machine. A developer running against `CURIO_DATA_ROOT=/tmp/scratch`
/// while their real library is open is not violating anything — the two share no database,
/// which is the thing the guard exists to protect. A fixed name would refuse that, and would
/// also make two of these guards inside one process (as in the tests below) collide for no
/// reason.
#[cfg(windows)]
fn mutex_name(lock_path: &Path) -> String {
    // FNV-1a over the lowercased path. Not a security boundary — the guard's strength is
    // the kernel object, not the obscurity of its name — just a stable, filename-safe
    // identifier, and Windows object names cannot contain the backslashes a path does.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in lock_path.to_string_lossy().to_lowercase().bytes() {
        hash ^= u64::from(byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("CurioSingleInstance-{hash:016x}")
}

impl Drop for InstanceGuard {
    fn drop(&mut self) {
        #[cfg(windows)]
        {
            use windows_sys::Win32::Foundation::CloseHandle;
            // SAFETY: the handle was created by CreateMutexW and is closed exactly once.
            unsafe { CloseHandle(self.handle) };
        }
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd as _;
            // SAFETY: releasing a lock this guard holds on a descriptor it owns. The
            // kernel would do this on exit anyway; doing it here makes the release
            // deterministic for a guard dropped inside a still-running process.
            unsafe { libc::flock(self.file.as_raw_fd(), libc::LOCK_UN) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lock_can_be_taken_and_released() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("curio.lock");

        {
            let _guard = InstanceGuard::acquire(&path).expect("first acquire");
        }
        // Dropping releases it, so a clean restart is not blocked by its predecessor.
        let _again = InstanceGuard::acquire(&path).expect("re-acquire after release");
    }

    #[test]
    fn a_second_guard_is_refused_while_the_first_is_held() {
        // The invariant everything else rests on: two instances must never both hold a
        // write connection to one library.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("curio.lock");

        let _first = InstanceGuard::acquire(&path).expect("first acquire");

        match InstanceGuard::acquire(&path) {
            Err(Error::AlreadyRunning) => {}
            other => panic!("expected AlreadyRunning, got {other:?}"),
        }
    }

    #[test]
    fn two_different_libraries_can_both_be_open() {
        // The guard protects a library, not a machine. A developer pointed at a scratch
        // data root while their real library is open shares no database with it, so
        // refusing would be a false positive — and would make this crate's own tests
        // collide with each other for the same non-reason.
        let first_dir = tempfile::tempdir().expect("tempdir");
        let second_dir = tempfile::tempdir().expect("tempdir");

        let _first =
            InstanceGuard::acquire(&first_dir.path().join("curio.lock")).expect("first library");
        let _second = InstanceGuard::acquire(&second_dir.path().join("curio.lock"))
            .expect("a second, unrelated library must be allowed");
    }

    #[test]
    fn acquiring_creates_the_parent_directory() {
        // The app-data directory may not exist on a first run, and the guard runs before
        // anything else that would create it (R-BE-4).
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("nested").join("deeper").join("curio.lock");

        let _guard = InstanceGuard::acquire(&path).expect("acquire");
        assert!(path.parent().expect("parent").is_dir());
    }
}
