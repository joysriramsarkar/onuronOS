// runtime/nilrt/src/permissions.rs — App permission policy (E2).
//
// This module is deliberately *pure*: it turns (granted permissions, a
// requested operation) into a decision and into a concrete restriction plan.
// The kernel-level application of that plan lives in `sandbox.rs`; keeping the
// decision logic dependency-free means it can be unit-tested on the Windows
// dev host, where namespaces/seccomp/mounts are unavailable.
//
// HONESTY NOTE: path checks are *lexical* and do not resolve symlinks or
// account for TOCTOU races. They are a defence-in-depth policy layer, not a
// substitute for the mount/namespace restrictions applied in the sandbox.  A
// full syscall-mediating supervisor (e.g. seccomp SECCOMP_RET_USER_NOTIF) is
// the TODO for adversarial enforcement; see `plan_restrictions`.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};

/// Outcome of a permission check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Allowed,
    Denied { permission: String, reason: String },
}

impl PermissionDecision {
    pub fn is_allowed(&self) -> bool {
        matches!(self, PermissionDecision::Allowed)
    }

    pub fn denied_permission(&self) -> Option<&str> {
        match self {
            PermissionDecision::Denied { permission, .. } => Some(permission.as_str()),
            PermissionDecision::Allowed => None,
        }
    }
}

/// The single source of truth: is `requested` in the app's granted set?
///
/// `granted` is the set of permissions the user/OS actually granted, *not* the
/// set the package asked for in its manifest. A package declaring `camera` but
/// not being granted it must be denied.
pub fn check_app_permissions(granted: &[String], requested: &str) -> PermissionDecision {
    if granted.iter().any(|g| g == requested) {
        PermissionDecision::Allowed
    } else {
        PermissionDecision::Denied {
            permission: requested.to_string(),
            reason: format!("app does not hold the '{requested}' permission"),
        }
    }
}

/// Map a device node path to the permission that gates it. `None` means the
/// path is not a recognised permission-bearing device (it is then governed by
/// the ordinary storage rules).
///
/// Rationale: `/dev/video*` is the V4L2 camera interface, `/dev/snd/*` the
/// ALSA audio devices, `/dev/gps*` a GNSS receiver, `/dev/rfkill` the radio
/// kill switch. A package without the matching permission must not open these.
pub fn device_permission(path: &Path) -> Option<&'static str> {
    let s = path.to_string_lossy();
    if s.starts_with("/dev/video") {
        Some("camera")
    } else if s.starts_with("/dev/snd") || s.starts_with("/dev/dsp") || s.starts_with("/dev/audio") {
        Some("audio")
    } else if s.starts_with("/dev/gps") {
        Some("location")
    } else if s.starts_with("/dev/rfkill") || s.starts_with("/dev/bluetooth") {
        Some("bluetooth")
    } else {
        None
    }
}

/// A per-app policy: where its private data lives and what it may do.
#[derive(Debug, Clone)]
pub struct AppPolicy {
    /// The app's private data directory. Reads/writes here need no storage
    /// permission because the directory is already app-isolated.
    pub data_dir: PathBuf,
    /// Permissions actually granted to the app.
    pub granted: Vec<String>,
}

impl AppPolicy {
    pub fn new(data_dir: impl Into<PathBuf>, granted: Vec<String>) -> Self {
        Self {
            data_dir: data_dir.into(),
            granted,
        }
    }

    fn require(&self, permission: &str) -> PermissionDecision {
        check_app_permissions(&self.granted, permission)
    }

    /// Decide an explicit open()/write on `path`.
    ///
    /// * Paths inside the app's own data dir are always allowed — the data dir
    ///   is the app's private storage and needs no storage permission.
    ///   (Computed lexically; see the module honesty note.)
    /// * A recognised device node is gated by its specific permission
    ///   (`camera`, `audio`, …), regardless of `write`.
    /// * Writing anywhere else requires `storage.write`; reading requires
    ///   `storage.read`.
    pub fn check_path(&self, path: &Path, write: bool) -> PermissionDecision {
        if is_within(path, &self.data_dir) {
            return PermissionDecision::Allowed;
        }
        if let Some(perm) = device_permission(path) {
            return self.require(perm);
        }
        let perm = if write { "storage.write" } else { "storage.read" };
        self.require(perm)
    }

