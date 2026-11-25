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

/// Where nilinit keeps the failed-boot counter (`nilinit/src/recovery.rs`).
/// The file is a bare decimal string; nilinit treats a missing or malformed
/// file as zero, so writing `0` here is equivalent to clearing it.
const BOOT_COUNT_PATH: &str = "/data/system/boot_count";

/// Filesystem type backing `mountpoint` according to `/proc/mounts` (the
/// third field). Needed to remount a device after an unmount when no
/// reformatting tool is shipped in the image.
pub fn data_fs_type_from_mounts(mounts: &str, mountpoint: &str) -> Option<String> {
    for line in mounts.lines() {
        let mut fields = line.split_whitespace();
        let _dev = fields.next()?;
        let mount = fields.next()?;
        if mount == mountpoint {
            return fields.next().map(str::to_string);
        }
    }
    None
}

/// Remove every entry directly under `dir` (files and directories), leaving
/// the directory itself in place.
fn remove_dir_contents(dir: &str) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            std::fs::remove_dir_all(&path)?;
        } else {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

/// Write the failed-boot counter in nilinit's on-disk format.
fn write_boot_counter(path: &str, value: u32) -> io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, value.to_string())
}

/// Wipe user data.
///
/// A real block device is reformatted when `mkfs.ext4` is available (the
/// fresh filesystem has no failed-boot counter by construction). When no
/// reformat tool ships in the image — the case today, see
/// `build/mkinitramfs.py` — the device is remounted with its recorded
/// filesystem type and its contents are removed in place, and the
/// failed-boot counter is cleared explicitly so the device cannot reboot
/// straight back into recovery. A `tmpfs`-backed `/data` is wiped in place
/// the same way.
///
/// A device that cannot be remounted is reported as an error rather than
/// silently left populated.
fn factory_reset() -> io::Result<()> {
    println!("Wiping user data...");

    if let Some(dev) = read_data_device() {
        // Remember the filesystem type before unmounting so the in-place
        // fallback can remount the device.
        let fs_type = std::fs::read_to_string("/proc/mounts")
            .ok()
            .and_then(|mounts| data_fs_type_from_mounts(&mounts, "/data"));

        #[cfg(target_os = "linux")]
        unsafe {
            let c_path = std::ffi::CString::new("/data").unwrap();
            libc::umount(c_path.as_ptr());
        }

        let status = std::process::Command::new("/sbin/mkfs.ext4")
            .args(["-F", &dev])
            .status()
            .or_else(|_| {
                std::process::Command::new("mkfs.ext4")
                    .args(["-F", &dev])
                    .status()
            });
        match status {
            Ok(s) if s.success() => println!("Reformatted {} (ext4).", dev),
            Ok(s) => {
                eprintln!("mkfs.ext4 exited with {} — wiping in place instead", s);
                wipe_device_in_place(&dev, fs_type.as_deref())?;
            }
            Err(e) => {
                eprintln!("Could not run mkfs.ext4: {} — wiping in place instead", e);
                wipe_device_in_place(&dev, fs_type.as_deref())?;
            }
        }
    } else {
        // /data is tmpfs (or not a block device): wipe the contents in place
        // and clear the counter so the next boot stays out of recovery.
        remove_dir_contents("/data")?;
        write_boot_counter(BOOT_COUNT_PATH, 0)?;
    }
    Ok(())
}

/// The unmount happened but no reformat tool ran: remount `dev` on `/data`
/// and remove its contents, clearing the failed-boot counter afterwards.
/// Returns an error when the device cannot be remounted, so a failed reset
/// is reported instead of silently leaving user data in place.
fn wipe_device_in_place(dev: &str, fs_type: Option<&str>) -> io::Result<()> {
    if !data_remounted(dev, fs_type) {
        return Err(io::Error::other(format!(
            "could not remount {} after unmount; user data left untouched",
            dev
        )));
    }
    remove_dir_contents("/data")?;
    write_boot_counter(BOOT_COUNT_PATH, 0)?;
    Ok(())
}

