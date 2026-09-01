// nilinit/src/main.rs — PID 1: Mount, Disk Init, OOBE Flag, SELinux, Supervisor, Socket Activation
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Write;
use std::process::{Child, Command};
use std::thread;
use std::time::{Duration, Instant};
use serde::Deserialize;

mod activate;
use activate::SocketActivationManager;

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
        let _ = fs::create_dir_all("/run/nilos");

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
    kmsg("[nilinit] Early virtual filesystems mounted (/proc, /sys, /dev, /run, /tmp).");
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
                        kmsg(&format!("[nilinit] Data partition mounted: {} → /data", dev));
                        mounted = true;
                        break;
                    }
                    Err(e) => {
                        kmsg(&format!("[nilinit] Could not mount {} as ext4/ext2: {} — trying next", dev, e));
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
            kmsg("[nilinit] WARNING: No persistent disk found — /data is tmpfs (ephemeral)");
        }
    }

    // Ensure essential data subdirectories exist
    for dir in &[
        "/data/nilos", "/data/app", "/data/user", "/data/config",
        "/data/contacts", "/data/sms", "/data/media", "/data/logs",
    ] {
        let _ = fs::create_dir_all(dir);
    }
    kmsg("[nilinit] Data directories initialized under /data.");
}

fn check_live_install() {
    if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("nilos.install=1") {
            kmsg("[nilinit] Detected live installer mode! Launching nilinstall...");
            let _ = Command::new("/usr/bin/nilinstall").status();
        }
    }
}

fn load_selinux() {
    if let Ok(mut f) = File::open("/sys/fs/selinux/load") {
        if let Ok(policy) = fs::read("/etc/selinux/targeted/policy/policy.33") {
            let _ = f.write_all(&policy);
            kmsg("[nilinit] SELinux policy loaded in enforcing mode.");
        }
    }
}

fn setup_cgroups() {
    let _ = fs::create_dir_all("/sys/fs/cgroup/nilos.slice");
}

fn write_system_env() {
    // Set NILOS_OOBE_DONE env variable for spawned services to read
    let oobe_done = std::path::Path::new("/data/nilos/oobe_done").exists();
    let _ = fs::write("/run/nilos/env", format!("NILOS_OOBE_DONE={}\n", if oobe_done { "1" } else { "0" }));
}

fn main() {
    mount_early_fs();

    kmsg("=========================================================");
    kmsg("        NilOS Initializing (PID 1 System Supervisor)     ");
    kmsg("=========================================================");

    setup_cgroups();
    mount_data_partition();
    load_selinux();
    check_live_install();
    write_system_env();

    let boot_start = Instant::now();

    let oobe_done = std::path::Path::new("/data/nilos/oobe_done").exists();
    if !oobe_done {
        kmsg("[nilinit] First boot detected — OOBE wizard will be shown by nilshell.");
    } else {
        kmsg("[nilinit] System configured — starting normally.");
    }

    let config_str = fs::read_to_string("/etc/nilos/services.toml")
        .unwrap_or_else(|_| include_str!("../../etc/nilos/services.toml").to_string());
    let config: Config = match toml::from_str(&config_str) {
        Ok(c) => c,
        Err(e) => {
            kmsg(&format!("[nilinit] FATAL: Could not parse services.toml: {}", e));
            loop { thread::sleep(Duration::from_secs(60)); }
        }
    };

    let mut activator = SocketActivationManager::new();
    let mut running: HashMap<String, Child> = HashMap::new();

    for s in &config.services {
        if let Some(sock_path) = &s.socket_activation {
            let _ = activator.register(&s.name, sock_path);
        }
    }

    for s in &config.services {
        if s.socket_activation.is_none() {
            kmsg(&format!("[nilinit] Spawning service: {}", s.name));
            let parts: Vec<&str> = s.exec.split_whitespace().collect();
            if let Some((bin, args)) = parts.split_first() {
                match Command::new(bin).args(args).spawn() {
                    Ok(child) => {
                        kmsg(&format!("[nilinit] Service '{}' started (PID {})", s.name, child.id()));
                        running.insert(s.name.clone(), child);
                    }
                    Err(e) => {
                        kmsg(&format!("[nilinit] Note: Service '{}' not available: {}", s.name, e));
                    }
                }
            }
        }
    }

    kmsg(&format!(
        "[nilinit] System boot complete in {:.2} ms. {} service(s) active.",
        boot_start.elapsed().as_secs_f64() * 1000.0,
        running.len()
    ));

    // Supervision Loop
    loop {
        let pending = activator.check_pending();
        for name in pending {
            if !running.contains_key(&name) {
                if let Some(s) = config.services.iter().find(|svc| svc.name == name) {
                    kmsg(&format!("[nilinit:activate] Waking lazy service: {}", name));
                    let parts: Vec<&str> = s.exec.split_whitespace().collect();
                    if let Some((bin, args)) = parts.split_first() {
                        let mut cmd = Command::new(bin);
                        cmd.args(args);
                        if let Some(_fd) = activator.get_raw_fd(&name) {
                            cmd.env("LISTEN_FDS", "1");
                            cmd.env("LISTEN_FDNAMES", &name);
                        }
                        if let Ok(child) = cmd.spawn() {
                            running.insert(name.clone(), child);
                        }
                    }
                }
            }
        }

        let mut dead = Vec::new();
        for (name, child) in running.iter_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    kmsg(&format!("[nilinit] Service '{}' exited ({})", name, status));
                    dead.push(name.clone());
                }
                Ok(None) => {}
                Err(e) => {
                    kmsg(&format!("[nilinit] Error polling '{}': {}", name, e));
                    dead.push(name.clone());
                }
            }
        }

        for name in dead {
            running.remove(&name);
            if let Some(s) = config.services.iter().find(|svc| svc.name == name) {
                if s.restart == "always" {
                    kmsg(&format!("[nilinit] Auto-restarting: {}", name));
                    let parts: Vec<&str> = s.exec.split_whitespace().collect();
                    if let Some((bin, args)) = parts.split_first() {
                        if let Ok(child) = Command::new(bin).args(args).spawn() {
                            running.insert(name, child);
                        }
                    }
                }
            }
        }

        thread::sleep(Duration::from_millis(500));
    }
}
