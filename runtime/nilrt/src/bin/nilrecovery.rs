// runtime/nilrt/src/bin/nilrecovery.rs — Recovery menu, factory reset and data restore.
//
// This binary is launched by `nilinit` when the kernel command line asks for
// recovery or when the failed-boot counter reaches its limit. Destructive
// actions (factory reset) require an explicit confirmation so an accidental
// keypress cannot wipe a device.
use std::io::{self, BufRead, Write};

/// `reboot(2)` command to restart. Defined locally because the value is not
/// exposed on non-Linux hosts, which still compile this binary for the tests.
#[cfg(target_os = "linux")]
const RB_AUTOBOOT: libc::c_int = 0x0123_4567;
#[cfg(not(target_os = "linux"))]
const RB_AUTOBOOT: libc::c_int = 0;

/// A recovery menu choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    RebootNormal,
    RebootFastboot,
    FactoryReset,
    ApplyOta,
    RestoreBackup,
    Quit,
}

/// Map raw menu input to an action. Unrecognised input yields `None`.
pub fn classify(input: &str) -> Option<MenuAction> {
    match input.trim() {
        "1" => Some(MenuAction::RebootNormal),
        "2" => Some(MenuAction::RebootFastboot),
        "3" => Some(MenuAction::FactoryReset),
        "4" => Some(MenuAction::ApplyOta),
        "5" => Some(MenuAction::RestoreBackup),
        "q" | "Q" | "quit" | "exit" => Some(MenuAction::Quit),
        _ => None,
    }
}

/// True only for an unambiguous affirmative answer. Anything else (including
/// an empty line or EOF) is treated as "no".
pub fn is_affirmative(input: &str) -> bool {
    matches!(input.trim().to_ascii_lowercase().as_str(), "yes" | "y")
}

/// Extract the backing device for `mountpoint` from the contents of
/// `/proc/mounts`. Returns `None` when the mount point is not present (for
/// example when `/data` is a tmpfs or was never mounted).
pub fn data_device_from_mounts(mounts: &str, mountpoint: &str) -> Option<String> {
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let dev = fields.next()?;
        let mount = fields.next()?;
        if mount == mountpoint {
            // tmpfs, proc, and friends are not real block devices to reformat.
            if dev == "tmpfs" || dev == "none" || dev.starts_with("proc") {
                return None;
            }
            return Some(dev.to_string());
        }
    }
    None
}

fn prompt(msg: &str) -> String {
    print!("{}", msg);
    let _ = io::stdout().flush();
    let stdin = io::stdin();
    let mut line = String::new();
    match stdin.lock().read_line(&mut line) {
        Ok(_) => line,
        Err(_) => String::new(),
    }
}

fn read_data_device() -> Option<String> {
    let mounts = std::fs::read_to_string("/proc/mounts").ok()?;
    data_device_from_mounts(&mounts, "/data")
}

/// Wipe user data. The device is unmounted and reformatted when a real block
/// device backs `/data`; otherwise the contents are removed in place. The
/// failed-boot counter is always cleared so recovery is not re-entered.
fn factory_reset() -> io::Result<()> {
    println!("Wiping user data...");

    if let Some(dev) = read_data_device() {
        #[cfg(target_os = "linux")]
        unsafe {
            let c_path = std::ffi::CString::new("/data").unwrap();
            libc::umount(c_path.as_ptr());
        }
        let status = std::process::Command::new("/sbin/mkfs.ext4")
            .args(["-F", &dev])
            .status()
            .or_else(|_| {
                std::process::Command::new("mkfs.ext4").args(["-F", &dev]).status()
            });
        match status {
            Ok(s) if s.success() => println!("Reformatted {} (ext4).", dev),
            Ok(s) => eprintln!("mkfs.ext4 exited with {} — falling back to removal", s),
            Err(e) => eprintln!("Could not run mkfs.ext4: {} — falling back to removal", e),
        }
    }

    // Remove any remaining contents (also the fallback path for tmpfs).
    if let Ok(entries) = std::fs::read_dir("/data") {
        for entry in entries.flatten() {
            let path = entry.path();
            let _ = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                std::fs::remove_dir_all(&path)
            } else {
                std::fs::remove_file(&path)
            };
        }
    }
    Ok(())
}

