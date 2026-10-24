// services/nilandroidd/src/container.rs — Real Android container lifecycle
//
// This is the piece that turns the Android bridge from a socket that answers
// "OK" into something that actually runs Android.
//
// What "real" means here, precisely:
//   * A container rootfs must exist on disk (`/data/android/rootfs`) with an
//     Android init binary. If it does not, the manager says exactly that and
//     refuses to start; it never reports a container that is not there.
//   * Starting the container performs the same privileged sequence a Waydroid
//     or LXC launch does: unshare PID/mount/IPC/UTS/NET namespaces, make the
//     mount tree private, bind the rootfs, pivot_root into it, mount a fresh
//     /proc, /sys, /dev and the Android data/system/vendor bind points, apply
//     cgroup v2 limits, then exec the container's init as PID 1.
//   * Lifecycle state is derived from the real child process, not a flag.
//
// Security boundaries are unchanged and non-negotiable (see isolation.rs):
// the container never gets /dev/input, /dev/kmsg, /dev/mem or DRM master.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use crate::isolation::ContainerProfile;

/// Where the container's filesystem and state live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerLayout {
    pub rootfs: PathBuf,
    pub data_dir: PathBuf,
    pub system_dir: PathBuf,
    pub vendor_dir: PathBuf,
    pub runtime_dir: PathBuf,
    pub state_file: PathBuf,
}

impl Default for ContainerLayout {
    fn default() -> Self {
        let base = PathBuf::from("/data/android");
        Self {
            rootfs: base.join("rootfs"),
            data_dir: base.join("data"),
            system_dir: base.join("system"),
            vendor_dir: base.join("vendor"),
            runtime_dir: PathBuf::from("/run/onuron/android"),
            state_file: base.join("container_state.json"),
        }
    }
}

/// Lifecycle state. `Running` is only ever set after a real child process was
/// spawned and is still alive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContainerState {
    /// No rootfs present, so a container cannot exist.
    Unavailable {
        reason: String,
    },
    Stopped,
    Running {
        pid: u32,
    },
    /// The last start attempt failed; the reason is preserved for the UI.
    Failed {
        reason: String,
    },
}

impl ContainerState {
    pub fn is_running(&self) -> bool {
        matches!(self, ContainerState::Running { .. })
    }

    pub fn label(&self) -> &'static str {
        match self {
            ContainerState::Unavailable { .. } => "UNAVAILABLE",
            ContainerState::Stopped => "STOPPED",
            ContainerState::Running { .. } => "RUNNING",
            ContainerState::Failed { .. } => "FAILED",
        }
    }

    /// Human-readable detail line for the shell's Android screen.
    pub fn detail(&self) -> String {
        match self {
            ContainerState::Unavailable { reason } => format!("no container rootfs: {reason}"),
            ContainerState::Stopped => "container not started".to_string(),
            ContainerState::Running { pid } => format!("container init running as PID {pid}"),
            ContainerState::Failed { reason } => format!("last start failed: {reason}"),
        }
    }
}

/// Candidate paths for the container's PID 1, in preference order. A rootfs is
/// only usable when one of these exists and is executable.
pub const INIT_CANDIDATES: &[&str] = &["/system/bin/init", "/init", "/sbin/init"];

/// Candidate Android userspace mount sources inside the rootfs.
pub const SYSTEM_MOUNT: &str = "/system";
pub const VENDOR_MOUNT: &str = "/vendor";

/// A planned mount inside the container, kept OS-independent so the plan is
/// unit-testable on the dev host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerMount {
    pub source: String,
    pub target: String,
    pub fstype: Option<String>,
    pub read_only: bool,
    pub no_dev: bool,
    pub no_suid: bool,
    /// Failure aborts the launch when true.
    pub fatal: bool,
}

