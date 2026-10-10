// nilinit/src/main.rs — Onuron OS PID 1: Mount, Disk Init, Mobile Storage Hierarchy, SELinux, Supervisor, Socket Activation
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};
use serde::Deserialize;

mod activate;
mod recovery;
mod supervisor;
use activate::SocketActivationManager;
use recovery::BootDecision;
use supervisor::{ServiceSpec, Supervisor};

/// Read the kernel command line and the persisted failed-boot counter, and
/// decide whether to continue a normal boot or drop into recovery.
fn decide_boot() -> BootDecision {
    let cmdline = fs::read_to_string("/proc/cmdline").unwrap_or_default();
    let previous = recovery::read_boot_count(recovery::BOOT_COUNT_PATH);
    recovery::plan_boot(&cmdline, previous)
}

/// Run the recovery menu and then reboot. Never returns.
fn enter_recovery(reason: &str) -> ! {
    kmsg(&format!(
        "\x1b[1;33m[ WARN ]\x1b[0m Entering recovery mode: {}",
        reason
    ));
    match Command::new("/usr/bin/nilrecovery").status() {
        Ok(status) => log_info(&format!("Recovery menu exited with {}", status)),
        Err(e) => log_warn(&format!("Could not start /usr/bin/nilrecovery: {}", e)),
    }
    #[cfg(target_os = "linux")]
    {
        unsafe {
            libc::sync();
            libc::reboot(libc::RB_AUTOBOOT);
        }
    }
    // If the reboot syscall failed (for example when not actually PID 1), do
    // not fall through into normal boot; stay parked so the operator can see
    // the failure.
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[derive(Deserialize, Clone)]
struct Service {
    name: String,
    exec: String,
    #[serde(default)]
    restart: String,
    #[serde(default)]
    socket_activation: Option<String>,
}

#[derive(Deserialize)]
struct Config {
    #[serde(default)]
    #[allow(dead_code)]
    version: Option<u32>,
    services: Vec<Service>,
}

impl Config {
    fn validate(&self) -> Result<(), String> {
        let mut seen = std::collections::HashSet::new();
        for s in &self.services {
            let name = s.name.trim();
            if name.is_empty() {
                return Err("Service name cannot be empty".into());
            }
            if !seen.insert(name) {
                return Err(format!("Duplicate service name: '{}'", name));
            }
            if s.exec.trim().is_empty() {
                return Err(format!("Service '{}' has empty exec path", name));
            }
        }
        Ok(())
    }
}

fn kmsg(msg: &str) {
    if let Ok(mut f) = fs::OpenOptions::new().write(true).open("/dev/kmsg") {
        let _ = writeln!(f, "<1>{}", msg);
    }
    println!("{}", msg);
    let _ = std::io::stdout().flush();
}

fn log_ok(msg: &str) {
    kmsg(&format!("\x1b[1;32m[  OK  ]\x1b[0m {}", msg));
}

fn log_warn(msg: &str) {
    kmsg(&format!("\x1b[1;33m[ WARN ]\x1b[0m {}", msg));
}

fn log_info(msg: &str) {
    kmsg(&format!("\x1b[1;36m[ INFO ]\x1b[0m {}", msg));
}

fn mount_early_fs() {
    #[cfg(target_os = "linux")]
    {
        let mounts = [
            ("proc", "/proc", "proc", 0),
            ("sysfs", "/sys", "sysfs", 0),
            ("devtmpfs", "/dev", "devtmpfs", 0),
            ("tmpfs", "/run", "tmpfs", 0),
            ("tmpfs", "/tmp", "tmpfs", 0),
            ("cgroup2", "/sys/fs/cgroup", "cgroup2", 0),
            ("selinuxfs", "/sys/fs/selinux", "selinuxfs", 0),
        ];

        for (src, target, fstype, flags) in mounts {
            let _ = fs::create_dir_all(target);
            let _ = nix::mount::mount(
                Some(src),
                target,
                Some(fstype),
                nix::mount::MsFlags::from_bits_truncate(flags),
                None::<&str>,
            );
        }
        let _ = fs::create_dir_all("/run/onuron");
        let _ = fs::create_dir_all("/run/onuron/ready");
        let _ = fs::create_dir_all("/run/nilos"); // backward-compatibility alias

        // Attach stdout/stderr to console or ttyS0
        unsafe {
            let mut fd = libc::open(b"/dev/console\0".as_ptr() as *const libc::c_char, libc::O_RDWR);
            if fd < 0 {
                fd = libc::open(b"/dev/ttyS0\0".as_ptr() as *const libc::c_char, libc::O_RDWR);
            }
            if fd >= 0 {
                libc::dup2(fd, 0);
                libc::dup2(fd, 1);
                libc::dup2(fd, 2);
                if fd > 2 { libc::close(fd); }
            }
        }
    }
    log_ok("Early virtual filesystems mounted (/proc, /sys, /dev, /run, /tmp)");
}

fn load_kernel_modules() {
    #[cfg(target_os = "linux")]
    {
        let module_dir = std::path::Path::new("/lib/modules/storage");
        if !module_dir.exists() {
            return;
        }

        // Required order: virtio_blk first for /dev/vda, then crc16, mbcache, jbd2, ext4
        let module_order = [
            "virtio_blk.ko",
            "crc16.ko",
            "mbcache.ko",
            "jbd2.ko",
            "ext4.ko",
        ];

        for mod_name in &module_order {
            let mod_path = module_dir.join(mod_name);
            if mod_path.exists() {
                if let Ok(file) = fs::File::open(&mod_path) {
                    use std::os::unix::io::AsRawFd;
                    let fd = file.as_raw_fd();
                    let param = std::ffi::CString::new("").unwrap();
                    let ret = unsafe {
                        libc::syscall(
                            libc::SYS_finit_module,
                            fd,
                            param.as_ptr(),
                            0 as libc::c_int,
                        )
                    };
                    if ret == 0 {
                        log_ok(&format!("Kernel module loaded: {}", mod_name));
                    } else {
                        let err = std::io::Error::last_os_error();
                        if err.raw_os_error() == Some(libc::EEXIST) {
                            log_ok(&format!("Kernel module already built-in/loaded: {}", mod_name));
                        } else {
                            log_warn(&format!("Failed loading module {}: {}", mod_name, err));
                        }
                    }
                }
            }
        }
    }
}

fn mount_data_partition() {
    load_kernel_modules();
    let _ = fs::create_dir_all("/data");

    #[cfg(target_os = "linux")]
    {
        // Try virtio-blk disk (/dev/vda) first, then IDE (/dev/sda), then fall back to tmpfs
        let candidates = ["/dev/vda", "/dev/vda1", "/dev/sda", "/dev/sda1", "/dev/hda"];
        let mut mounted = false;

        for dev in &candidates {
            // Wait briefly for device to appear
            let mut tries = 0;
            while tries < 10 && !std::path::Path::new(dev).exists() {
                thread::sleep(Duration::from_millis(100));
                tries += 1;
            }

            if std::path::Path::new(dev).exists() {
                // Try mounting as ext4/ext2
                let result = nix::mount::mount(
                    Some(*dev),
                    "/data",
                    Some("ext4"),
                    nix::mount::MsFlags::empty(),
                    None::<&str>,
                ).or_else(|_| {
                    nix::mount::mount(
                        Some(*dev),
                        "/data",
                        Some("ext2"),
                        nix::mount::MsFlags::empty(),
                        None::<&str>,
                    )
                });

                match result {
                    Ok(_) => {
                        log_ok(&format!("Persistent storage mounted: {} → /data", dev));
                        mounted = true;
                        break;
                    }
                    Err(e) => {
                        log_warn(&format!("Could not mount {} as ext4/ext2: {} — trying next", dev, e));
                    }
                }
            }
        }

        if !mounted {
            // Fall back to tmpfs — data will not persist across reboots
            let _ = nix::mount::mount(
                Some("tmpfs"),
                "/data",
                Some("tmpfs"),
                nix::mount::MsFlags::empty(),
                Some("size=64M"),
            );
            log_warn("No persistent disk found — /data is tmpfs (ephemeral)");
        }
    }

    // Standard Android/Linux mobile storage hierarchy:
    // /system, /vendor, /cache, /recovery, /metadata
    // /data/user, /data/app, /data/system, /data/media, /data/config
    for dir in &[
        "/system", "/vendor", "/cache", "/recovery", "/metadata",
        "/data/user", "/data/app", "/data/system", "/data/media",
        "/data/config", "/data/contacts", "/data/sms", "/data/logs",
        "/data/nilos", // backward compatibility for existing prototypes
    ] {
        let _ = fs::create_dir_all(dir);
    }
    log_ok("Mobile filesystem hierarchy initialized (/data/user, /data/app, /data/system, /system)");
}

fn check_live_install() {
    if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("onuron.install=1") || cmdline.contains("nilos.install=1") {
            log_info("Detected live installer mode! Launching nilinstall...");
            let _ = Command::new("/usr/bin/nilinstall").status();
        }
    }
}

