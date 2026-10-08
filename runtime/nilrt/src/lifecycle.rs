// runtime/nilrt/src/lifecycle.rs — App lifecycle helpers (E1).
//
// A thin, testable layer over the package manager and the sandbox:
//   install (nilpkg) → launch (sandbox) → app writes its data dir →
//   relaunch (data persists) → uninstall (app dir + data removed).
//
// The launch path is split from process execution so it can be integration
// tested on the Windows dev host, where the native payloads cannot run.

use crate::sandbox::{spawn_sandboxed, SandboxConfig};

/// Root of installed application directories (normally `/data/app`).
pub fn app_dir(app_root: &std::path::Path, app_id: &str) -> std::path::PathBuf {
    app_root.join(app_id)
}

/// App-private data directory (kept *outside* the app dir so that a package
/// upgrade, which replaces the app dir, cannot destroy user data).
pub fn app_data_dir(data_root: &std::path::Path, app_id: &str) -> std::path::PathBuf {
    data_root.join(app_id)
}

/// Marker written while a launch is in progress. A crashed launch leaves one
/// behind; `cleanup_stale_launches` removes them so they never accumulate.
pub fn launch_stage_path(app_root: &std::path::Path, app_id: &str) -> std::path::PathBuf {
    app_root.join(format!(".nilrt-launch-{app_id}"))
}

/// RAII guard for a launch. Dropping it always removes the staging marker, so
/// an aborted or failed launch leaves no residue.
pub struct LaunchGuard {
    path: std::path::PathBuf,
    armed: bool,
}

impl LaunchGuard {
    pub fn begin(app_root: &std::path::Path, app_id: &str) -> std::io::Result<Self> {
        std::fs::create_dir_all(app_root)?;
        let path = launch_stage_path(app_root, app_id);
        std::fs::write(&path, b"launching\n")?;
        Ok(Self { path, armed: true })
    }

    /// Explicitly disarm before drop (equivalent to simply letting it drop).
    pub fn commit(mut self) {
        self.armed = false;
        let _ = std::fs::remove_file(&self.path);
    }
}

impl Drop for LaunchGuard {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_file(&self.path);
        }
    }
}

