// runtime/nilrt/src/bin/nilrt-launch.rs — App Launcher with Snapshot Restore
//
// Wires the permission model (E2) and the lifecycle/sandbox path (E1) into the
// real launcher: the installed manifest's requested permissions are combined
// with what the permission broker actually granted, then the app is launched
// through the sandbox with enforcement enabled.
use std::env;
use std::path::{Path, PathBuf};

use nilrt::lifecycle::{self, LaunchSpec};
use nilrt::PermissionBroker;

/// Minimal view of an installed manifest. Extra fields are ignored.
#[derive(serde::Deserialize)]
struct PartialManifest {
    #[serde(default)]
    permissions: Vec<String>,
}

/// Package IDs are portable identifiers, never filesystem paths. Without this
/// an app ID like `../../bin/sh` escapes /data/app and launches any binary.
fn validate_app_id(id: &str) -> Result<(), String> {
    if lifecycle::is_valid_app_id(id) {
        Ok(())
    } else {
        Err(format!("invalid app id: {id}"))
    }
}

fn env_path(key: &str, default: &str) -> PathBuf {
    env::var_os(key)
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(default))
}

/// Permissions *requested* by the installed package (empty if no manifest).
fn requested_permissions(app_dir: &Path) -> Vec<String> {
    let manifest = app_dir.join("manifest.json");
    match std::fs::read_to_string(&manifest) {
        Ok(text) => match serde_json::from_str::<PartialManifest>(&text) {
            Ok(m) => m.permissions,
            Err(e) => {
                eprintln!("[nilrt-launch] ignoring unreadable {}: {e}", manifest.display());
                Vec::new()
            }
        },
        Err(_) => Vec::new(),
    }
}

/// Keep only the permissions the broker actually granted. A package that asks
/// for `camera` but was never granted it must not receive it.
fn granted_permissions(
    broker: &mut PermissionBroker,
    app_id: &str,
    requested: &[String],
) -> Vec<String> {
    requested
        .iter()
        .filter(|perm| broker.check_permission(app_id, perm))
        .cloned()
        .collect()
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: nilrt-launch <app_id> [snapshot_payload_path]");
        std::process::exit(2);
    }

    let app_id = &args[1];
    let snapshot_path = args.get(2);

    if let Err(e) = validate_app_id(app_id) {
        eprintln!("[nilrt-launch] Refusing to launch: {e}");
        std::process::exit(1);
    }

    let default_app = if cfg!(target_os = "windows") {
        env::var_os("USERPROFILE")
            .map(|p| PathBuf::from(p).join(".onuron").join("apps"))
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        PathBuf::from("/data/app")
    };
    let app_root = env_path("NIL_APP_ROOT", &default_app.to_string_lossy());

    let default_data = if cfg!(target_os = "windows") {
        env::var_os("USERPROFILE")
            .map(|p| PathBuf::from(p).join(".onuron").join("data"))
            .unwrap_or_else(|| PathBuf::from("."))
    } else {
        PathBuf::from("/data/data")
    };
    let data_root = env_path("NIL_DATA_ROOT", &default_data.to_string_lossy());
    let app_dir = app_root.join(app_id);
    let data_dir = data_root.join(app_id);

    let requested = requested_permissions(&app_dir);

    let default_db = if cfg!(target_os = "windows") {
        env::var_os("USERPROFILE")
            .map(|p| PathBuf::from(p).join(".onuron").join("nilrt").join("permissions.json"))
            .unwrap_or_else(|| PathBuf::from("permissions.json"))
    } else {
        PathBuf::from("/data/nilrt/permissions.json")
    };
    let db_path = env_path("NIL_PERM_DB", &default_db.to_string_lossy());
    let mut broker = PermissionBroker::new(&db_path.to_string_lossy());
    let granted = granted_permissions(&mut broker, app_id, &requested);

    // Enforce by default once an app is installed (has a manifest); allow an
    // explicit opt-out for bring-up with `NIL_PERMISSION_ENFORCE=0`.
    let manifest_present = app_dir.join("manifest.json").is_file();
    let strict = match env::var("NIL_PERMISSION_ENFORCE").as_deref() {
        Ok("0") => false,
        Ok("1") => true,
        _ => manifest_present,
    };

    println!("[nilrt-launch] Launching application: {}", app_id);
    println!(
        "[nilrt-launch] permissions requested={:?} granted={:?} enforce={}",
        requested, granted, strict
    );
    if let Some(snap) = snapshot_path {
        println!("[nilrt-launch] Restoring from Handoff Snapshot: {}", snap);
    }

    let spec = LaunchSpec {
        app_id: app_id.clone(),
        rootfs: app_dir.join("root").to_string_lossy().into_owned(),
        data_dir: data_dir.to_string_lossy().into_owned(),
        uid: 1000,
        gid: 1000,
        permissions: granted,
        strict_permissions: strict,
    };

    let mut launch_args: Vec<String> = Vec::new();
    if let Some(snap) = snapshot_path {
        launch_args.push("--restore".to_string());
        launch_args.push(snap.clone());
    }

    // Prefer the installed payload; fall back to a system binary (dev hosts).
    let has_installed_bin = app_dir.join("bin").join(app_id).is_file()
        || (cfg!(target_os = "windows") && app_dir.join("bin").join(format!("{app_id}.exe")).is_file());
    let result = if has_installed_bin {
        lifecycle::launch_installed(&app_root, &spec, &launch_args)
    } else {
        let system = PathBuf::from("/usr/bin").join(app_id);
        if system.is_file() {
            lifecycle::launch(&spec, &system.to_string_lossy(), &launch_args)
                .map_err(|e| e.to_string())
        } else {
            Err(format!(
                "no installed executable for {app_id} in {} or /usr/bin",
                app_dir.display()
            ))
        }
    };

    if let Err(e) = result {
        eprintln!("[nilrt-launch] Failed to launch {}: {e}", app_id);
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::validate_app_id;

    #[test]
    fn rejects_traversal_and_shell_metacharacters() {
        for id in ["../bin/sh", "..", ".", "a/b", "a\\b", "", "a b", "a;id", "a$(id)", "é"] {
            assert!(validate_app_id(id).is_err(), "accepted {id:?}");
        }
    }

    #[test]
    fn accepts_normal_package_ids() {
        for id in ["notes", "org.onuron.notes", "com.signal.android", "my-app_2"] {
            assert!(validate_app_id(id).is_ok(), "rejected {id:?}");
        }
        assert!(validate_app_id(&"a".repeat(129)).is_err());
    }
}