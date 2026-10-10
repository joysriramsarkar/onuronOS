// runtime/nilrt/src/sandbox.rs — Real Linux namespace + pivot_root isolation
//
// On Linux: unshare(CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWIPC | CLONE_NEWUTS)
//           + PR_SET_NO_NEW_PRIVS + pivot_root/chroot into app rootfs +
//           a fresh private /proc, an app-private tmpfs /tmp, and a hidden
//           /sys; optional permission restrictions; then exec.
// On non-Linux: graceful passthrough (dev-host builds still compile and run).
//
// Threat model / accepted risks (see also the C2 review in docs):
//  * Same-UID ptrace: without a user namespace or a LSM policy, an app running
//    as the same uid as another process can `ptrace` it. We rely on per-app
//    UIDs (set by the launcher) and optionally SELinux to make this
//    impossible; the namespace layer alone does not prevent it.
//  * Device nodes: `/dev` is shared with the host. Permission-bearing device
//    nodes are masked by binding `/dev/null` over them (see permissions.rs);
//    a real devices cgroup is the follow-up.
//  * This is defence in depth, not a claim of escape-proofing. Kernel bugs in
//    namespaces/mounts remain exploitable.

use std::process::Command;

/// Mount behaviour flags for a planned mount.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MountFlags {
    pub read_only: bool,
    pub no_suid: bool,
    pub no_dev: bool,
    pub no_exec: bool,
    /// Perform a bind mount (`source` is an existing path) instead of mounting
    /// a filesystem of type `fstype`.
    pub bind: bool,
}

/// A planned mount. Kept OS-independent so `plan_sandbox_mounts` is unit
/// testable on the Windows dev host.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SandboxMount {
    pub target: String,
    pub source: String,
    pub fstype: Option<String>,
    pub flags: MountFlags,
    /// `true`: failure to mount aborts the launch (security-critical).
    /// `false`: best-effort (e.g. `/tmp`), logged and skipped on failure.
    pub fatal: bool,
}

fn secure_fs(read_only: bool, no_exec: bool) -> MountFlags {
    MountFlags {
        read_only,
        no_suid: true,
        no_dev: true,
        no_exec,
        bind: false,
    }
}

#[derive(Clone)]
pub struct SandboxConfig {
    pub app_id: String,
    pub uid: u32,
    pub gid: u32,
    /// Absolute path to the app's root filesystem overlay (or "" to skip pivot_root)
    pub rootfs: String,
    pub data_dir: String,
    /// Permissions actually granted to the app. Only consulted when
    /// `strict_permissions` is set.
    pub permissions: Vec<String>,
    /// When true, apply `permissions::plan_restrictions` inside the sandbox
    /// (read-only root unless `storage.write`, masked device nodes). Defaults
    /// to false to preserve the permissive dev behaviour.
    pub strict_permissions: bool,
    /// When true (default), `/sys` is replaced with an empty read-only tmpfs so
    /// the app cannot read host hardware topology. When false, a read-only
    /// sysfs is mounted instead.
    pub hide_sysfs: bool,
    /// SELinux security context string to apply before exec (e.g. nilos_app_t)
    pub selinux_context: Option<String>,
}

impl SandboxConfig {
    pub fn new(
        app_id: impl Into<String>,
        uid: u32,
        gid: u32,
        rootfs: impl Into<String>,
        data_dir: impl Into<String>,
    ) -> Self {
        Self {
            app_id: app_id.into(),
            uid,
            gid,
            rootfs: rootfs.into(),
            data_dir: data_dir.into(),
            permissions: Vec::new(),
            strict_permissions: false,
            hide_sysfs: true,
            selinux_context: None,
        }
    }
}