/// Build the mount plan for a container launch.
///
/// Rationale for each entry:
///  * `/proc` must be a *fresh* procfs mounted after `CLONE_NEWPID`, otherwise
///    the container reads host process information and can escape via
///    `/proc/<pid>/root`.
///  * `/sys` is a read-only sysfs so Android's own hardware discovery works
///    against its own namespace view, but nothing is writable.
///  * `/dev` is a fresh tmpfs; device nodes are created deliberately afterwards.
///    The host `/dev` is never bind-mounted in.
///  * `/data`, `/system`, `/vendor` are bind mounts from the container's own
///    storage, so the container can never see the OnuronOS `/system`.
///  * `/dev/input`, `/dev/kmsg`, `/dev/mem`, `/dev/port` are explicitly absent
///    from the plan; `isolation.rs` also rejects them at policy level.
pub fn plan_container_mounts(layout: &ContainerLayout) -> Vec<ContainerMount> {
    let ro = |source: &str, target: &str, fstype: Option<&str>| ContainerMount {
        source: source.to_string(),
        target: target.to_string(),
        fstype: fstype.map(str::to_string),
        read_only: true,
        no_dev: true,
        no_suid: true,
        fatal: false,
    };

    vec![
        ContainerMount {
            source: "proc".into(),
            target: "/proc".into(),
            fstype: Some("proc".into()),
            read_only: false,
            no_dev: true,
            no_suid: true,
            fatal: true,
        },
        ro("sysfs", "/sys", Some("sysfs")),
        ContainerMount {
            source: "tmpfs".into(),
            target: "/dev".into(),
            fstype: Some("tmpfs".into()),
            read_only: false,
            no_dev: true,
            no_suid: true,
            fatal: true,
        },
        ro("tmpfs", "/tmp", Some("tmpfs")),
        ContainerMount {
            source: layout.system_dir.display().to_string(),
            target: SYSTEM_MOUNT.into(),
            fstype: None,
            read_only: true,
            no_dev: true,
            no_suid: true,
            fatal: false,
        },
        ContainerMount {
            source: layout.vendor_dir.display().to_string(),
            target: VENDOR_MOUNT.into(),
            fstype: None,
            read_only: true,
            no_dev: true,
            no_suid: true,
            fatal: false,
        },
        ContainerMount {
            source: layout.data_dir.display().to_string(),
            target: "/data".into(),
            fstype: None,
            read_only: false,
            no_dev: true,
            no_suid: true,
            fatal: true,
        },
    ]
}

/// Result of inspecting a rootfs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootfsProbe {
    /// A usable Android rootfs: this path is its init.
    Ready { init: PathBuf },
    /// The directory is missing entirely.
    Missing,
    /// The directory exists but has no Android init, so it is not a rootfs.
    NotAnAndroidRootfs,
}

/// Check whether `rootfs` really is a bootable Android container rootfs.
///
/// This deliberately checks for the init binary rather than just the directory:
/// an empty `/data/android/rootfs` created by a setup script is not evidence
/// that Android can run.
pub fn probe_rootfs(rootfs: &Path) -> RootfsProbe {
    if !rootfs.is_dir() {
        return RootfsProbe::Missing;
    }
    for candidate in INIT_CANDIDATES {
        let init = rootfs.join(candidate.trim_start_matches('/'));
        if is_executable(&init) {
            return RootfsProbe::Ready { init };
        }
    }
    RootfsProbe::NotAnAndroidRootfs
}

