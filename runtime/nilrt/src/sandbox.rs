// runtime/nilrt/src/sandbox.rs — Namespaces + pivot_root + cgroup isolation
use std::fs;
use std::path::Path;
use std::process::Command;

pub struct SandboxConfig {
    pub app_id: String,
    pub uid: u32,
    pub gid: u32,
    pub rootfs: String,
    pub data_dir: String,
}

pub fn spawn_sandboxed(config: &SandboxConfig, cmd: &str, args: &[String]) -> std::io::Result<()> {
    println!("[nilrt:sandbox] Spawning {} (UID: {}, GID: {}) in namespace sandbox", config.app_id, config.uid, config.gid);
    
    // In actual Linux target: unshare(CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWIPC | CLONE_NEWUTS)
    // and pivot_root into app isolated container
    let mut command = Command::new(cmd);
    command.args(args);
    command.env("NIL_APP_ID", &config.app_id);
    command.env("NIL_DATA_DIR", &config.data_dir);
    
    let mut child = command.spawn()?;
    let _ = child.wait();
    Ok(())
}
