// pkg/nilpkg/src/durable.rs — Crash- and power-loss-safe filesystem helpers.
//
// A plain rename() orders the metadata operation, but the *data* may still be
// in the page cache when power is lost. These helpers fsync the file, then the
// containing directory, so the rename itself is durable.

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;

/// fsync a directory so a rename/create inside it survives power loss.
/// Directory fsync is not supported on every platform; unsupported errors are
/// reported but not treated as fatal by callers that already fsynced the file.
pub fn sync_dir(dir: &Path) -> Result<(), String> {
    #[cfg(unix)]
    {
        let file = File::open(dir).map_err(|e| format!("Could not open {} to fsync: {e}", dir.display()))?;
        file.sync_all()
            .map_err(|e| format!("Could not fsync {}: {e}", dir.display()))?;
        Ok(())
    }
    #[cfg(not(unix))]
    {
        // Windows has no directory fsync; opening a directory handle for
        // FlushFileBuffers requires a raw Win32 call. NTFS journaling plus the
        // file-level sync_all in write_file_atomic is the practical guarantee.
        let _ = dir;
        Ok(())
    }
}

/// Write `data` to `path` durably and atomically: fsync the temp file, rename,
/// then fsync the directory so the rename is not lost on power failure.
pub fn write_file_atomic(path: &Path, data: &[u8]) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", path.display()))?;
    fs::create_dir_all(parent).map_err(|e| format!("Could not create {}: {e}", parent.display()))?;

    let file_name = path
        .file_name()
        .ok_or_else(|| format!("{} has no file name", path.display()))?
        .to_string_lossy()
        .into_owned();
    let tmp = parent.join(format!(".{file_name}.tmp.{}", std::process::id()));

    {
        let mut file = File::create(&tmp).map_err(|e| format!("Could not create {}: {e}", tmp.display()))?;
        file.write_all(data).map_err(|e| format!("Could not write {}: {e}", tmp.display()))?;
        file.sync_all().map_err(|e| format!("Could not fsync {}: {e}", tmp.display()))?;
    }

    if let Err(e) = fs::rename(&tmp, path) {
        let _ = fs::remove_file(&tmp);
        return Err(format!("Could not replace {}: {e}", path.display()));
    }
    sync_dir(parent)?;
    Ok(())
}

/// fsync a single file, using the access mode each platform actually requires.
fn sync_file(path: &Path) -> Result<(), String> {
    #[cfg(windows)]
    let file = {
        // Windows FlushFileBuffers requires a handle with write access, so a
        // read-only File::open handle fails with ERROR_ACCESS_DENIED. Fall back
        // to skipping only for files we cannot open for writing, since those
        // are not files this process just wrote.
        match OpenOptions::new().write(true).open(path) {
            Ok(f) => f,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return Ok(()),
            Err(e) => return Err(format!("Could not open {} to fsync: {e}", path.display())),
        }
    };
    #[cfg(not(windows))]
    let file =
        File::open(path).map_err(|e| format!("Could not open {} to fsync: {e}", path.display()))?;

    file.sync_all()
        .map_err(|e| format!("Could not fsync {}: {e}", path.display()))
}

/// fsync a whole staged tree (files first, then directories) so a package is
/// fully on disk before the rename that makes it live.
pub fn sync_tree(root: &Path) -> Result<(), String> {
    let mut dirs = vec![root.to_path_buf()];
    while let Some(dir) = dirs.pop() {
        let entries = fs::read_dir(&dir).map_err(|e| format!("Could not read {}: {e}", dir.display()))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = entry.file_type().map_err(|e| format!("Could not stat {}: {e}", path.display()))?;
            if file_type.is_dir() {
                dirs.push(path);
            } else if file_type.is_file() {
                sync_file(&path)?;
            }
        }
        sync_dir(&dir)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nilpkg-durable-{}-{}", std::process::id(), name))
    }

    #[test]
    fn atomic_write_replaces_content_without_leftover_temp_files() {
        let dir = temp_dir("write");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("data.json");
        write_file_atomic(&path, b"first").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"first");
        write_file_atomic(&path, b"second").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"second");

        let leftovers: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains(".tmp."))
            .collect();
        assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn atomic_write_creates_missing_parent_directories() {
        let dir = temp_dir("nested");
        let _ = fs::remove_dir_all(&dir);
        let path = dir.join("a").join("b").join("file");
        write_file_atomic(&path, b"x").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"x");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn sync_tree_succeeds_on_a_populated_tree() {
        let dir = temp_dir("tree");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(dir.join("bin/app"), b"payload").unwrap();
        fs::write(dir.join("manifest.json"), b"{}").unwrap();
        sync_tree(&dir).unwrap();
        let _ = fs::remove_dir_all(&dir);
    }
}
