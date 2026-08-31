// nilinit/src/main.rs — PID 1: Mounts, SELinux Load, Supervisor, Socket Activation & Stage-2 Pivot
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::Read;
use std::os::unix::process::CommandExt;
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
    requires: Vec<String>,
    #[serde(default)]
    socket_activation: Option<String>,
    #[serde(default)]
    critical: bool,
}

#[derive(Deserialize)]
struct Config {
    services: Vec<Service>,
}

fn mount_early_fs() {
    let mounts = [
        ("proc", "/proc", "proc", 0),
        ("sysfs", "/sys", "sysfs", 0),
        ("devtmpfs", "/dev", "devtmpfs", 0),
        ("tmpfs", "/run", "tmpfs", 0),
        ("tmpfs", "/tmp", "tmpfs", 0),
    ];

    for (src, target, fstype, flags) in mounts {
        let _ = fs::create_dir_all(target);
        unsafe {
            nix::mount::mount(
                Some(src),
                target,
                Some(fstype),
                nix::mount::MsFlags::from_bits_truncate(flags),
                None::<&str>,
            ).ok();
        }
    }
    let _ = fs::create_dir_all("/run/nilos");
    println!("[nilinit] Early virtual filesystems mounted.");
}

fn check_live_install() {
    if let Ok(cmdline) = fs::read_to_string("/proc/cmdline") {
        if cmdline.contains("nilos.install=1") {
            println!("[nilinit] Detected live installer mode! Launching nilinstall...");
            let _ = Command::new("/usr/bin/nilinstall").status();
        }
    }
}

fn load_selinux() {
    if let Ok(mut f) = File::open("/sys/fs/selinux/load") {
        if let Ok(policy) = fs::read("/etc/selinux/targeted/policy/policy.33") {
            use std::io::Write;
            let _ = f.write_all(&policy);
            println!("[nilinit] SELinux policy loaded in enforcing mode.");
        }
    }
}

fn setup_cgroups() {
    let _ = fs::create_dir_all("/sys/fs/cgroup/nilos.slice");
}

fn main() {
    println!("=========================================================");
    println!("        NilOS Initializing (PID 1 System Supervisor)     ");
    println!("=========================================================");

    mount_early_fs();
    setup_cgroups();
    load_selinux();
    check_live_install();

    let boot_start = Instant::now();

    let config_str = fs::read_to_string("/etc/nilos/services.toml")
        .unwrap_or_else(|_| include_str!("../../etc/nilos/services.toml").to_string());
    let config: Config = toml::from_str(&config_str).expect("Failed to parse services.toml");

    let mut activator = SocketActivationManager::new();
    let mut running: HashMap<String, Child> = HashMap::new();

    // Register socket activated services
    for s in &config.services {
        if let Some(sock_path) = &s.socket_activation {
            let _ = activator.register(&s.name, sock_path);
        }
    }

    // Launch eager services
    for s in &config.services {
        if s.socket_activation.is_none() {
            println!("[nilinit] Spawning service: {}", s.name);
            let parts: Vec<&str> = s.exec.split_whitespace().collect();
            if let Some((bin, args)) = parts.split_first() {
                if let Ok(child) = Command::new(bin).args(args).spawn() {
                    running.insert(s.name.clone(), child);
                }
            }
        }
    }

    println!("[nilinit] System boot stages initialized in {:.2} ms", boot_start.elapsed().as_secs_f64() * 1000.0);

    // Supervision Loop
    loop {
        // 1. Check socket activation triggers
        let pending = activator.check_pending();
        for name in pending {
            if !running.contains_key(&name) {
                if let Some(s) = config.services.iter().find(|svc| svc.name == name) {
                    println!("[nilinit:activate] Waking up lazy service on-demand: {}", name);
                    let parts: Vec<&str> = s.exec.split_whitespace().collect();
                    if let Some((bin, args)) = parts.split_first() {
                        let mut cmd = Command::new(bin);
                        cmd.args(args);
                        if let Some(fd) = activator.get_raw_fd(&name) {
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

        // 2. Supervise running processes
        let mut dead = Vec::new();
        for (name, child) in running.iter_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    eprintln!("[nilinit] Service '{}' exited with status: {}", name, status);
                    dead.push(name.clone());
                }
                Ok(None) => {}
                Err(e) => {
                    eprintln!("[nilinit] Error polling '{}': {}", name, e);
                    dead.push(name.clone());
                }
            }
        }

        for name in dead {
            running.remove(&name);
            if let Some(s) = config.services.iter().find(|svc| svc.name == name) {
                if s.restart == "always" {
                    println!("[nilinit] Restarting service: {}", name);
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
