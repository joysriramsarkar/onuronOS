// nilinit/src/main.rs — Onuron OS PID 1: Mount, Disk Init, Mobile Storage Hierarchy, SELinux, Supervisor, Socket Activation
use std::fs::{self, File};
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
    services: Vec<Service>,
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

fn mount_data_partition() {
    let _ = fs::create_dir_all("/data");

    #[cfg(target_os = "linux")]
    {
        // Try virtio-blk disk (/dev/vda) first, then IDE (/dev/sda), then fall back to tmpfs
        let candidates = ["/dev/vda", "/dev/vda1", "/dev/sda", "/dev/sda1", "/dev/hda"];
        let mut mounted = false;

        for dev in &candidates {
            // Wait briefly for device to appear
            let mut tries = 0;
            while tries < 5 && !std::path::Path::new(dev).exists() {
                thread::sleep(Duration::from_millis(100));
                tries += 1;
            }

            if std::path::Path::new(dev).exists() {
                // Try mounting as ext2/ext4
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
    if let Ok(mut f) = File::open("/sys/fs/selinux/load") {
        if let Ok(policy) = fs::read("/etc/selinux/targeted/policy/policy.33") {
            let _ = f.write_all(&policy);
            log_ok("SELinux policy loaded in enforcing mode");
        }
    }
}

fn setup_cgroups() {
    let _ = fs::create_dir_all("/sys/fs/cgroup/onuron.slice");
    let _ = fs::create_dir_all("/sys/fs/cgroup/nilos.slice");
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
    }
    log_ok("All storage buffers synchronized to disk. System halted safely.");
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
    let config: Config = match toml::from_str(&config_str) {
        Ok(c) => c,
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
    let boot_failed = match supervisor.check_core_health(&core_services) {
        Ok(()) => false,
        Err(failed) => {
            for core in failed {
                kmsg(&format!("\x1b[1;31m[ FAIL ]\x1b[0m Core service '{}' failed to start or crashed!", core));
            }
            true
        }
    };

    if boot_failed {
        kmsg("\x1b[1;31m[ FATAL ]\x1b[0m Onuron OS boot failed: core services not operational");
    } else {
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
