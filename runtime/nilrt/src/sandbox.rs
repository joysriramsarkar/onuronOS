// runtime/nilrt/src/sandbox.rs — Real Linux namespace + pivot_root isolation
//
// On Linux: unshare(CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWIPC | CLONE_NEWUTS)
//           + PR_SET_NO_NEW_PRIVS + optional pivot_root/chroot into app rootfs
// On non-Linux: graceful passthrough (dev-host builds still compile and run).

pub struct SandboxConfig {
    pub app_id: String,
    pub uid: u32,
    pub gid: u32,
    /// Absolute path to the app's root filesystem overlay (or "" to skip pivot_root)
    pub rootfs: String,
    pub data_dir: String,
}

#[cfg(target_os = "linux")]
pub fn spawn_sandboxed(
    config: &SandboxConfig,
    cmd: &str,
    args: &[String],
) -> std::io::Result<()> {
    use nix::sched::{unshare, CloneFlags};
    use nix::sys::prctl;
    use nix::unistd::{chroot, pivot_root, Gid, Uid};
    use std::fs;
    use std::process::Command;

    // ── 1. Drop privilege-escalation paths BEFORE namespace entry ────────────
    // PR_SET_NO_NEW_PRIVS prevents execve() from gaining privileges via
    // setuid/setgid bits or file capabilities inside the sandbox.
    prctl::set_no_new_privs().map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("[nilrt:sandbox] PR_SET_NO_NEW_PRIVS failed: {e}"),
        )
    })?;

    // ── 2. Enter new namespaces ───────────────────────────────────────────────
    // CLONE_NEWPID  — process sees itself as PID 1 inside the sandbox
    // CLONE_NEWNS   — private mount namespace (mounts don't leak)
    // CLONE_NEWIPC  — isolated SysV IPC / POSIX message queues
    // CLONE_NEWUTS  — own hostname (apps can't read the real hostname)
    //
    // NOTE: CLONE_NEWUSER would also let us map UID 0 inside without real root,
    // but requires the kernel to allow unprivileged user namespaces
    // (`kernel.unprivileged_userns_clone = 1`).  We skip it here so the
    // sandbox works correctly both with and without that sysctl.
    let flags = CloneFlags::CLONE_NEWPID
        | CloneFlags::CLONE_NEWNS
        | CloneFlags::CLONE_NEWIPC
        | CloneFlags::CLONE_NEWUTS;

    unshare(flags).map_err(|e| {
        std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            format!("[nilrt:sandbox] unshare() failed: {e}"),
        )
    })?;

    println!(
        "[nilrt:sandbox] {} ({uid}/{gid}) entered PID+mount+IPC+UTS namespaces",
        config.app_id,
        uid = config.uid,
        gid = config.gid
    );

    // ── 3. Filesystem isolation ───────────────────────────────────────────────
    // If a rootfs overlay exists we pivot_root into it, giving the process a
    // completely isolated view of the filesystem.  Without a rootfs we fall
    // back to a bare chroot into /data/app/<id>/root (if it exists) so at
    // minimum the app cannot reach /proc, /sys, or /etc of the host.
    let use_pivot = !config.rootfs.is_empty() && fs::metadata(&config.rootfs).is_ok();
    let chroot_path = format!("/data/app/{}/root", config.app_id);
    let use_chroot = !use_pivot && fs::metadata(&chroot_path).is_ok();

    if use_pivot {
        // pivot_root(new_root, put_old):
        //   1. Bind-mount new_root onto itself (required by pivot_root)
        //   2. Create a put_old directory inside new_root
        //   3. Call pivot_root(new_root, put_old)
        //   4. Unmount put_old (the old root)
        let put_old = format!("{}/old_root", config.rootfs);
        let _ = fs::create_dir_all(&put_old);

        // Bind-mount the new root on itself (pivot_root requirement)
        nix::mount::mount(
            Some(config.rootfs.as_str()),
            config.rootfs.as_str(),
            None::<&str>,
            nix::mount::MsFlags::MS_BIND | nix::mount::MsFlags::MS_REC,
            None::<&str>,
        )
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e.to_string()))?;

        pivot_root(config.rootfs.as_str(), put_old.as_str()).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, format!("pivot_root: {e}"))
        })?;

        // Unmount the old root so it's no longer accessible
        nix::mount::umount2("/old_root", nix::mount::MntFlags::MNT_DETACH).ok();
        println!("[nilrt:sandbox] pivot_root → {}", config.rootfs);
    } else if use_chroot {
        chroot(chroot_path.as_str()).map_err(|e| {
            std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("chroot({chroot_path}): {e}"),
            )
        })?;
        println!("[nilrt:sandbox] chroot → {chroot_path}");
    } else {
        println!(
            "[nilrt:sandbox] No rootfs overlay found for {}; running in namespace-only mode",
            config.app_id
        );
    }

    // ── 4. Drop to the app's UID/GID ─────────────────────────────────────────
    let gid = Gid::from_raw(config.gid);
    let uid = Uid::from_raw(config.uid);
    nix::unistd::setresgid(gid, gid, gid)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string()))?;
    nix::unistd::setresuid(uid, uid, uid)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::PermissionDenied, e.to_string()))?;

    // ── 5. Spawn and wait ─────────────────────────────────────────────────────
    println!(
        "[nilrt:sandbox] exec: {cmd} {:?}  (uid={}, gid={})",
        args, config.uid, config.gid
    );
    let mut child = Command::new(cmd)
        .args(args)
        .env_clear()
        .env("NIL_APP_ID", &config.app_id)
        .env("NIL_DATA_DIR", &config.data_dir)
        .env("HOME", &config.data_dir)
        .env("PATH", "/usr/local/bin:/usr/bin:/bin")
        .spawn()?;

    let status = child.wait()?;
    println!(
        "[nilrt:sandbox] {} exited with {}",
        config.app_id, status
    );
    Ok(())
}

/// Non-Linux fallback — compiles cleanly on Windows/macOS dev hosts.
#[cfg(not(target_os = "linux"))]
pub fn spawn_sandboxed(
    config: &SandboxConfig,
    cmd: &str,
    args: &[String],
) -> std::io::Result<()> {
    println!(
        "[nilrt:sandbox] (non-Linux passthrough) spawning {} for app {}",
        cmd, config.app_id
    );
    let mut child = std::process::Command::new(cmd)
        .args(args)
        .env("NIL_APP_ID", &config.app_id)
        .env("NIL_DATA_DIR", &config.data_dir)
        .spawn()?;
    child.wait()?;
    Ok(())
}