    pub fn has_permission(&self, perm: &str) -> bool {
        self.granted.iter().any(|g| g == perm)
    }
}

/// Concrete restrictions the sandbox should apply for a policy.
///
/// This is the bridge from *policy* to *kernel mechanism*:
///  * `read_only_root` + `writable_paths`: when the app lacks
///    `storage.write`, the sandbox root is remounted read-only and only these
///    paths are bind-mounted back writable.
///  * `device_masks`: device nodes matching these glob patterns are masked by
///    bind-mounting `/dev/null` over them.
///
/// TODO(kernel-enforcement): the mount plan above is the mechanism wired in
/// `sandbox.rs`; a path-checking seccomp user-notification supervisor is the
/// follow-up for finer-grained, race-free mediation.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct RestrictionPlan {
    pub read_only_root: bool,
    pub writable_paths: Vec<String>,
    pub device_masks: Vec<String>,
}

pub fn plan_restrictions(policy: &AppPolicy) -> RestrictionPlan {
    let can_write = policy.has_permission("storage.write");
    let mut device_masks = Vec::new();
    if !policy.has_permission("camera") {
        device_masks.push("/dev/video*".to_string());
    }
    if !policy.has_permission("audio") {
        device_masks.push("/dev/snd/*".to_string());
        device_masks.push("/dev/dsp*".to_string());
    }
    if !policy.has_permission("location") {
        device_masks.push("/dev/gps*".to_string());
    }

    let mut writable_paths = Vec::new();
    if !can_write {
        // Even without storage.write an app needs its own data dir and a
        // scratch space; both are exposed read-write explicitly.
        writable_paths.push(policy.data_dir.to_string_lossy().into_owned());
        writable_paths.push("/tmp".to_string());
    }

    RestrictionPlan {
        read_only_root: !can_write,
        writable_paths,
        device_masks,
    }
}

/// Does `path` sit inside `dir` after lexical normalisation?
///
/// `..` is resolved textually so `<data>/../etc/passwd` is *not* treated as
/// being inside `<data>`. This is intentionally conservative and does not
/// touch the filesystem.
pub fn is_within(path: &Path, dir: &Path) -> bool {
    let p = normalize(path);
    let d = normalize(dir);
    if d.is_empty() || p.len() < d.len() {
        return false;
    }
    p[..d.len()]
        .iter()
        .zip(d.iter())
        .all(|(a, b)| component_eq(a, b))
}

fn component_eq(a: &OsString, b: &OsString) -> bool {
    // Windows filesystems are case-insensitive; fold case there so a path
    // spelled with different capitalisation cannot dodge the data-dir check.
    if cfg!(windows) {
        a.to_string_lossy()
            .eq_ignore_ascii_case(&b.to_string_lossy())
    } else {
        a == b
    }
}