/// Pure decision logic: the mounts to set up *inside* the sandbox once its
/// filesystem root has been established.
///
/// Security rationale:
///  * `/proc` — a *fresh* procfs instance mounted after `CLONE_NEWPID`, so the
///    app sees only its own PID namespace and cannot read host `/proc/<pid>`
///    (cmdline, environ, fds, maps) or use `/proc/<pid>/root` to escape.
///    Mounted `nosuid,nodev,noexec`; failure is fatal.
///  * `/tmp` — a fresh private tmpfs so the app cannot write through a shared
///    host `/tmp` (sticky-bit symlink attacks, leaking state to other apps).
///    Best effort: a rootfs without a writable /tmp must not break launch.
///  * `/sys` — the host sysfs is never exposed. `hide_sysfs` (default) mounts
///    an empty read-only tmpfs; otherwise a read-only sysfs is mounted. Both
///    are `nosuid,nodev,noexec` and never writable.
pub fn plan_sandbox_mounts(config: &SandboxConfig) -> Vec<SandboxMount> {
    let (sys_source, sys_fstype) = if config.hide_sysfs {
        ("tmpfs", "tmpfs")
    } else {
        ("sysfs", "sysfs")
    };

    vec![
        SandboxMount {
            target: "/proc".to_string(),
            source: "proc".to_string(),
            fstype: Some("proc".to_string()),
            flags: secure_fs(false, true),
            fatal: true,
        },
        SandboxMount {
            target: "/tmp".to_string(),
            source: "tmpfs".to_string(),
            fstype: Some("tmpfs".to_string()),
            flags: secure_fs(false, false),
            fatal: false,
        },
        SandboxMount {
            target: "/sys".to_string(),
            source: sys_source.to_string(),
            fstype: Some(sys_fstype.to_string()),
            flags: secure_fs(true, true),
            fatal: false,
        },
    ]
}