fn load_selinux() {
    let policy_path = "/etc/selinux/targeted/policy/policy.33";
    let selinux_load_path = "/sys/fs/selinux/load";

    if !std::path::Path::new(selinux_load_path).exists() {
        log_info("SELinux filesystem not present at /sys/fs/selinux/load (kernel disabled or not mounted)");
        return;
    }

    let policy = match fs::read(policy_path) {
        Ok(p) => p,
        Err(e) => {
            log_warn(&format!("SELinux policy file not found at {}: {}", policy_path, e));
            return;
        }
    };

    match OpenOptions::new().write(true).open(selinux_load_path) {
        Ok(mut f) => match f.write_all(&policy) {
            Ok(_) => {
                log_ok("SELinux binary policy successfully committed to kernel (/sys/fs/selinux/load)");
                let mut verified_enforcing = false;
                let enforce_path = "/sys/fs/selinux/enforce";
                if let Ok(mut enforce_file) = OpenOptions::new().write(true).open(enforce_path) {
                    if enforce_file.write_all(b"1").is_ok() {
                        // Read back to verify kernel accepted enforcing mode
                        if let Ok(val) = fs::read_to_string(enforce_path) {
                            if val.trim() == "1" {
                                verified_enforcing = true;
                                log_ok("SELinux policy active and verified in enforcing mode (enforce=1)");
                            }
                        }
                    }
                }
                if !verified_enforcing {
                    log_warn("SELinux policy loaded, but enforce=1 verification failed");
                }
            }
            Err(e) => {
                log_warn(&format!("Failed writing SELinux binary policy to kernel: {}", e));
            }
        },
        Err(e) => {
            log_warn(&format!("Cannot open {} with write permissions: {}", selinux_load_path, e));
        }
    }
}