/// Remove stale launch markers left by a crashed launcher. Runs on every
/// launch; returns how many were removed.
pub fn cleanup_stale_launches(app_root: &std::path::Path) -> usize {
    let Ok(entries) = std::fs::read_dir(app_root) else {
        return 0;
    };
    let mut removed = 0;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with(".nilrt-launch-") && std::fs::remove_file(entry.path()).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Parameters for a sandboxed launch, decoupled from any particular binary so
/// the same path is exercised by tests and by `nilrt-launch`.
#[derive(Debug, Clone)]
pub struct LaunchSpec {
    pub app_id: String,
    pub rootfs: String,
    pub data_dir: String,
    pub uid: u32,
    pub gid: u32,
    /// Permissions the app was actually granted.
    pub permissions: Vec<String>,
    /// Enable sandbox permission enforcement (read-only root, masked devices).
    pub strict_permissions: bool,
}

/// Launch a command through the sandbox with the given spec.
pub fn launch(spec: &LaunchSpec, cmd: &str, args: &[String]) -> std::io::Result<()> {
    let config = SandboxConfig {
        app_id: spec.app_id.clone(),
        uid: spec.uid,
        gid: spec.gid,
        rootfs: spec.rootfs.clone(),
        data_dir: spec.data_dir.clone(),
        permissions: spec.permissions.clone(),
        strict_permissions: spec.strict_permissions,
        hide_sysfs: true,
    };
    spawn_sandboxed(&config, cmd, args)
}

/// Resolve the installed executable for an app, refusing path-like IDs.
pub fn resolve_executable(
    app_root: &std::path::Path,
    app_id: &str,
) -> Result<std::path::PathBuf, String> {
    if !is_valid_app_id(app_id) {
        return Err(format!("invalid app id: {app_id}"));
    }
    let bin = app_root.join(app_id).join("bin").join(app_id);
    if bin.is_file() {
        Ok(bin)
    } else {
        #[cfg(target_os = "windows")]
        {
            let bin_exe = app_root.join(app_id).join("bin").join(format!("{app_id}.exe"));
            if bin_exe.is_file() {
                return Ok(bin_exe);
            }
        }
        Err(format!("no installed executable at {}", bin.display()))
    }
}

/// Full launch path: create the data dir, arm the staging guard, resolve the
/// installed binary, and run it through the sandbox. Any failure unwinds the
/// guard so no staging residue remains.
pub fn launch_installed(
    app_root: &std::path::Path,
    spec: &LaunchSpec,
    args: &[String],
) -> Result<(), String> {
    cleanup_stale_launches(app_root);
    let exe = resolve_executable(app_root, &spec.app_id)?;
    let data_dir = std::path::Path::new(&spec.data_dir);
    std::fs::create_dir_all(data_dir)
        .map_err(|e| format!("could not create data dir {}: {e}", data_dir.display()))?;
    let guard = LaunchGuard::begin(app_root, &spec.app_id).map_err(|e| e.to_string())?;
    let exe = exe.to_string_lossy().into_owned();
    let result = launch(spec, &exe, args).map_err(|e| e.to_string());
    guard.commit();
    result
}

/// Uninstall: remove the app directory *and* its data, as documented.
pub fn remove_app(
    app_root: &std::path::Path,
    data_root: &std::path::Path,
    app_id: &str,
) -> Result<(), String> {
    if !is_valid_app_id(app_id) {
        return Err(format!("invalid app id: {app_id}"));
    }
    let app = app_root.join(app_id);
    let data = data_root.join(app_id);
    if !app.exists() && !data.exists() {
        return Err(format!("app {app_id} is not installed"));
    }
    if app.exists() {
        std::fs::remove_dir_all(&app)
            .map_err(|e| format!("could not remove {}: {e}", app.display()))?;
    }
    if data.exists() {
        std::fs::remove_dir_all(&data)
            .map_err(|e| format!("could not remove {}: {e}", data.display()))?;
    }
    Ok(())
}

/// Same validation the launcher uses: package IDs are portable identifiers,
/// never filesystem paths.
pub fn is_valid_app_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id.as_bytes()[0].is_ascii_alphanumeric()
        && id.split('.').all(|part| {
            !part.is_empty()
                && part
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(tag: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nilrt-lifecycle-{}-{tag}", std::process::id()))
    }

    #[test]
    fn app_id_validation_rejects_traversal() {
        assert!(is_valid_app_id("org.onuron.app"));
        assert!(!is_valid_app_id("../escape"));
        assert!(!is_valid_app_id("a/b"));
        assert!(!is_valid_app_id(""));
    }

    #[test]
    fn launch_guard_leaves_no_staging_residue_and_stale_markers_are_cleaned() {
        let root = temp_root("stage");
        let _ = std::fs::remove_dir_all(&root);
        let marker = launch_stage_path(&root, "org.onuron.app");

        {
            let _guard = LaunchGuard::begin(&root, "org.onuron.app").unwrap();
            assert!(launch_stage_path(&root, "org.onuron.app").exists());
        }
        assert!(
            !launch_stage_path(&root, "org.onuron.app").exists(),
            "dropping the guard must remove the staging marker"
        );

        // Simulate a crash: leave a marker behind, then a later launch sweeps it.
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(&marker, b"stale").unwrap();
        assert_eq!(cleanup_stale_launches(&root), 1);
        assert!(!marker.exists());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_executable_is_reported_not_launched() {
        let root = std::env::temp_dir()
            .join(format!("nilrt-lifecycle-missing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let spec = LaunchSpec {
            app_id: "org.onuron.missing".into(),
            rootfs: String::new(),
            data_dir: root.join("data").to_string_lossy().into_owned(),
            uid: 1000,
            gid: 1000,
            permissions: vec![],
            strict_permissions: false,
        };
        let err = launch_installed(&root, &spec, &[]).unwrap_err();
        assert!(err.contains("no installed executable"), "{err}");
        assert!(!launch_stage_path(&root, &spec.app_id).exists());
        let _ = std::fs::remove_dir_all(&root);
    }
}