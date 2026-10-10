// runtime/nilrt/src/selinux.rs — SELinux transition helpers and runtime verification
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;

/// Operational status of SELinux in the running kernel.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SelinuxMode {
    /// SELinux policy is actively enforced (denials are blocked and logged).
    Enforcing,
    /// SELinux policy is loaded but only logs denials (not blocked).
    Permissive,
    /// SELinux is disabled in the kernel or kernel command line.
    Disabled,
    /// SELinux is not available on this platform/kernel.
    Unavailable,
}

impl SelinuxMode {
    pub fn as_str(&self) -> &'static str {
        match self {
            SelinuxMode::Enforcing => "Enforcing",
            SelinuxMode::Permissive => "Permissive",
            SelinuxMode::Disabled => "Disabled",
            SelinuxMode::Unavailable => "Unavailable",
        }
    }
}

/// Checks whether SELinux filesystem or proc attribute endpoints exist.
pub fn is_selinux_enabled() -> bool {
    #[cfg(target_os = "linux")]
    {
        Path::new("/sys/fs/selinux").is_dir() || Path::new("/proc/thread-self/attr/exec").exists()
    }
    #[cfg(not(target_os = "linux"))]
    {
        false
    }
}

/// Query current SELinux runtime mode from kernel filesystem `/sys/fs/selinux/enforce`.
pub fn get_selinux_mode() -> SelinuxMode {
    #[cfg(target_os = "linux")]
    {
        if !Path::new("/sys/fs/selinux").is_dir() {
            return SelinuxMode::Disabled;
        }
        if let Ok(val) = fs::read_to_string("/sys/fs/selinux/enforce") {
            match val.trim() {
                "1" => SelinuxMode::Enforcing,
                "0" => SelinuxMode::Permissive,
                _ => SelinuxMode::Unavailable,
            }
        } else {
            SelinuxMode::Unavailable
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        SelinuxMode::Unavailable
    }
}

/// Helper returning true only if the kernel has confirmed enforcing mode.
pub fn is_selinux_enforcing() -> bool {
    get_selinux_mode() == SelinuxMode::Enforcing
}

/// Read the current process's active SELinux security context.
pub fn getcon() -> Result<String, String> {
    #[cfg(target_os = "linux")]
    {
        let paths = ["/proc/thread-self/attr/current", "/proc/self/attr/current"];
        for path in &paths {
            if Path::new(path).exists() {
                if let Ok(con) = fs::read_to_string(path) {
                    let clean = con.trim_matches('\0').trim().to_string();
                    if !clean.is_empty() {
                        return Ok(clean);
                    }
                }
            }
        }
        Err("Unable to read SELinux context from /proc attr/current".to_string())
    }
    #[cfg(not(target_os = "linux"))]
    {
        Ok("unconfined_u:unconfined_r:unconfined_t:s0".to_string())
    }
}

/// Sets the execution context for the next `execve` call in the current thread.
pub fn setexeccon(con: &str) -> Result<(), String> {
    println!("[nilrt:selinux] Setting exec context to: {}", con);

    #[cfg(target_os = "linux")]
    {
        let paths = ["/proc/thread-self/attr/exec", "/proc/self/attr/exec"];
        let mut written = false;

        for path in &paths {
            if Path::new(path).exists() {
                match fs::write(path, format!("{}\0", con.trim())) {
                    Ok(_) => {
                        written = true;
                        break;
                    }
                    Err(e) => {
                        if Path::new("/sys/fs/selinux/enforce").exists() {
                            if let Ok(enforce_val) = fs::read_to_string("/sys/fs/selinux/enforce") {
                                if enforce_val.trim() == "1" {
                                    return Err(format!("SELinux is enforcing but writing {con} to {path} failed: {e}"));
                                }
                            }
                        }
                    }
                }
            }
        }

        if !written && Path::new("/sys/fs/selinux").is_dir() {
            return Err("SELinux filesystem is present but /proc attr/exec is unavailable".to_string());
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_selinux_mode_labels() {
        assert_eq!(SelinuxMode::Enforcing.as_str(), "Enforcing");
        assert_eq!(SelinuxMode::Permissive.as_str(), "Permissive");
        assert_eq!(SelinuxMode::Disabled.as_str(), "Disabled");
        assert_eq!(SelinuxMode::Unavailable.as_str(), "Unavailable");
    }

    #[test]
    fn test_selinux_mode_mock_parse() {
        fn parse_enforce_str(s: &str) -> SelinuxMode {
            match s.trim() {
                "1" => SelinuxMode::Enforcing,
                "0" => SelinuxMode::Permissive,
                _ => SelinuxMode::Unavailable,
            }
        }
        assert_eq!(parse_enforce_str("1\n"), SelinuxMode::Enforcing);
        assert_eq!(parse_enforce_str("0\n"), SelinuxMode::Permissive);
        assert_eq!(parse_enforce_str("invalid"), SelinuxMode::Unavailable);
    }

    #[test]
    fn test_setexeccon_does_not_panic() {
        let _ = setexeccon("nilos_app_t");
    }
}