fn setup_cgroups() {
    let cgroup_root = std::path::Path::new("/sys/fs/cgroup");
    let controllers_file = cgroup_root.join("cgroup.controllers");
    let subtree_control = cgroup_root.join("cgroup.subtree_control");

    let system_slice = cgroup_root.join("system.slice");
    let onuron_slice = cgroup_root.join("onuron.slice");
    let nilos_slice = cgroup_root.join("nilos.slice");

    let _ = fs::create_dir_all(&system_slice);
    let _ = fs::create_dir_all(&onuron_slice);
    let _ = fs::create_dir_all(&nilos_slice);

    if !controllers_file.exists() {
        log_info("cgroups v2 controllers file not present (kernel restricted or virtual)");
        return;
    }

    if let Ok(controllers_text) = fs::read_to_string(&controllers_file) {
        let available: Vec<&str> = controllers_text.split_whitespace().collect();
        let targets = ["cpu", "memory", "pids", "io"];
        let to_enable: Vec<String> = targets
            .iter()
            .filter(|t| available.contains(t))
            .map(|t| format!("+{}", t))
            .collect();

        if !to_enable.is_empty() {
            let enable_cmd = to_enable.join(" ");
            let _ = fs::write(&subtree_control, &enable_cmd);
            let onuron_subtree = onuron_slice.join("cgroup.subtree_control");
            let _ = fs::write(&onuron_subtree, &enable_cmd);

            let active = fs::read_to_string(&subtree_control).unwrap_or_default();
            let clean_active = active.trim();
            if !clean_active.is_empty() {
                log_ok(&format!(
                    "cgroups v2 controllers verified active: [{}] (system.slice, onuron.slice)",
                    clean_active
                ));
                return;
            } else {
                log_warn("cgroups v2 subtree_control write did not activate requested controllers");
            }
        }
    }

    log_ok("Cgroups v2 control group initialized (/sys/fs/cgroup/onuron.slice)");
}