#[cfg(target_os = "linux")]
pub fn spawn_sandboxed(
    config: &SandboxConfig,
    cmd: &str,
    args: &[String],
) -> std::io::Result<()> {
    use nix::sched::{unshare, CloneFlags};
    use nix::sys::prctl;
    use std::os::unix::process::CommandExt;

    // ── 1. Drop privilege-escalation paths BEFORE namespace entry ────────────
    // PR_SET_NO_NEW_PRIVS prevents execve() from gaining privileges via
    // setuid/setgid bits or file capabilities inside the sandbox.
    if let Err(e) = prctl::set_no_new_privs() {
        eprintln!("[nilrt:sandbox] PR_SET_NO_NEW_PRIVS warning: {e}");
    }

    // ── 2. Enter new namespaces ───────────────────────────────────────────────
    // CLONE_NEWPID  — processes inside see themselves as PID 1
    // CLONE_NEWNS   — private mount namespace (mounts don't leak)
    // CLONE_NEWIPC  — isolated SysV IPC / POSIX message queues
    // CLONE_NEWUTS  — own hostname (apps can't read the real hostname)
    //
    // NOTE: CLONE_NEWUSER would also let us map UID 0 inside without real root,
    // but requires the kernel to allow unprivileged user namespaces
    // (`kernel.unprivileged_userns_clone = 1`). We skip it so the sandbox works
    // correctly both with and without that sysctl.
    //
    // In unprivileged test / CI container environments, namespace operations
    // return EPERM / EACCES due to lack of CAP_SYS_ADMIN. Fall back to direct
    // execution so development integration tests can run.
    let mut flags = CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWUTS;

    // Kernel-level Network Isolation:
    // If the app is NOT granted network permissions, isolate it in a private
    // network namespace (CLONE_NEWNET) where outside access is physically blocked.
    let has_network = config.permissions.iter().any(|p| p == "network" || p == "net.internet");
    if !has_network {
        flags |= CloneFlags::CLONE_NEWNET;
    }

    if let Err(e) = unshare(flags) {
        let is_unprivileged_test_env = std::env::var("NILRT_ALLOW_INSECURE_DEV").as_deref() == Ok("1")
            || std::env::var("CI").as_deref() == Ok("true");
        if (e == nix::errno::Errno::EPERM || e == nix::errno::Errno::EACCES) && is_unprivileged_test_env {
            eprintln!(
                "[nilrt:sandbox] WARNING: unshare() not permitted ({e}); unprivileged test container detected. Falling back to direct launch."
            );
            return spawn_unprivileged(config, cmd, args);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("[nilrt:sandbox] unshare({flags:?}) failed: {e}. Refusing unconfined execution in production mode."),
        ));
    }

    // Make the new mount namespace private so host mounts cannot propagate in
    // and the sandbox's own mounts cannot leak back out.
    if let Err(e) = nix::mount::mount(
        None::<&str>,
        "/",
        None::<&str>,
        nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_PRIVATE,
        None::<&str>,
    ) {
        let is_unprivileged_test_env = std::env::var("NILRT_ALLOW_INSECURE_DEV").as_deref() == Ok("1")
            || std::env::var("CI").as_deref() == Ok("true");
        if (e == nix::errno::Errno::EPERM || e == nix::errno::Errno::EACCES) && is_unprivileged_test_env {
            eprintln!(
                "[nilrt:sandbox] WARNING: mount --make-rprivate not permitted ({e}); unprivileged test container detected. Falling back to direct launch."
            );
            return spawn_unprivileged(config, cmd, args);
        }
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("[nilrt:sandbox] mount --make-rprivate failed: {e}. Refusing unconfined execution in production mode."),
        ));
    }

    println!(
        "[nilrt:sandbox] {} ({uid}/{gid}) entered PID+mount+IPC+UTS namespaces",
        config.app_id,
        uid = config.uid,
        gid = config.gid
    );

    // ── 3. Build the command; isolate the filesystem in the child ────────────
    // All privileged setup runs in the forked child (pre_exec), never in the
    // launcher itself. Doing the mounts in the child means /proc is a fresh
    // instance for the child's PID namespace, and the parent stays privileged
    // only long enough to spawn.
    let mut command = Command::new(cmd);
    command
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .env_clear()
        .env("NIL_APP_ID", &config.app_id)
        .env("NIL_DATA_DIR", &config.data_dir)
        .env("PATH", "/usr/bin:/bin");

    if let Ok(val) = std::env::var("NILRT_LIFECYCLE_VALUE") {
        command.env("NILRT_LIFECYCLE_VALUE", val);
    }

    let child_config = config.clone();
    // SAFETY: `pre_exec` runs in the forked child between fork and exec. The
    // closure only performs setup syscalls (mount, chroot, setuid, prctl) and
    // allocates strings for error messages. The launcher is single-threaded
    // during launch.
    unsafe {
        command.pre_exec(move || run_in_sandbox(&child_config));
    }

    let mut child = command.spawn().map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("[nilrt:sandbox] failed to start {}: {e}", config.app_id),
        )
    })?;
    let cgroup_dir = attach_child_cgroup(&config.app_id, child.id());
    let status = child.wait()?;
    if let Some(cg) = cgroup_dir {
        let _ = crate::cgroup::cleanup_app_cgroup(&cg);
    }
    println!("[nilrt:sandbox] {} exited with {status}", config.app_id);
    Ok(())
}

#[cfg(target_os = "linux")]
fn attach_child_cgroup(app_id: &str, pid: u32) -> Option<std::path::PathBuf> {
    let cgroup_root = std::path::Path::new("/sys/fs/cgroup");
    if !cgroup_root.exists() {
        return None;
    }
    let limits = crate::cgroup::CgroupLimits::default();
    match crate::cgroup::create_app_cgroup(cgroup_root, "onuron.slice", app_id, &limits) {
        Ok(dir) => {
            if let Err(e) = crate::cgroup::attach_pid_to_cgroup(&dir, pid) {
                eprintln!("[nilrt:sandbox] cgroup attach notice: {e}");
            } else {
                println!(
                    "[nilrt:sandbox] Attached PID {pid} to cgroup {}",
                    dir.display()
                );
            }
            Some(dir)
        }
        Err(e) => {
            eprintln!("[nilrt:sandbox] cgroup setup notice: {e}");
            None
        }
    }
}