fn normalize(path: &Path) -> Vec<OsString> {
    let mut out: Vec<OsString> = Vec::new();
    for c in path.components() {
        match c {
            Component::Prefix(p) => out.push(p.as_os_str().to_os_string()),
            Component::RootDir => out.push(OsString::from("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                // Only pop a real component; never pop past the root sentinel.
                if out.last().map(|s| s != "/").unwrap_or(false) {
                    out.pop();
                }
            }
            Component::Normal(s) => out.push(s.to_os_string()),
        }
    }
    out
}

/// Match a single-`*` glob (the only form the plan emits) against a path.
pub fn matches_device_pattern(pattern: &str, path: &Path) -> bool {
    let p = path.to_string_lossy();
    match pattern.split_once('*') {
        Some((prefix, suffix)) => p.starts_with(prefix) && p.ends_with(suffix),
        None => p == pattern,
    }
}

/// Reserved for callers that want to reason about an existing path.
pub fn data_dir_for(data_root: &Path, app_id: &str) -> PathBuf {
    data_root.join(app_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn policy(granted: &[&str]) -> AppPolicy {
        AppPolicy::new(
            PathBuf::from("/data/data/org.onuron.app"),
            granted.iter().map(|s| s.to_string()).collect(),
        )
    }

    #[test]
    fn missing_permission_is_denied_and_present_is_allowed() {
        let granted = vec!["storage.read".to_string()];
        assert!(check_app_permissions(&granted, "storage.read").is_allowed());
        let denied = check_app_permissions(&granted, "camera");
        assert_eq!(denied.denied_permission(), Some("camera"));
    }

    #[test]
    fn camera_is_required_for_video_devices() {
        let without = policy(&["storage.read"]);
        let denied = without.check_path(Path::new("/dev/video0"), true);
        assert_eq!(denied.denied_permission(), Some("camera"), "{denied:?}");

        let with = policy(&["camera"]);
        assert!(with.check_path(Path::new("/dev/video0"), true).is_allowed());
        // A different device still needs its own permission.
        assert_eq!(
            with.check_path(Path::new("/dev/snd/pcmC0D0p"), true).denied_permission(),
            Some("audio")
        );
    }

    #[test]
    fn storage_write_is_required_outside_the_data_dir() {
        let without = policy(&["storage.read"]);
        let denied = without.check_path(Path::new("/etc/shadow"), true);
        assert_eq!(denied.denied_permission(), Some("storage.write"));

        let with = policy(&["storage.write"]);
        assert!(with.check_path(Path::new("/sdcard/notes.txt"), true).is_allowed());
    }

    #[test]
    fn writing_inside_the_data_dir_needs_no_storage_permission() {
        let p = policy(&[]);
        assert!(p.check_path(Path::new("/data/data/org.onuron.app/state.json"), true).is_allowed());
        assert!(p
            .check_path(Path::new("/data/data/org.onuron.app/sub/dir/file"), false)
            .is_allowed());
    }

    #[test]
    fn traversal_out_of_the_data_dir_is_not_inside() {
        let p = policy(&[]);
        // `<data>/../secret` must not be mistaken for app-private state.
        let escape = Path::new("/data/data/org.onuron.app/../secret");
        assert!(!is_within(escape, &p.data_dir));
        assert_eq!(
            p.check_path(escape, false).denied_permission(),
            Some("storage.read")
        );
    }

    #[test]
    fn reading_outside_requires_storage_read() {
        let without = policy(&[]);
        assert_eq!(
            without.check_path(Path::new("/etc/hosts"), false).denied_permission(),
            Some("storage.read")
        );
        let with = policy(&["storage.read"]);
        assert!(with.check_path(Path::new("/etc/hosts"), false).is_allowed());
    }

    #[test]
    fn restriction_plan_matches_permissions() {
        let locked = plan_restrictions(&policy(&[]));
        assert!(locked.read_only_root, "no storage.write must mean a read-only root");
        assert!(locked.writable_paths.contains(&"/data/data/org.onuron.app".to_string()));
        assert!(locked.writable_paths.contains(&"/tmp".to_string()));
        assert!(locked.device_masks.iter().any(|m| m == "/dev/video*"),
            "camera-less app must have video devices masked");

        let full = plan_restrictions(&policy(&["storage.write", "camera", "audio", "location"]));
        assert!(!full.read_only_root);
        assert!(full.writable_paths.is_empty());
        assert!(full.device_masks.is_empty());
    }

    #[test]
    fn device_patterns_are_single_star_globs() {
        assert!(matches_device_pattern("/dev/video*", Path::new("/dev/video0")));
        assert!(!matches_device_pattern("/dev/video*", Path::new("/dev/snd/pcmC0D0p")));
        assert!(matches_device_pattern("/dev/snd/*", Path::new("/dev/snd/pcmC0D0p")));
        assert!(!matches_device_pattern("/dev/snd/*", Path::new("/dev/snd")));
        assert!(matches_device_pattern("/dev/gps", Path::new("/dev/gps")));
    }
}