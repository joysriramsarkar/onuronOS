// pkg/nilpkg/src/lockfile.rs — Cross-process lock for package operations.
//
// Two `nilpkg` processes mutating the same app root can otherwise interleave
// their staging renames and destroy an installed package. The lock is held by
// the OS, so it is released automatically when a process exits or crashes —
// there is no stale-lock file to clean up by hand.

use std::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};

pub struct PackageLock {
    #[allow(dead_code)]
    path: PathBuf,
    // Holding the handle is what keeps the lock; dropping it releases.
    _file: File,
}

const BUSY: &str = "another nilpkg operation is already running on this app root";

impl PackageLock {
    /// Take an exclusive, non-blocking lock on the app root.
    pub fn acquire(app_root: &Path) -> Result<Self, String> {
        std::fs::create_dir_all(app_root).map_err(|e| format!("Could not create app root: {e}"))?;
        let path = app_root.join(".nilpkg.lock");
        let file = open_locked(&path)?;
        Ok(Self { path, _file: file })
    }
}

#[cfg(unix)]
fn open_locked(path: &Path) -> Result<File, String> {
    use std::os::unix::io::AsRawFd;

    let file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| format!("Could not open lock file {}: {e}", path.display()))?;

    // flock() is advisory and released on close/exit, including a crash.
    let rc = unsafe { libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
    if rc != 0 {
        let err = std::io::Error::last_os_error();
        if err.raw_os_error() == Some(libc::EWOULDBLOCK) {
            return Err(BUSY.into());
        }
        return Err(format!("Could not lock {}: {e}", path.display(), e = err));
    }
    Ok(file)
}

#[cfg(windows)]
fn open_locked(path: &Path) -> Result<File, String> {
    use std::os::windows::fs::OpenOptionsExt;

    // ERROR_SHARING_VIOLATION — the other holder opened the file exclusively.
    const ERROR_SHARING_VIOLATION: i32 = 32;
    // share_mode(0) denies all sharing. The handle is closed by the kernel when
    // the owning process dies, so a crash cannot leave the lock stuck.
    match OpenOptions::new()
        .write(true)
        .create(true)
        .share_mode(0)
        .open(path)
    {
        Ok(file) => Ok(file),
        Err(e) if e.raw_os_error() == Some(ERROR_SHARING_VIOLATION) => Err(BUSY.into()),
        Err(e) => Err(format!("Could not open lock file {}: {e}", path.display())),
    }
}

#[cfg(not(any(unix, windows)))]
fn open_locked(path: &Path) -> Result<File, String> {
    OpenOptions::new()
        .write(true)
        .create(true)
        .open(path)
        .map_err(|e| format!("Could not open lock file {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("nilpkg-lock-{}-{}", std::process::id(), name))
    }

    #[test]
    fn second_acquire_is_refused_while_first_is_held() {
        let root = temp_root("held");
        let _ = std::fs::remove_dir_all(&root);
        let first = PackageLock::acquire(&root).unwrap();
        let second = PackageLock::acquire(&root);
        assert!(second.is_err(), "a second lock attempt must not succeed");
        drop(first);
        // Released on drop, so it can be taken again.
        PackageLock::acquire(&root).unwrap();
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn acquire_creates_a_missing_app_root() {
        let root = temp_root("create").join("nested").join("apps");
        let _ = std::fs::remove_dir_all(&root);
        let lock = PackageLock::acquire(&root).unwrap();
        assert!(root.is_dir());
        assert!(root.join(".nilpkg.lock").exists());
        drop(lock);
        let _ = std::fs::remove_dir_all(root.parent().unwrap().parent().unwrap());
    }

    /// The lock must be visible across processes, not just within one.
    #[test]
    fn lock_is_enforced_across_processes() {
        // When re-invoked as the child prober, report whether the lock could be
        // taken via the exit code, then stop before running the rest.
        if std::env::var("NILPKG_LOCK_PROBE").is_ok() {
            let root = std::path::PathBuf::from(std::env::var("NILPKG_LOCK_ROOT").unwrap());
            std::process::exit(match PackageLock::acquire(&root) {
                Ok(_) => 0,  // acquired: nobody was holding it
                Err(_) => 3, // refused: the parent holds it
            });
        }

        let root = temp_root("xproc");
        let _ = std::fs::remove_dir_all(&root);
        let held = PackageLock::acquire(&root).unwrap();

        let probe = |root: &std::path::Path| {
            std::process::Command::new(std::env::current_exe().unwrap())
                .args(["--exact", "lockfile::tests::lock_is_enforced_across_processes", "--nocapture"])
                .env("NILPKG_LOCK_PROBE", "1")
                .env("NILPKG_LOCK_ROOT", root)
                .status()
                .unwrap()
                .code()
        };

        assert_eq!(probe(&root), Some(3), "a second process must be refused the held lock");
        drop(held);
        assert_eq!(probe(&root), Some(0), "the lock must be acquirable once released");
        let _ = std::fs::remove_dir_all(&root);
    }
}