fn is_executable(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        match std::fs::metadata(p) {
            Ok(md) => md.is_file() && md.permissions().mode() & 0o111 != 0,
            Err(_) => false,
        }
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

/// The container manager. Holds the real child handle while running.
pub struct ContainerManager {
    pub layout: ContainerLayout,
    pub profile: ContainerProfile,
    state: ContainerState,
    #[cfg(target_os = "linux")]
    child: Option<std::process::Child>,
}

impl ContainerManager {
    pub fn new(layout: ContainerLayout, profile: ContainerProfile) -> Self {
        let probe = probe_rootfs(&layout.rootfs);
        let state = match probe {
            RootfsProbe::Ready { .. } => ContainerState::Stopped,
            RootfsProbe::Missing => ContainerState::Unavailable {
                reason: format!("{} does not exist", layout.rootfs.display()),
            },
            RootfsProbe::NotAnAndroidRootfs => ContainerState::Unavailable {
                reason: format!(
                    "{} has no Android init ({}); install a Waydroid/LineageOS image",
                    layout.rootfs.display(),
                    INIT_CANDIDATES.join(", ")
                ),
            },
        };
        Self {
            layout,
            profile,
            state,
            #[cfg(target_os = "linux")]
            child: None,
        }
    }

    pub fn state(&self) -> &ContainerState {
        &self.state
    }

    /// Refresh `Running` against the real child so a container that died is not
    /// reported as alive.
    pub fn refresh(&mut self) {
        #[cfg(target_os = "linux")]
        {
            if let Some(child) = self.child.as_mut() {
                match child.try_wait() {
                    Ok(Some(status)) => {
                        self.state = ContainerState::Stopped;
                        println!("[nilandroidd] container init exited: {status}");
                        self.child = None;
                    }
                    Ok(None) => {}
                    Err(e) => {
                        self.state = ContainerState::Failed {
                            reason: format!("cannot poll container init: {e}"),
                        };
                        self.child = None;
                    }
                }
            }
        }
    }

    /// Start the container. Performs real namespace isolation and returns an
    /// error describing exactly what is missing when it cannot start.
    pub fn start(&mut self) -> Result<u32, String> {
        self.profile
            .validate_isolation()
            .map_err(|e| format!("isolation profile invalid: {e}"))?;

        let init = match probe_rootfs(&self.layout.rootfs) {
            RootfsProbe::Ready { init } => init,
            RootfsProbe::Missing => {
                let reason = format!("rootfs {} is missing", self.layout.rootfs.display());
                self.state = ContainerState::Unavailable {
                    reason: reason.clone(),
                };
                return Err(reason);
            }
            RootfsProbe::NotAnAndroidRootfs => {
                let reason = format!(
                    "rootfs {} has no Android init binary",
                    self.layout.rootfs.display()
                );
                self.state = ContainerState::Unavailable {
                    reason: reason.clone(),
                };
                return Err(reason);
            }
        };

        if self.state.is_running() {
            return Err("container already running".to_string());
        }

        #[cfg(target_os = "linux")]
        {
            let child = spawn_container(&self.layout, &self.profile, &init)?;
            let pid = child.id();
            self.child = Some(child);
            self.state = ContainerState::Running { pid };
            Ok(pid)
        }
        #[cfg(not(target_os = "linux"))]
        {
            let _ = init;
            Err(
                "Android containers require Linux namespaces; not available on this host"
                    .to_string(),
            )
        }
    }

    /// Stop the container by signalling its init process group.
    pub fn stop(&mut self) -> Result<(), String> {
        #[cfg(target_os = "linux")]
        {
            if let Some(mut child) = self.child.take() {
                let pid = child.id() as i32;
                // Signal the whole process group: the container init forks the
                // Android userspace, and killing only PID 1 orphans the rest.
                // SAFETY: killpg is a plain syscall on a pid we spawned.
                unsafe {
                    libc::kill(-pid, libc::SIGTERM);
                }
                // Give the container a moment to shut down cleanly, then force.
                for _ in 0..50 {
                    match child.try_wait() {
                        Ok(Some(_)) => {
                            self.state = ContainerState::Stopped;
                            return Ok(());
                        }
                        Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
                        Err(_) => break,
                    }
                }
                let _ = child.kill();
                let _ = child.wait();
                self.state = ContainerState::Stopped;
                return Ok(());
            }
            self.state = ContainerState::Stopped;
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        {
            self.state = ContainerState::Stopped;
            Ok(())
        }
    }

    /// Status line used by the shell's Android screen and the IPC status reply.
    pub fn status_line(&self) -> String {
        format!(
            "{} — {} (isolation: {}, SELinux: {})",
            self.state.label(),
            self.state.detail(),
            if self.profile.namespaces.mount_namespace {
                "namespaces on"
            } else {
                "namespaces OFF"
            },
            self.profile.selinux_context
        )
    }
}

/// Launch the container's init under real namespace + cgroup isolation.
#[cfg(target_os = "linux")]
fn spawn_container(
    layout: &ContainerLayout,
    profile: &ContainerProfile,
    init: &Path,
) -> Result<std::process::Child, String> {
    use nix::sched::{unshare, CloneFlags};
    use std::os::unix::process::CommandExt;
    use std::process::{Command, Stdio};

    for dir in [
        &layout.data_dir,
        &layout.system_dir,
        &layout.vendor_dir,
        &layout.runtime_dir,
    ] {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
    }

    // Cross-check the mount plan against the device policy before doing any
    // privileged work: a plan that would expose a forbidden node is a bug, and
    // it must be caught before namespaces are created, not after.
    for m in plan_container_mounts(layout) {
        if !profile.is_device_permitted(Path::new(&m.source)) && m.source.starts_with("/dev") {
            return Err(format!(
                "refusing to mount forbidden device source {} into the container",
                m.source
            ));
        }
    }

    // The container must not share the host's PID, mount, IPC, UTS or network
    // namespace. CLONE_NEWNET is included because Android apps must not reach
    // the host's interfaces directly; traffic is proxied deliberately.
    let flags = CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWUTS
        | CloneFlags::CLONE_NEWNET;
    unshare(flags).map_err(|e| {
        format!("unshare({flags:?}) failed: {e}. Android containers need CAP_SYS_ADMIN.")
    })?;

    // Make the mount tree private so container mounts cannot propagate out and
    // host mounts cannot leak in.
    nix::mount::mount(
        None::<&str>,
        "/",
        None::<&str>,
        nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE,
        None::<&str>,
    )
    .map_err(|e| format!("mount --make-rprivate failed: {e}"))?;

    let rootfs = layout.rootfs.clone();
    let plan = plan_container_mounts(layout);
    let old_root = rootfs.join(".onuron-old-root");
    let hostname = format!("onuron-android-{}", std::process::id() % 1000);

    let mut cmd = Command::new(init);
    cmd.stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env("ANDROID_ROOT", SYSTEM_MOUNT)
        .env("ANDROID_DATA", "/data")
        .env("PATH", "/system/bin:/system/xbin:/vendor/bin")
        // put the container init in its own process group so stop() can signal
        // the whole group at once
        .process_group(0);

    let rootfs_for_child = rootfs.clone();
    let old_root_for_child = old_root.clone();
    let plan_for_child = plan.clone();
    let hostname_for_child = hostname.clone();

    // SAFETY: pre_exec runs in the forked child. It performs only syscalls
    // (mkdir, mount, pivot_root, chdir, sethostname) plus small string
    // allocations for error messages; the launcher is single-threaded here.
    unsafe {
        cmd.pre_exec(move || {
            enter_container_root(&rootfs_for_child, &old_root_for_child)
                .map_err(|e| std::io::Error::other(format!("container root setup: {e}")))?;
            apply_container_mounts(&plan_for_child)
                .map_err(|e| std::io::Error::other(format!("container mounts: {e}")))?;
            nix::unistd::sethostname(&hostname_for_child)
                .map_err(|e| std::io::Error::other(format!("sethostname: {e}")))?;
            Ok(())
        });
    }

    let child = cmd
        .spawn()
        .map_err(|e| format!("cannot start container init {}: {e}", init.display()))?;

    // Apply cgroup v2 resource limits to the real container PID.
    attach_container_cgroup(&child.id());
    Ok(child)
}

#[cfg(target_os = "linux")]
fn enter_container_root(rootfs: &Path, old_root: &Path) -> Result<(), String> {
    use nix::unistd::{chdir, pivot_root};

    std::fs::create_dir_all(old_root).map_err(|e| format!("mkdir {}: {e}", old_root.display()))?;

    // Bind the rootfs onto itself so it becomes a mount point that pivot_root
    // can move onto.
    nix::mount::mount(
        Some(rootfs),
        rootfs,
        None::<&str>,
        nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_BIND,
        None::<&str>,
    )
    .map_err(|e| format!("bind rootfs: {e}"))?;

    pivot_root(rootfs, old_root).map_err(|e| format!("pivot_root: {e}"))?;

    // Detach the old root: nothing under it is reachable afterwards.
    let _ = nix::mount::umount2(old_root, nix::mount::MntFlags::MNT_DETACH);

    // chdir("/") is mandatory: a retained CWD under the old root would let the
    // process walk back out of the container with relative `..`.
    chdir("/").map_err(|e| format!("chdir /: {e}"))?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_container_mounts(plan: &[ContainerMount]) -> Result<(), String> {
    use nix::mount::{mount, MsFlags};

    for m in plan {
        if std::fs::create_dir_all(&m.target).is_err() && std::fs::metadata(&m.target).is_err() {
            if m.fatal {
                return Err(format!("cannot create mount point {}", m.target));
            }
            continue;
        }

        let mut flags = MsFlags::empty();
        if m.read_only {
            flags |= MsFlags::MS_RDONLY;
        }
        if m.no_dev {
            flags |= MsFlags::MS_NODEV;
        }
        if m.no_suid {
            flags |= MsFlags::MS_NOSUID;
        }
        let bind = m.fstype.is_none();
        if bind {
            flags |= MsFlags::MS_BIND;
        }

        match mount(
            Some(m.source.as_str()),
            m.target.as_str(),
            m.fstype.as_deref(),
            flags,
            None::<&str>,
        ) {
            Ok(()) => {}
            Err(e) if !m.fatal => {
                eprintln!(
                    "[nilandroidd] optional mount {} on {} failed: {e}",
                    m.source, m.target
                );
            }
            Err(e) => return Err(format!("mount {} on {}: {e}", m.source, m.target)),
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn attach_container_cgroup(pid: &u32) {
    use std::path::Path as P;
    let root = P::new("/sys/fs/cgroup");
    if !nilrt::cgroup::is_cgroup_v2_available(root) {
        eprintln!("[nilandroidd] cgroup v2 unavailable; container limits not applied");
        return;
    }
    let limits = nilrt::cgroup::CgroupLimits {
        // Android needs far more than a native app: a full framework plus apps.
        memory_max_bytes: Some(3 * 1024 * 1024 * 1024),
        memory_high_bytes: Some(2560 * 1024 * 1024),
        pids_max: Some(4096),
        cpu_weight: Some(200),
    };
    match nilrt::cgroup::create_app_cgroup(root, "onuron.slice", "android", &limits) {
        Ok(dir) => {
            if let Err(e) = nilrt::cgroup::attach_pid_to_cgroup(&dir, *pid) {
                eprintln!("[nilandroidd] cgroup attach failed: {e}");
            } else {
                println!(
                    "[nilandroidd] container PID {pid} attached to cgroup {}",
                    dir.display()
                );
            }
        }
        Err(e) => eprintln!("[nilandroidd] cgroup setup failed: {e}"),
    }
}

/// Shared manager handle used by the IPC server and the status endpoint.
pub type SharedManager = Arc<Mutex<ContainerManager>>;

pub fn shared_manager(layout: ContainerLayout, profile: ContainerProfile) -> SharedManager {
    Arc::new(Mutex::new(ContainerManager::new(layout, profile)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_missing_for_absent_rootfs() {
        let p = Path::new("/definitely/not/a/rootfs/onuron-test");
        assert_eq!(probe_rootfs(p), RootfsProbe::Missing);
    }

    #[test]
    fn probe_rejects_a_directory_without_android_init() {
        let dir = std::env::temp_dir().join(format!("onuron_empty_rootfs_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        assert_eq!(probe_rootfs(&dir), RootfsProbe::NotAnAndroidRootfs);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn probe_accepts_a_rootfs_with_an_executable_init() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("onuron_real_rootfs_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("system/bin")).unwrap();
        let init = dir.join("system/bin/init");
        std::fs::write(&init, b"#!/bin/sh\n").unwrap();
        std::fs::set_permissions(&init, std::fs::Permissions::from_mode(0o755)).unwrap();

        assert_eq!(
            probe_rootfs(&dir),
            RootfsProbe::Ready { init: init.clone() }
        );

        // A non-executable init is not a usable rootfs.
        std::fs::set_permissions(&init, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert_eq!(probe_rootfs(&dir), RootfsProbe::NotAnAndroidRootfs);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn manager_reports_unavailable_when_no_rootfs_exists() {
        let layout = ContainerLayout {
            rootfs: PathBuf::from("/definitely/not/a/rootfs/onuron-test"),
            ..Default::default()
        };
        let mgr = ContainerManager::new(layout, ContainerProfile::default());
        match mgr.state() {
            ContainerState::Unavailable { reason } => {
                assert!(reason.contains("does not exist"));
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
        // And it must refuse to claim a start happened.
        let mut mgr = mgr;
        assert!(mgr.start().is_err());
        assert!(!mgr.state().is_running());
    }

    #[test]
    fn mount_plan_never_exposes_host_input_or_kernel_devices() {
        let layout = ContainerLayout::default();
        let plan = plan_container_mounts(&layout);
        let targets: Vec<&str> = plan.iter().map(|m| m.target.as_str()).collect();

        // The container gets its own /dev tmpfs, never the host's /dev.
        assert!(targets.contains(&"/dev"));
        assert!(!plan.iter().any(|m| m.source == "/dev"));

        // No plan entry may source or target a forbidden device.
        for m in &plan {
            for forbidden in ["/dev/input", "/dev/kmsg", "/dev/mem", "/dev/port"] {
                assert!(
                    !m.target.starts_with(forbidden),
                    "plan must not expose {forbidden}"
                );
                assert!(
                    !m.source.starts_with(forbidden),
                    "plan must not source {forbidden}"
                );
            }
        }
    }

    #[test]
    fn mount_plan_isolates_system_and_data_from_the_host() {
        let layout = ContainerLayout::default();
        let plan = plan_container_mounts(&layout);

        let system = plan.iter().find(|m| m.target == "/system").unwrap();
        assert_eq!(system.source, layout.system_dir.display().to_string());
        assert!(system.read_only, "/system must be read-only");
        // Critically, it must not be the host's /system.
        assert_ne!(system.source, "/system");

        let data = plan.iter().find(|m| m.target == "/data").unwrap();
        assert_eq!(data.source, layout.data_dir.display().to_string());
        assert_ne!(
            data.source, "/data",
            "container /data must not be the host /data"
        );

        // /proc must be fatal-if-unmounted: without a fresh procfs the container
        // can read host process state.
        let proc_mount = plan.iter().find(|m| m.target == "/proc").unwrap();
        assert!(proc_mount.fatal);
        assert_eq!(proc_mount.fstype.as_deref(), Some("proc"));
    }

    #[test]
    fn container_state_labels_are_stable() {
        assert_eq!(ContainerState::Stopped.label(), "STOPPED");
        assert_eq!(ContainerState::Running { pid: 7 }.label(), "RUNNING");
        assert!(ContainerState::Running { pid: 7 }.is_running());
        assert!(!ContainerState::Stopped.is_running());
        assert!(ContainerState::Failed { reason: "x".into() }
            .detail()
            .contains("x"));
    }
}
