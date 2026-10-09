// runtime/nilrt/src/selinux.rs — SELinux transition helpers
#[cfg(target_os = "linux")]
use std::fs;
#[cfg(target_os = "linux")]
use std::path::Path;

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