fn write_system_env() {
    // Set ONURON_OOBE_DONE / NILOS_OOBE_DONE env variable for spawned services
    let oobe_done = std::path::Path::new("/data/config/oobe_done").exists()
        || std::path::Path::new("/data/nilos/oobe_done").exists();

    let env_content = format!(
        "ONURON_OOBE_DONE={}\nNILOS_OOBE_DONE={}\n",
        if oobe_done { "1" } else { "0" },
        if oobe_done { "1" } else { "0" }
    );
    let _ = fs::write("/run/onuron/env", &env_content);
    let _ = fs::write("/run/nilos/env", &env_content);
}

fn handle_system_shutdown(action: &str, supervisor: &mut Supervisor) {
    log_info(&format!("Initiating system {}", action));
    supervisor.shutdown();
    #[cfg(target_os = "linux")]
    unsafe {
        libc::sync();
        if action == "poweroff" || action == "halt" {
            libc::reboot(libc::RB_POWER_OFF);
        } else if action == "reboot" {
            libc::reboot(libc::RB_AUTOBOOT);
        }
    }
    log_ok("All storage buffers synchronized to disk. System halted safely.");
    std::process::exit(0);
}

fn check_persistence_test(supervisor: &mut Supervisor) {
    if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("onuron.test_persistence=write") {
            log_info("Executing automated persistence test: WRITE phase...");
            let marker_path = "/data/persistence_test_marker.bin";
            let test_payload = b"ONURON_EXT4_PERSISTENCE_TEST_PAYLOAD_V1_VERIFIED";

            // Verify /data is NOT tmpfs
            if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
                let is_block_mount = mounts.lines().any(|l| l.contains("/data ext4") || l.contains("/data ext2"));
                if !is_block_mount {
                    kmsg("\x1b[1;31m[ FAIL ]\x1b[0m Persistence test failed: /data is not mounted from real ext block device!");
                    handle_system_shutdown("halt", supervisor);
                    return;
                }
            }

            match fs::write(marker_path, test_payload) {
                Ok(_) => {
                    #[cfg(target_os = "linux")]
                    unsafe { libc::sync(); }
                    log_ok("Persistence marker written and synced to /data");
                    log_ok("Persistence test WRITE phase completed successfully");
                    handle_system_shutdown("poweroff", supervisor);
                }
                Err(e) => {
                    kmsg(&format!("\x1b[1;31m[ FAIL ]\x1b[0m Failed writing persistence marker: {}", e));
                    handle_system_shutdown("halt", supervisor);
                }
            }
        } else if cmdline.contains("onuron.test_persistence=verify") {
            log_info("Executing automated persistence test: VERIFY phase...");
            let marker_path = "/data/persistence_test_marker.bin";
            let expected_payload = b"ONURON_EXT4_PERSISTENCE_TEST_PAYLOAD_V1_VERIFIED";

            // Verify /data is NOT tmpfs
            if let Ok(mounts) = fs::read_to_string("/proc/mounts") {
                let is_block_mount = mounts.lines().any(|l| l.contains("/data ext4") || l.contains("/data ext2"));
                if !is_block_mount {
                    kmsg("\x1b[1;31m[ FAIL ]\x1b[0m Persistence test verify failed: /data is tmpfs!");
                    handle_system_shutdown("halt", supervisor);
                    return;
                }
            }

            match fs::read(marker_path) {
                Ok(content) if content == expected_payload => {
                    log_ok("Persistence marker verified across reboot: payload matches byte-for-byte");
                    log_ok("Persistence test VERIFY phase completed successfully");
                    handle_system_shutdown("poweroff", supervisor);
                }
                Ok(content) => {
                    kmsg(&format!("\x1b[1;31m[ FAIL ]\x1b[0m Persistence marker corrupt (read {} bytes)", content.len()));
                    handle_system_shutdown("halt", supervisor);
                }
                Err(e) => {
                    kmsg(&format!("\x1b[1;31m[ FAIL ]\x1b[0m Persistence marker missing after reboot: {}", e));
                    handle_system_shutdown("halt", supervisor);
                }
            }
        }
    }
}