/// True when `/data` is mounted again after the unmount. Tries the recorded
/// filesystem type first, then ext4/ext2. Non-Linux dev hosts cannot have a
/// dev-backed `/data`, so the wipe is refused there rather than faked.
fn data_remounted(dev: &str, fs_type: Option<&str>) -> bool {
    remount_data_inner(dev, fs_type)
}

#[cfg(target_os = "linux")]
fn remount_data_inner(dev: &str, fs_type: Option<&str>) -> bool {
    let Ok(c_dev) = std::ffi::CString::new(dev) else {
        return false;
    };
    let Ok(c_data) = std::ffi::CString::new("/data") else {
        return false;
    };
    let mut types: Vec<&str> = Vec::new();
    if let Some(t) = fs_type {
        types.push(t);
    }
    for fallback in ["ext4", "ext2"] {
        if !types.contains(&fallback) {
            types.push(fallback);
        }
    }
    for ty in types {
        let Ok(c_ty) = std::ffi::CString::new(ty) else {
            continue;
        };
        let ret = unsafe {
            libc::mount(
                c_dev.as_ptr(),
                c_data.as_ptr(),
                c_ty.as_ptr(),
                0,
                std::ptr::null(),
            )
        };
        if ret == 0 {
            return true;
        }
    }
    false
}

#[cfg(not(target_os = "linux"))]
fn remount_data_inner(_dev: &str, _fs_type: Option<&str>) -> bool {
    false
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
                let answer = prompt("This permanently erases all user data. Type YES to confirm: ");
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
        assert_eq!(
            data_device_from_mounts(mounts, "/data"),
            Some("/dev/vda".into())
        );
        assert_eq!(data_device_from_mounts(mounts, "/nonexistent"), None);

        let tmpfs = "tmpfs /data tmpfs rw,size=64M 0 0\n";
        assert_eq!(data_device_from_mounts(tmpfs, "/data"), None);
    }

    #[test]
    fn fs_type_is_extracted_for_the_in_place_remount() {
        let mounts = "\
/dev/vda /data ext4 rw,relatime 0 0
tmpfs /run tmpfs rw 0 0
/dev/vdb /media/usb vfat ro 0 0
";
        assert_eq!(
            data_fs_type_from_mounts(mounts, "/data").as_deref(),
            Some("ext4")
        );
        assert_eq!(
            data_fs_type_from_mounts(mounts, "/run").as_deref(),
            Some("tmpfs")
        );
        assert_eq!(
            data_fs_type_from_mounts(mounts, "/media/usb").as_deref(),
            Some("vfat")
        );
        assert_eq!(data_fs_type_from_mounts(mounts, "/not/mounted"), None);
    }

    #[test]
    fn remove_dir_contents_clears_files_and_directories_but_keeps_dir() {
        let dir = std::env::temp_dir().join(format!(
            "onuron-recov-wipe-{}-{:x}",
            std::process::id(),
            now_nanos()
        ));
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("file.txt"), b"x").unwrap();
        std::fs::write(dir.join("nested/inner.txt"), b"y").unwrap();

        remove_dir_contents(dir.to_str().unwrap()).unwrap();

        assert!(dir.is_dir(), "the wiped directory itself must remain");
        assert!(std::fs::read_dir(&dir).unwrap().next().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn boot_counter_write_matches_nilinit_disk_format() {
        let dir = std::env::temp_dir().join(format!(
            "onuron-recov-count-{}-{:x}",
            std::process::id(),
            now_nanos()
        ));
        let path = dir.join("system/boot_count");

        // Creating the parent directory is part of the contract: nilinit's
        // write_boot_count does the same when the hierarchy is fresh.
        write_boot_counter(path.to_str().unwrap(), 0).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "0");

        // A bare decimal, exactly what nilinit's parse_boot_count reads back.
        write_boot_counter(path.to_str().unwrap(), 3).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "3");

        let _ = std::fs::remove_dir_all(&dir);
    }

    // Tiny helper so temp paths unique per run need no extra dependency.
    fn now_nanos() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}
