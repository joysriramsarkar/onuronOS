// runtime/nilrt/src/cgroup.rs — Linux cgroups v2 resource control and process isolation
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// Resource constraints applied to a sandboxed application cgroup.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CgroupLimits {
    /// Hard memory cap in bytes (`memory.max`). Exceeding this triggers OOM killer.
    pub memory_max_bytes: Option<u64>,
    /// Soft memory throttling threshold (`memory.high`).
    pub memory_high_bytes: Option<u64>,
    /// Maximum number of concurrent tasks/threads in the cgroup (`pids.max`).
    pub pids_max: Option<u32>,
    /// Relative CPU weight from 1 to 10000 (`cpu.weight`). Default is 100.
    pub cpu_weight: Option<u32>,
}

impl Default for CgroupLimits {
    fn default() -> Self {
        Self {
            memory_max_bytes: Some(256 * 1024 * 1024), // 256 MiB default app cap
            memory_high_bytes: Some(224 * 1024 * 1024), // 224 MiB throttling threshold
            pids_max: Some(64),                        // 64 tasks maximum (prevents fork bombs)
            cpu_weight: Some(100),                     // Standard priority weight
        }
    }
}

/// Parse available controllers list from `cgroup.controllers` (space-separated tokens).
pub fn parse_controllers(content: &str) -> Vec<String> {
    content
        .split_whitespace()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Format controllers into a `cgroup.subtree_control` string with `+` prefixes.
pub fn format_subtree_control_string(controllers: &[String]) -> String {
    controllers
        .iter()
        .map(|c| format!("+{}", c))
        .collect::<Vec<String>>()
        .join(" ")
}

/// Path generator for an app-specific cgroup directory.
pub fn app_cgroup_dir(root: &Path, slice: &str, app_id: &str) -> PathBuf {
    root.join(slice).join("apps").join(app_id)
}

/// Check if cgroup v2 filesystem is mounted and accessible.
pub fn is_cgroup_v2_available(cgroup_root: &Path) -> bool {
    cgroup_root.join("cgroup.controllers").is_file()
}

/// Read available controllers from `/sys/fs/cgroup/cgroup.controllers`.
pub fn read_available_controllers(cgroup_root: &Path) -> Vec<String> {
    let controllers_file = cgroup_root.join("cgroup.controllers");
    if let Ok(content) = fs::read_to_string(&controllers_file) {
        parse_controllers(&content)
    } else {
        Vec::new()
    }
}

/// Enable supported controllers in `cgroup.subtree_control` for a given cgroup directory.
pub fn enable_subtree_controllers(dir: &Path, requested: &[&str]) -> io::Result<Vec<String>> {
    let controllers_file = dir.join("cgroup.controllers");
    let subtree_file = dir.join("cgroup.subtree_control");

    if !controllers_file.exists() || !subtree_file.exists() {
        return Ok(Vec::new());
    }

    let available = parse_controllers(&fs::read_to_string(&controllers_file)?);
    let to_enable: Vec<String> = requested
        .iter()
        .filter(|req| available.iter().any(|avail| avail == *req))
        .map(|s| s.to_string())
        .collect();

    if !to_enable.is_empty() {
        let control_str = format_subtree_control_string(&to_enable);
        fs::write(&subtree_file, control_str)?;
    }

    Ok(to_enable)
}

/// Setup and configure a sandboxed app's cgroup v2 directory with resource limits.
pub fn create_app_cgroup(
    root: &Path,
    slice: &str,
    app_id: &str,
    limits: &CgroupLimits,
) -> io::Result<PathBuf> {
    let slice_dir = root.join(slice);
    if slice_dir.exists() {
        let _ = enable_subtree_controllers(&slice_dir, &["cpu", "memory", "pids", "io"]);
    }
    let apps_dir = slice_dir.join("apps");
    if fs::create_dir_all(&apps_dir).is_ok() {
        let _ = enable_subtree_controllers(&apps_dir, &["cpu", "memory", "pids", "io"]);
    }

    let dir = app_cgroup_dir(root, slice, app_id);
    fs::create_dir_all(&dir)?;

    // Apply limits (best-effort if specific kernel controllers are not compiled in)
    if let Some(max_mem) = limits.memory_max_bytes {
        let _ = fs::write(dir.join("memory.max"), max_mem.to_string());
    }
    if let Some(high_mem) = limits.memory_high_bytes {
        let _ = fs::write(dir.join("memory.high"), high_mem.to_string());
    }
    if let Some(pids) = limits.pids_max {
        let _ = fs::write(dir.join("pids.max"), pids.to_string());
    }
    if let Some(weight) = limits.cpu_weight {
        let _ = fs::write(dir.join("cpu.weight"), weight.to_string());
    }

    Ok(dir)
}

/// Attach process PID to a cgroup by writing into `cgroup.procs`.
pub fn attach_pid_to_cgroup(cgroup_dir: &Path, pid: u32) -> io::Result<()> {
    let procs_file = cgroup_dir.join("cgroup.procs");
    fs::write(procs_file, pid.to_string())
}

/// Read current memory usage from `memory.current` in bytes.
pub fn read_memory_current(cgroup_dir: &Path) -> Option<u64> {
    fs::read_to_string(cgroup_dir.join("memory.current"))
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
}

/// Read current process count from `pids.current`.
pub fn read_pids_current(cgroup_dir: &Path) -> Option<u32> {
    fs::read_to_string(cgroup_dir.join("pids.current"))
        .ok()
        .and_then(|s| s.trim().parse::<u32>().ok())
}

/// Clean up and remove an app's cgroup directory after termination.
pub fn cleanup_app_cgroup(cgroup_dir: &Path) -> io::Result<()> {
    if cgroup_dir.exists() {
        fs::remove_dir(cgroup_dir)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_controllers() {
        let sample = "cpuset cpu io memory pids";
        let controllers = parse_controllers(sample);
        assert_eq!(controllers, vec!["cpuset", "cpu", "io", "memory", "pids"]);

        let empty = parse_controllers("   \n\t  ");
        assert!(empty.is_empty());
    }

    #[test]
    fn test_format_subtree_control_string() {
        let list = vec!["cpu".to_string(), "memory".to_string(), "pids".to_string()];
        let s = format_subtree_control_string(&list);
        assert_eq!(s, "+cpu +memory +pids");
    }

    #[test]
    fn test_app_cgroup_dir_construction() {
        let root = Path::new("/sys/fs/cgroup");
        let dir = app_cgroup_dir(root, "onuron.slice", "org.onuron.calculator");
        assert_eq!(
            dir,
            PathBuf::from("/sys/fs/cgroup/onuron.slice/apps/org.onuron.calculator")
        );
    }

    #[test]
    fn test_create_and_read_mock_app_cgroup() {
        let temp_dir = std::env::temp_dir().join(format!("cgroup_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let limits = CgroupLimits {
            memory_max_bytes: Some(128 * 1024 * 1024),
            memory_high_bytes: Some(100 * 1024 * 1024),
            pids_max: Some(32),
            cpu_weight: Some(200),
        };

        let app_dir = create_app_cgroup(&temp_dir, "onuron.slice", "test_app", &limits).unwrap();
        assert!(app_dir.exists());

        // Verify written limits
        assert_eq!(
            fs::read_to_string(app_dir.join("memory.max")).unwrap(),
            (128 * 1024 * 1024).to_string()
        );
        assert_eq!(
            fs::read_to_string(app_dir.join("pids.max")).unwrap(),
            "32"
        );
        assert_eq!(
            fs::read_to_string(app_dir.join("cpu.weight")).unwrap(),
            "200"
        );

        // Attach PID
        attach_pid_to_cgroup(&app_dir, 4321).unwrap();
        assert_eq!(fs::read_to_string(app_dir.join("cgroup.procs")).unwrap(), "4321");

        // Mock current values
        fs::write(app_dir.join("memory.current"), "52428800\n").unwrap();
        fs::write(app_dir.join("pids.current"), "4\n").unwrap();

        assert_eq!(read_memory_current(&app_dir), Some(52428800));
        assert_eq!(read_pids_current(&app_dir), Some(4));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