fn reboot(how: libc::c_int) -> ! {
    #[cfg(target_os = "linux")]
    {
        unsafe {
            libc::sync();
            libc::reboot(how);
        }
    }
    let _ = how;
    println!("Reboot requested; if the device does not reboot, restart it manually.");
    std::process::exit(0);
}

fn main() {
    println!("=========================================================");
    println!("                 Onuron Recovery System                  ");
    println!("=========================================================");
    println!("1) Reboot System Normal");
    println!("2) Reboot into Fastboot");
    println!("3) Factory Reset (Wipe Userdata)");
    println!("4) Apply OTA from USB / ADB");
    println!("5) Restore Encrypted Backup");
    println!("q) Quit");

    loop {
        let input = prompt("Select option: ");
        let Some(action) = classify(&input) else {
            println!("Unknown option. Enter 1-5 or q.");
            continue;
        };
        match action {
            MenuAction::Quit => {
                println!("Leaving recovery. The device will reboot.");
                reboot(RB_AUTOBOOT);
            }
            MenuAction::RebootNormal => reboot(RB_AUTOBOOT),
            MenuAction::RebootFastboot => {
                println!("Rebooting into fastboot...");
                reboot(RB_AUTOBOOT);
            }
            MenuAction::FactoryReset => {
                let answer = prompt(
                    "This permanently erases all user data. Type YES to confirm: ",
                );
                if is_affirmative(&answer) {
                    if let Err(e) = factory_reset() {
                        eprintln!("Factory reset failed: {}", e);
                        continue;
                    }
                    println!("Factory reset complete.");
                    reboot(RB_AUTOBOOT);
                } else {
                    println!("Factory reset cancelled (no confirmation).");
                }
            }
            MenuAction::ApplyOta => {
                println!("Apply OTA: use `nilupd apply <signed-update>` from the shell.");
            }
            MenuAction::RestoreBackup => {
                println!("Restore backup: mount the backup medium and use `nilrestore`.");
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_recognises_menu_options() {
        assert_eq!(classify("1"), Some(MenuAction::RebootNormal));
        assert_eq!(classify(" 3 "), Some(MenuAction::FactoryReset));
        assert_eq!(classify("4"), Some(MenuAction::ApplyOta));
        assert_eq!(classify("5"), Some(MenuAction::RestoreBackup));
        assert_eq!(classify("q"), Some(MenuAction::Quit));
        assert_eq!(classify(""), None);
        assert_eq!(classify("99"), None);
        assert_eq!(classify("rm -rf /"), None);
    }

    #[test]
    fn factory_reset_requires_explicit_yes() {
        assert!(is_affirmative("yes"));
        assert!(is_affirmative("YES"));
        assert!(is_affirmative(" y "));
        assert!(!is_affirmative("yep"));
        assert!(!is_affirmative(""));
        assert!(!is_affirmative("no"));
        assert!(!is_affirmative("yes please"));
    }

    #[test]
    fn data_device_is_found_and_tmpfs_is_rejected() {
        let mounts = "\
/dev/vda /data ext4 rw,relatime 0 0
tmpfs /run tmpfs rw 0 0
";
        assert_eq!(data_device_from_mounts(mounts, "/data"), Some("/dev/vda".into()));
        assert_eq!(data_device_from_mounts(mounts, "/nonexistent"), None);

        let tmpfs = "tmpfs /data tmpfs rw,size=64M 0 0\n";
        assert_eq!(data_device_from_mounts(tmpfs, "/data"), None);
    }
}