fn main() {
    mount_early_fs();

    kmsg("\x1b[1;36m=========================================================\x1b[0m");
    kmsg("\x1b[1;36m       Onuron OS Initializing (PID 1 System Init)        \x1b[0m");
    kmsg("\x1b[1;36m=========================================================\x1b[0m");

    setup_cgroups();
    mount_data_partition();
    load_selinux();

    // Recovery decision: an explicit kernel flag or too many consecutive
    // failed boots drops the device into `nilrecovery` instead of the UI.
    match decide_boot() {
        BootDecision::Recovery { reason } => enter_recovery(&reason.describe()),
        BootDecision::Normal { failed_boots } => {
            // Persist the incremented counter *before* services start; it is
            // only cleared once boot genuinely completes below. If we die
            // before that point, the next boot sees the higher count.
            if let Err(e) = recovery::write_boot_count(recovery::BOOT_COUNT_PATH, failed_boots) {
                log_warn(&format!("Could not persist boot counter: {}", e));
            }
        }
    }

    check_live_install();
    write_system_env();

    let boot_start = Instant::now();

    let oobe_done = std::path::Path::new("/data/config/oobe_done").exists()
        || std::path::Path::new("/data/nilos/oobe_done").exists();

    if !oobe_done {
        log_info("First boot detected — OOBE setup wizard will be launched by shell");
    } else {
        log_ok("System configured — normal operational mode active");
    }

    let config_str = fs::read_to_string("/etc/nilos/services.toml")
        .unwrap_or_else(|_| include_str!("../../etc/nilos/services.toml").to_string());
    let config: Config = match toml::from_str::<Config>(&config_str) {
        Ok(c) => match c.validate() {
            Ok(()) => c,
            Err(err) => {
                kmsg(&format!("\x1b[1;31m[ FATAL ]\x1b[0m Invalid services.toml schema: {}", err));
                loop { thread::sleep(Duration::from_secs(60)); }
            }
        },
        Err(e) => {
            kmsg(&format!("\x1b[1;31m[ FATAL ]\x1b[0m Could not parse services.toml: {}", e));
            loop { thread::sleep(Duration::from_secs(60)); }
        }
    };

    let mut activator = SocketActivationManager::new();

    for s in &config.services {
        if let Some(sock_path) = &s.socket_activation {
            let _ = activator.register(&s.name, sock_path);
        }
    }
    log_ok("Socket activation manager initialized");

    // Only start services that are not purely on-demand socket activated
    let services: Vec<ServiceSpec> = config
        .services
        .iter()
        .filter(|s| s.socket_activation.is_none())
        .map(|s| ServiceSpec {
            name: s.name.clone(),
            exec: s.exec.clone(),
            restart: s.restart.clone(),
        })
        .collect();
    let mut supervisor = Supervisor::new(services);
    supervisor.start_all();

    // Give services a brief moment to settle (50ms) to detect immediate launch crashes
    thread::sleep(Duration::from_millis(50));

    // Core services that MUST be running for Onuron OS to be considered operational
    let core_services = ["nild", "nilkeyd", "nilbus", "netd", "audiod", "powerd", "nilshell"];
    let mut boot_failed = match supervisor.check_core_health(&core_services) {
        Ok(()) => false,
        Err(failed) => {
            for core in failed {
                kmsg(&format!("\x1b[1;31m[ FAIL ]\x1b[0m Core service '{}' failed to start or crashed!", core));
            }
            true
        }
    };

    // Core services readiness probes (socket existence and service liveness)
    let readiness_probes: [(&str, Option<&std::path::Path>); 7] = [
        ("nild", Some(std::path::Path::new("/run/onuron/ready/nild"))),
        ("nilkeyd", Some(std::path::Path::new("/run/nilos/keyd.sock"))),
        ("nilbus", Some(std::path::Path::new("/run/nilos/bus.sock"))),
        ("netd", Some(std::path::Path::new("/run/onuron/net.sock"))),
        ("audiod", Some(std::path::Path::new("/run/onuron/audio.sock"))),
        ("powerd", Some(std::path::Path::new("/run/onuron/power.sock"))),
        ("nilshell", Some(std::path::Path::new("/run/onuron/ready/nilshell"))),
    ];

    // Bounded retry loop for readiness checks
    let max_readiness_wait = Duration::from_millis(5000);
    let poll_interval = Duration::from_millis(100);
    let readiness_start = Instant::now();
    let mut readiness_ok = false;
    let mut last_not_ready = Vec::new();

    while readiness_start.elapsed() < max_readiness_wait {
        supervisor.tick();
        match supervisor.check_readiness(&readiness_probes) {
            Ok(()) => {
                readiness_ok = true;
                break;
            }
            Err(not_ready) => {
                last_not_ready = not_ready;
                if let Err(failed) = supervisor.check_core_health(&core_services) {
                    for core in failed {
                        kmsg(&format!("\x1b[1;31m[ FAIL ]\x1b[0m Core service '{}' crashed during boot initialization!", core));
                    }
                    boot_failed = true;
                    break;
                }
                thread::sleep(poll_interval);
            }
        }
    }

    if boot_failed {
        kmsg("\x1b[1;31m[ FATAL ]\x1b[0m Onuron OS boot failed: core services not operational");
    } else if !readiness_ok {
        kmsg(&format!(
            "\x1b[1;33m[ WARN ]\x1b[0m Core service readiness timed out after {:.2} ms (pending: {:?})",
            readiness_start.elapsed().as_secs_f64() * 1000.0,
            last_not_ready
        ));
        kmsg("\x1b[1;33m[ WARN ]\x1b[0m Onuron OS boot degraded: core readiness incomplete");
    } else {
        log_ok("Core service sockets verified ready");
        log_ok(&format!("Core services verified healthy ({}/{} active)", core_services.len(), core_services.len()));
        log_ok(&format!(
            "Onuron OS boot completed in {:.2} ms ({} services active, core services verified healthy)",
            boot_start.elapsed().as_secs_f64() * 1000.0,
            supervisor.live_count()
        ));

        // Boot reached completion: clear failed-boot counter
        if let Err(e) = recovery::write_boot_count(
            recovery::BOOT_COUNT_PATH,
            recovery::successful_boot_counter(),
        ) {
            log_warn(&format!("Could not clear boot counter: {}", e));
        }
    }

    // Check if booted with persistence test flag
    check_persistence_test(&mut supervisor);

    // Supervision Loop
    loop {
        if !supervisor.tick() {
            break;
        }

        // On-demand socket activation poll
        let pending = activator.check_pending();
        for name in pending {
            if !supervisor.is_running(&name) {
                if let Some(spec) = config.services.iter().find(|s| s.name == name) {
                    log_info(&format!("Socket activation triggered for {}", name));
                    supervisor.spawn_with_fd(
                        &ServiceSpec {
                            name: spec.name.clone(),
                            exec: spec.exec.clone(),
                            restart: spec.restart.clone(),
                        },
                        activator.get_raw_fd(&name),
                    );
                }
            }
        }

        // Check for pending shutdown / reboot requests
        let power_req = std::path::Path::new("/run/onuron/power_action");
        if power_req.exists() {
            if let Ok(action) = fs::read_to_string(power_req) {
                let action = action.trim();
                let _ = fs::remove_file(power_req);
                handle_system_shutdown(action, &mut supervisor);
                break;
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_embedded_services_config_is_valid() {
        let config_str = include_str!("../../etc/nilos/services.toml");
        let config: Config = toml::from_str(config_str).expect("Embedded services.toml must be valid TOML");
        assert!(config.validate().is_ok(), "Embedded services.toml must pass schema validation");
        assert!(config.services.len() >= 7, "Must contain all core mobile daemons");
    }

    #[test]
    fn test_services_validation_rejects_duplicates_and_empty() {
        let duplicate_toml = r#"
            [[services]]
            name = "nild"
            exec = "/usr/bin/nild"

            [[services]]
            name = "nild"
            exec = "/usr/bin/nild_other"
        "#;
        let config: Config = toml::from_str(duplicate_toml).unwrap();
        let err = config.validate().unwrap_err();
        assert!(err.contains("Duplicate service name: 'nild'"));

        let empty_exec_toml = r#"
            [[services]]
            name = "powerd"
            exec = "   "
        "#;
        let config2: Config = toml::from_str(empty_exec_toml).unwrap();
        let err2 = config2.validate().unwrap_err();
        assert!(err2.contains("empty exec path"));
    }
}