#[cfg(target_os = "linux")]
fn spawn_unprivileged(
    config: &SandboxConfig,
    cmd: &str,
    args: &[String],
) -> std::io::Result<()> {
    println!(
        "[nilrt:sandbox] unprivileged environment: direct launch of {} (app {}, without namespace isolation)",
        cmd, config.app_id
    );
    let mut command = Command::new(cmd);
    command
        .args(args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .env("NIL_APP_ID", &config.app_id)
        .env("NIL_DATA_DIR", &config.data_dir);
    if let Ok(val) = std::env::var("NILRT_LIFECYCLE_VALUE") {
        command.env("NILRT_LIFECYCLE_VALUE", val);
    }
    let mut child = command.spawn().map_err(|e| {
        std::io::Error::new(
            e.kind(),
            format!("[nilrt:sandbox] failed to start {}: {e}", config.app_id),
        )
    })?;
    let cgroup_dir = attach_child_cgroup(&config.app_id, child.id());
    let status = child.wait()?;
    if let Some(cg) = cgroup_dir {
        let _ = crate::cgroup::cleanup_app_cgroup(&cg);
    }
    println!("[nilrt:sandbox] {} exited with {status}", config.app_id);
    Ok(())
}

/// Everything that must happen in the child after `fork` and before `exec`:
/// filesystem isolation, namespace-local mounts, permission restrictions,
/// privilege drop, and the syscall filter.
#[cfg(target_os = "linux")]
fn run_in_sandbox(config: &SandboxConfig) -> std::io::Result<()> {
    setup_rootfs(config)?;
    let mounts = plan_sandbox_mounts(config);
    apply_mount_plan(&mounts)?;

    if config.strict_permissions {
        let policy = crate::permissions::AppPolicy::new(
            std::path::PathBuf::from(&config.data_dir),
            config.permissions.clone(),
        );
        apply_permission_restrictions(&policy)?;
        // Apply seccomp last: it blocks the privileged syscalls (mount,
        // setuid, chroot, …) this function still needs.
        crate::seccomp::apply_app_seccomp().map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::PermissionDenied, e)
        })?;
    }

    drop_privileges(config.gid, config.uid)?;

    if let Some(ctx) = &config.selinux_context {
        if let Err(e) = crate::selinux::setexeccon(ctx) {
            if config.strict_permissions {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    format!("[nilrt:sandbox] could not set mandatory SELinux context {ctx}: {e}"),
                ));
            }
            eprintln!("[nilrt:sandbox] Warning: could not set SELinux context {ctx}: {e}");
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn setup_rootfs(config: &SandboxConfig) -> std::io::Result<()> {
    use nix::unistd::{chroot, pivot_root};
    use std::fs;

    let use_pivot = !config.rootfs.is_empty() && fs::metadata(&config.rootfs).is_ok();
    let chroot_path = format!("/data/app/{}/root", config.app_id);
    let use_chroot = !use_pivot && fs::metadata(&chroot_path).is_ok();

    if use_pivot {
        // pivot_root moves the old root under the new one; the old root is then
        // detached so the host filesystem's /proc, /sys and /dev become
        // unreachable.
        let old_root = format!("{}/.nilrt-old-root", config.rootfs);
        let _ = fs::create_dir_all(&old_root);
        nix::mount::mount(
            Some(config.rootfs.as_str()),
            config.rootfs.as_str(),
            None::<&str>,
            nix::mount::MsFlags::MS_REC | nix::mount::MsFlags::MS_BIND,
            None::<&str>,
        )?;
        pivot_root(config.rootfs.as_str(), old_root.as_str()).map_err(nix_err)?;
        // MNT_DETACH: lazily detach the old root; nothing under it is reachable.
        let _ = nix::mount::umount2(old_root.as_str(), nix::mount::MntFlags::MNT_DETACH);
        std::env::set_current_dir("/")?;
    } else if use_chroot {
        chroot(chroot_path.as_str()).map_err(nix_err)?;
        // Without chdir("/") a process can escape a chroot by walking `..`
        // from the retained CWD (the classic chroot escape). Always reset it.
        std::env::set_current_dir("/")?;
    } else {
        // No rootfs: the app still gets fresh /proc, /tmp and /sys from
        // `plan_sandbox_mounts`, but the host root remains visible. This mode
        // is only appropriate for trusted development payloads.
        println!(
            "[nilrt:sandbox] no rootfs for {}; mount-namespace isolation only",
            config.app_id
        );
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn apply_mount_plan(mounts: &[SandboxMount]) -> std::io::Result<()> {
    use nix::mount::{mount, MsFlags};

    for m in mounts {
        if let Some(parent) = std::path::Path::new(&m.target).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if std::fs::create_dir_all(&m.target).is_err() && std::fs::metadata(&m.target).is_err() {
            if m.fatal {
                return Err(std::io::Error::other(format!(
                    "[nilrt:sandbox] cannot create mount point {}",
                    m.target
                )));
            }
            eprintln!(
                "[nilrt:sandbox] skipping optional mount {}: no mount point",
                m.target
            );
            continue;
        }

        let mut flags = MsFlags::empty();
        if m.flags.read_only {
            flags |= MsFlags::MS_RDONLY;
        }
        if m.flags.no_suid {
            flags |= MsFlags::MS_NOSUID;
        }
        if m.flags.no_dev {
            flags |= MsFlags::MS_NODEV;
        }
        if m.flags.no_exec {
            flags |= MsFlags::MS_NOEXEC;
        }
        if m.flags.bind {
            flags |= MsFlags::MS_BIND;
        }

        let fstype = if m.flags.bind { None } else { m.fstype.as_deref() };
        match mount(
            Some(m.source.as_str()),
            m.target.as_str(),
            fstype,
            flags,
            None::<&str>,
        ) {
            Ok(()) => {}
            Err(e) if !m.fatal => {
                eprintln!(
                    "[nilrt:sandbox] optional mount {} on {} failed (continuing): {e}",
                    m.source, m.target
                );
            }
            Err(e) => {
                return Err(std::io::Error::other(format!(
                    "[nilrt:sandbox] mount {} on {} ({}): {e}",
                    m.source,
                    m.target,
                    fstype.unwrap_or("bind")
                )));
            }
        }
    }
    Ok(())
}

/// Apply the concrete restrictions derived from an app's permissions.
///
/// Mechanism (documented in `permissions::plan_restrictions`):
///  1. Denied device nodes are masked by bind-mounting `/dev/null` over them,
///     so opening them yields /dev/null semantics, not the real device.
///  2. Without `storage.write`, writable bind mounts for the app's data dir and
///     `/tmp` are created first, then the sandbox root is remounted read-only.
#[cfg(target_os = "linux")]
fn apply_permission_restrictions(policy: &crate::permissions::AppPolicy) -> std::io::Result<()> {
    use nix::mount::{mount, MsFlags};

    let plan = crate::permissions::plan_restrictions(policy);

    for pattern in &plan.device_masks {
        let Some(dir) = std::path::Path::new(pattern).parent() else {
            continue;
        };
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if crate::permissions::matches_device_pattern(pattern, &entry.path()) {
                // Best-effort: if a mask fails the app still may be denied the
                // node by filesystem permissions, but log the gap.
                if let Err(e) = mount(
                    Some("/dev/null"),
                    entry.path().as_os_str(),
                    None::<&str>,
                    MsFlags::MS_BIND,
                    None::<&str>,
                ) {
                    eprintln!(
                        "[nilrt:sandbox] could not mask device {}: {e}",
                        entry.path().display()
                    );
                }
            }
        }
    }

    if plan.read_only_root {
        // Create the writable bind mounts first: a bind mount keeps its own
        // writable flag when the parent is later remounted read-only.
        for path in &plan.writable_paths {
            let _ = std::fs::create_dir_all(path);
            if let Err(e) = mount(
                Some(path.as_str()),
                path.as_str(),
                None::<&str>,
                MsFlags::MS_BIND,
                None::<&str>,
            ) {
                return Err(std::io::Error::other(format!(
                    "[nilrt:sandbox] could not bind-mount writable path {path}: {e}"
                )));
            }
        }
        mount(
            None::<&str>,
            "/",
            None::<&str>,
            MsFlags::MS_REMOUNT | MsFlags::MS_BIND | MsFlags::MS_RDONLY,
            None::<&str>,
        )
        .map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!("[nilrt:sandbox] could not remount root read-only: {e}"),
            )
        })?;
    }
    Ok(())
}

