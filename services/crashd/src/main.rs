// services/crashd/src/main.rs — Crash Tombstone & Kernel Panic Collector
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;

pub const MAX_TOMBSTONES: usize = 16;

pub fn write_tombstone(
    dir: &Path,
    pid: u32,
    process_name: &str,
    signal: i32,
    summary: &str,
) -> std::io::Result<PathBuf> {
    fs::create_dir_all(dir)?;
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let filename = format!("tombstone_{}_{}.txt", pid, ts);
    let path = dir.join(filename);
    let payload = format!(
        "*** ONURON OS CRASH TOMBSTONE ***\nPID: {}\nProcess: {}\nSignal: {}\nSummary: {}\n",
        pid, process_name, signal, summary
    );
    fs::write(&path, payload)?;
    prune_tombstones(dir, MAX_TOMBSTONES)?;
    Ok(path)
}

pub fn prune_tombstones(dir: &Path, max_keep: usize) -> std::io::Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    let mut files: Vec<PathBuf> = fs::read_dir(dir)?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();
    while files.len() > max_keep {
        let oldest = files.remove(0);
        let _ = fs::remove_file(oldest);
    }
    Ok(())
}

fn main() {
    println!("[crashd] Crash Tombstone & Kernel Panic Collector active.");
    let _ = nilsd::notify_ready("crashd", None);
    let dir = Path::new("/data/logs/tombstones");
    let _ = fs::create_dir_all(dir);

    loop {
        let _ = prune_tombstones(dir, MAX_TOMBSTONES);
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_write_and_prune_tombstones() {
        let dir = std::env::temp_dir().join("onuron_crashd_test");
        let _ = fs::remove_dir_all(&dir);

        for i in 0..5 {
            write_tombstone(&dir, 100 + i, "test_app", 11, "SIGSEGV fault").unwrap();
        }
        prune_tombstones(&dir, 3).unwrap();
        let remaining = fs::read_dir(&dir).unwrap().count();
        assert_eq!(remaining, 3);

        let _ = fs::remove_dir_all(&dir);
    }
}
