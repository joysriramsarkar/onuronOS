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

/// Deterministically map an app ID to an isolated per-app UID/GID in the range [10000..29999].
/// This ensures per-app process isolation, filesystem DAC separation, and ptrace boundaries.
/// Deterministically map an app ID to an isolated per-app UID/GID in the range [10000..29999].
/// Checks against persisted/active UIDs in a registry to prevent hash collisions.
pub fn allocate_app_uid_with_registry(app_id: &str, registry_path: Option<&Path>) -> u32 {
    if let Ok(override_uid) = env::var("NIL_APP_UID") {
        if let Ok(uid) = override_uid.parse::<u32>() {
            return uid;
        }
    }

    let mut uids_map: std::collections::HashMap<String, u32> = std::collections::HashMap::new();
    if let Some(path) = registry_path {
        if let Ok(content) = std::fs::read_to_string(path) {
            if let Ok(map) = serde_json::from_str::<std::collections::HashMap<String, u32>>(&content) {
                uids_map = map;
            }
        }
    }

    if let Some(&existing_uid) = uids_map.get(app_id) {
        return existing_uid;
    }

    let mut hash: u32 = 0x811c_9dc5;
    for &byte in app_id.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    let mut candidate = 10000 + (hash % 20000);

    let allocated_uids: std::collections::HashSet<u32> = uids_map.values().copied().collect();
    let mut probed = 0;
    while allocated_uids.contains(&candidate) && probed < 20000 {
        candidate = 10000 + ((candidate - 10000 + 1) % 20000);
        probed += 1;
    }

    if let Some(path) = registry_path {
        uids_map.insert(app_id.to_string(), candidate);
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(json) = serde_json::to_string_pretty(&uids_map) {
            let _ = std::fs::write(path, json);
        }
    }

    candidate
}

pub fn allocate_app_uid(app_id: &str) -> u32 {
    let default_reg = if cfg!(target_os = "windows") {
        env::var_os("USERPROFILE")
            .map(|p| PathBuf::from(p).join(".onuron").join("nilrt").join("app_uids.json"))
            .unwrap_or_else(|| PathBuf::from("app_uids.json"))
    } else {
        PathBuf::from("/data/nilrt/app_uids.json")
    };
    let reg_path = env::var_os("NIL_UID_REGISTRY")
        .map(PathBuf::from)
        .unwrap_or(default_reg);
    allocate_app_uid_with_registry(app_id, Some(&reg_path))
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

    let app_uid = allocate_app_uid(app_id);
    println!("[nilrt-launch] Allocated per-app isolated UID/GID: {}", app_uid);

    let spec = LaunchSpec {
        app_id: app_id.clone(),
        rootfs: app_dir.join("root").to_string_lossy().into_owned(),
        data_dir: data_dir.to_string_lossy().into_owned(),
        uid: app_uid,
        gid: app_uid,
        permissions: granted,
        strict_permissions: strict,
    };

    let mut launch_args: Vec<String> = Vec::new();
    if let Some(snap) = snapshot_path {
        launch_args.push("--restore".to_string());
        launch_args.push(snap.clone());
    }

    // Prefer the installed payload; check manifest.exec, then bin/<app_id> or .nib
    let mut installed_binary: Option<PathBuf> = None;
    let manifest_path = app_dir.join("manifest.json");
    if let Ok(text) = std::fs::read_to_string(&manifest_path) {
        if let Ok(val) = serde_json::from_str::<serde_json::Value>(&text) {
            if let Some(exec_rel) = val.get("exec").and_then(|v| v.as_str()) {
                let candidate = app_dir.join(exec_rel);
                if candidate.is_file() {
                    installed_binary = Some(candidate);
                }
            }
        }
    }

    if installed_binary.is_none() {
        let bin_candidate = app_dir.join("bin").join(app_id);
        if bin_candidate.is_file() {
            installed_binary = Some(bin_candidate);
        } else if cfg!(target_os = "windows") && app_dir.join("bin").join(format!("{app_id}.exe")).is_file() {
            installed_binary = Some(app_dir.join("bin").join(format!("{app_id}.exe")));
        } else if app_dir.join(format!("{app_id}.nib")).is_file() {
            installed_binary = Some(app_dir.join(format!("{app_id}.nib")));
        }
    }

    let result = if let Some(target) = installed_binary {
        let is_nib = target.extension().and_then(|s| s.to_str()) == Some("nib")
            || std::fs::read(&target).map(|bytes| bytes.starts_with(b"NIB1")).unwrap_or(false);

        if is_nib {
            println!("[nilrt-launch] Executing compiled NilLang bytecode via nilc runtime engine");
            launch_args.insert(0, target.to_string_lossy().into_owned());
            launch_args.insert(0, "run".to_string());
            let nilc_cmd = if Path::new("/usr/bin/nilc").is_file() {
                "/usr/bin/nilc"
            } else {
                "nilc"
            };
            lifecycle::launch(&spec, nilc_cmd, &launch_args).map_err(|e| e.to_string())
        } else {
            lifecycle::launch(&spec, &target.to_string_lossy(), &launch_args).map_err(|e| e.to_string())
        }
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
    use super::{allocate_app_uid, validate_app_id};

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

    #[test]
    fn allocates_deterministic_isolated_uids() {
        let uid1 = allocate_app_uid("org.onuron.calculator");
        let uid2 = allocate_app_uid("org.onuron.calculator");
        let uid3 = allocate_app_uid("org.onuron.camera");
        assert_eq!(uid1, uid2, "UID allocation must be deterministic");
        assert_ne!(uid1, uid3, "Different apps should receive different UIDs");
        assert!((10000..30000).contains(&uid1), "UID must be in isolated range: {uid1}");
        assert!((10000..30000).contains(&uid3), "UID must be in isolated range: {uid3}");
    }

    #[test]
    fn test_collision_free_probe() {
        let tmp_dir = std::env::temp_dir().join(format!("nilrt_test_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&tmp_dir);
        let reg_path = tmp_dir.join("uids.json");

        let uid1 = super::allocate_app_uid_with_registry("app.one", Some(&reg_path));
        let uid2 = super::allocate_app_uid_with_registry("app.one", Some(&reg_path));
        assert_eq!(uid1, uid2, "Same app must receive identical persistent UID");

        let mut map = std::collections::HashMap::new();
        map.insert("app.fake".to_string(), 15000);
        let json = serde_json::to_string(&map).unwrap();
        std::fs::write(&reg_path, json).unwrap();

        let uid3 = super::allocate_app_uid_with_registry("app.fake", Some(&reg_path));
        assert_eq!(uid3, 15000);

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}