#[cfg(target_os = "linux")]
fn drop_privileges(gid: u32, uid: u32) -> std::io::Result<()> {
    use nix::unistd::{setgroups, setresgid, setresuid, Gid, Uid};

    let gid = Gid::from_raw(gid);
    let uid = Uid::from_raw(uid);
    // Drain the supplementary groups first: setresgid does not remove them, so
    // a process could otherwise retain the host's group memberships.
    setgroups(&[gid]).map_err(nix_err)?;
    setresgid(gid, gid, gid).map_err(nix_err)?;
    setresuid(uid, uid, uid).map_err(nix_err)?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn nix_err(e: nix::errno::Errno) -> std::io::Error {
    std::io::Error::from_raw_os_error(e as i32)
}

// ── Non-Linux host ────────────────────────────────────────────────────────────

/// Graceful passthrough for the Windows/macOS dev host. No isolation is
/// applied; this exists so the launcher compiles and runs for development.
#[cfg(not(target_os = "linux"))]
pub fn spawn_sandboxed(
    config: &SandboxConfig,
    cmd: &str,
    args: &[String],
) -> std::io::Result<()> {
    println!(
        "[nilrt:sandbox] non-Linux host: passthrough launch of {} (app {}, no isolation)",
        cmd, config.app_id
    );
    let mut child = Command::new(cmd);
    child
        .args(args)
        .env("NIL_APP_ID", &config.app_id)
        .env("NIL_DATA_DIR", &config.data_dir);
    if let Ok(val) = std::env::var("NILRT_LIFECYCLE_VALUE") {
        child.env("NILRT_LIFECYCLE_VALUE", val);
    }
    let mut child = child.spawn()?;
    let status = child.wait()?;
    println!("[nilrt:sandbox] {} exited with {status}", config.app_id);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> SandboxConfig {
        SandboxConfig::new(
            "org.onuron.test",
            1000,
            1000,
            "",
            "/data/data/org.onuron.test",
        )
    }

    fn find<'a>(mounts: &'a [SandboxMount], target: &str) -> &'a SandboxMount {
        mounts
            .iter()
            .find(|m| m.target == target)
            .unwrap_or_else(|| panic!("no mount planned for {target}"))
    }

    #[test]
    fn always_mounts_a_fresh_procfs() {
        let mounts = plan_sandbox_mounts(&config());
        let proc_mount = find(&mounts, "/proc");
        assert_eq!(proc_mount.fstype.as_deref(), Some("proc"));
        assert!(
            proc_mount.fatal,
            "/proc must be fatal: without it the app sees host processes"
        );
        assert!(proc_mount.flags.no_suid && proc_mount.flags.no_dev && proc_mount.flags.no_exec);
    }

    #[test]
    fn tmp_is_a_private_tmpfs() {
        let mounts = plan_sandbox_mounts(&config());
        let tmp = find(&mounts, "/tmp");
        assert_eq!(tmp.fstype.as_deref(), Some("tmpfs"));
        assert!(tmp.flags.no_suid && tmp.flags.no_dev);
        // /tmp must stay writable, so it is never read-only.
        assert!(!tmp.flags.read_only);
    }

    #[test]
    fn sysfs_is_hidden_by_default_and_never_writable() {
        let mounts = plan_sandbox_mounts(&config());
        let sys_mount = find(&mounts, "/sys");
        assert_eq!(sys_mount.fstype.as_deref(), Some("tmpfs"));
        assert!(sys_mount.flags.read_only, "hidden /sys must be read-only");
        assert!(sys_mount.flags.no_dev && sys_mount.flags.no_exec);
    }

    #[test]
    fn sysfs_can_be_exposed_read_only() {
        let mut c = config();
        c.hide_sysfs = false;
        let mounts = plan_sandbox_mounts(&c);
        let sys_mount = find(&mounts, "/sys");
        assert_eq!(sys_mount.fstype.as_deref(), Some("sysfs"));
        assert!(
            sys_mount.flags.read_only,
            "host sysfs must never be writable"
        );
    }
}