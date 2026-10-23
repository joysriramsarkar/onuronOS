// services/logd/src/main.rs — Structured pstore & System Log Collector
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

pub const MAX_LOG_FILE_BYTES: u64 = 512 * 1024;

pub fn append_log_entry(log_path: &Path, tag: &str, message: &str) -> std::io::Result<()> {
    if let Some(parent) = log_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if let Ok(meta) = fs::metadata(log_path) {
        if meta.len() >= MAX_LOG_FILE_BYTES {
            let rotated = log_path.with_extension("log.1");
            let _ = fs::rename(log_path, rotated);
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(log_path)?;
    writeln!(file, "[{}] [{}] {}", ts, tag.trim(), message.trim())
}

pub fn harvest_pstore(pstore_dir: &Path, log_path: &Path) -> usize {
    if !pstore_dir.is_dir() {
        return 0;
    }
    let mut count = 0;
    if let Ok(entries) = fs::read_dir(pstore_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Ok(content) = fs::read_to_string(entry.path()) {
                let first_line = content.lines().next().unwrap_or("pstore dump");
                if append_log_entry(log_path, "pstore", &format!("{}: {}", name, first_line))
                    .is_ok()
                {
                    count += 1;
                }
            }
        }
    }
    count
}

fn main() {
    println!("[logd] pstore & kmsg Logger active.");
    let _ = nilsd::notify_ready("logd", None);
    let log_path = Path::new("/data/logs/system.log");
    let pstore_dir = Path::new("/sys/fs/pstore");
    let harvested = harvest_pstore(pstore_dir, log_path);
    let _ = append_log_entry(
        log_path,
        "logd",
        &format!(
            "Log daemon started (harvested {} pstore entries)",
            harvested
        ),
    );

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_append_log_and_harvest_pstore() {
        let dir = std::env::temp_dir().join("onuron_logd_test");
        let _ = fs::remove_dir_all(&dir);
        let pstore = dir.join("pstore");
        let log_file = dir.join("system.log");
        fs::create_dir_all(&pstore).unwrap();
        fs::write(
            pstore.join("console-ramoops-0"),
            "previous boot kernel log\n",
        )
        .unwrap();

        let n = harvest_pstore(&pstore, &log_file);
        assert_eq!(n, 1);
        append_log_entry(&log_file, "nilinit", "Boot completed").unwrap();

        let text = fs::read_to_string(&log_file).unwrap();
        assert!(text.contains("[pstore] console-ramoops-0"));
        assert!(text.contains("[nilinit] Boot completed"));
        let _ = fs::remove_dir_all(&dir);
    }
}
