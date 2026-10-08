// pkg/nilpkg/src/lib.rs — Onuron OS package manager library.
// Signed, atomic, reproducible package management with SHA-256 payload
// integrity and Ed25519 publisher signatures. The `nilpkg` binary is a thin
// CLI over this crate; `nilupd` reuses the durable/lock/fetch machinery.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer};
use rand::rngs::OsRng;

pub mod durable;
pub mod extract;
pub mod fetch;
pub mod keymgmt;
pub mod lockfile;
use durable::{sync_dir, sync_tree, write_file_atomic};
use keymgmt::{encrypt_private_key, read_passphrase, write_encrypted_key};
use lockfile::PackageLock;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Manifest {
    pub name: String,
    pub app_id: String,
    pub version: String,
    pub arch: String,
    pub min_os_version: String,
    pub description: String,
    pub permissions: Vec<String>,
    pub exec: String,
    pub sha256: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub signature_hex: Option<String>,
    #[serde(default)]
    pub public_key_hex: Option<String>,
    /// SHA-256 of the whole archive the manifest was published with. Optional
    /// (older manifests predate it); when present it is covered by the
    /// signature and checked before the archive is accepted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub archive_sha256: Option<String>,
}

/// Compute true cryptographic SHA-256 hex digest using sha2
pub fn compute_sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    format!("{:064x}", result)
}

/// Generate a new Ed25519 signing keypair
pub fn generate_keypair() -> (SigningKey, VerifyingKey) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    (signing_key, verifying_key)
}

/// Sign bytes using an Ed25519 signing key
pub fn sign_bytes(signing_key: &SigningKey, data: &[u8]) -> Signature {
    signing_key.sign(data)
}

/// Verify an Ed25519 signature over bytes
pub fn verify_signature(verifying_key: &VerifyingKey, data: &[u8], signature: &Signature) -> bool {
    verifying_key.verify_strict(data, signature).is_ok()
}

/// Package IDs are portable directory names, never filesystem paths or shell input.
pub fn validate_app_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 128 || id.split('.').any(|part| {
        part.is_empty() || !part.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
    }) || !id.as_bytes()[0].is_ascii_alphanumeric() {
        return Err("Invalid package ID: use 1–128 ASCII letters, digits, dots, underscores or hyphens; start with a letter or digit and do not use empty dot-separated components".into());
    }
    // Avoid Windows device names even when packages are prepared on Linux.
    let stem = id.split('.').next().unwrap().to_ascii_uppercase();
    if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (stem.len() == 4 && (stem.starts_with("COM") || stem.starts_with("LPT"))
            && matches!(stem.as_bytes()[3], b'1'..=b'9')) {
        return Err("Reserved package ID".into());
    }
    Ok(())
}

pub fn get_app_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        base.join(".onuron").join("apps")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/data/app")
    }
}

pub fn get_key_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        base.join(".onuron").join("keys")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/etc/onuron/keys")
    }
}

/// Where fetched archives are cached until installed or pruned.
pub fn get_cache_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."));
        base.join(".onuron").join("cache")
    }
    #[cfg(not(target_os = "windows"))]
    {
        PathBuf::from("/var/cache/nilpkg")
    }
}

/// Sign a manifest by computing hash and signing it with an Ed25519 key
pub fn sign_manifest(manifest: &mut Manifest, signing_key: &SigningKey) {
    // Clear signature fields before hashing the canonical manifest payload
    manifest.signature_hex = None;
    manifest.public_key_hex = None;

    let canonical_bytes = serde_json::to_vec(manifest).expect("Failed to serialize manifest");
    let sig = sign_bytes(signing_key, &canonical_bytes);
    let pubkey = signing_key.verifying_key();

    manifest.signature_hex = Some(hex::encode(sig.to_bytes()));
    manifest.public_key_hex = Some(hex::encode(pubkey.to_bytes()));
}

/// Verify a manifest's cryptographic signature and SHA-256 payload integrity
pub fn verify_manifest(manifest: &Manifest, payload_bytes: &[u8]) -> Result<(), String> {
    validate_app_id(&manifest.app_id)?;
    if manifest.size_bytes != payload_bytes.len() as u64 {
        return Err("Payload size does not match manifest size_bytes".into());
    }
    // 1. Verify payload SHA-256 digest
    let actual_sha = compute_sha256(payload_bytes);
    if manifest.sha256 != actual_sha {
        return Err(format!(
            "SHA-256 Integrity check failed! Expected {}, got {}",
            manifest.sha256, actual_sha
        ));
    }

    // 2. Verify signature presence
    let sig_hex = manifest.signature_hex.as_ref().ok_or("Package is unsigned (missing signature_hex)")?;
    let pub_hex = manifest.public_key_hex.as_ref().ok_or("Package is missing public_key_hex")?;

    let sig_bytes = hex::decode(sig_hex).map_err(|e| format!("Invalid signature hex: {}", e))?;
    let pub_bytes = hex::decode(pub_hex).map_err(|e| format!("Invalid public key hex: {}", e))?;

    if sig_bytes.len() != 64 {
        return Err("Invalid Ed25519 signature length (must be 64 bytes)".into());
    }
    if pub_bytes.len() != 32 {
        return Err("Invalid Ed25519 public key length (must be 32 bytes)".into());
    }

    let mut sig_arr = [0u8; 64];
    sig_arr.copy_from_slice(&sig_bytes);
    let signature = Signature::from_bytes(&sig_arr);

    let mut pub_arr = [0u8; 32];
    pub_arr.copy_from_slice(&pub_bytes);
    let verifying_key = VerifyingKey::from_bytes(&pub_arr)
        .map_err(|e| format!("Invalid Ed25519 public key bytes: {}", e))?;

    // Create a copy of manifest with cleared signature fields for canonical byte verification
    let mut canonical_manifest = manifest.clone();
    canonical_manifest.signature_hex = None;
    canonical_manifest.public_key_hex = None;
    let canonical_bytes = serde_json::to_vec(&canonical_manifest)
        .map_err(|e| format!("Serialization error: {}", e))?;

    if !verify_signature(&verifying_key, &canonical_bytes, &signature) {
        return Err("Ed25519 cryptographic signature verification failed! Package may be tampered.".into());
    }

    Ok(())
}

/// Current OS release that packages declare compatibility against.
pub const CURRENT_OS_VERSION: &str = "1.0.0";

pub fn current_arch() -> &'static str {
    if cfg!(target_arch = "x86_64") {
        "x86_64"
    } else if cfg!(target_arch = "aarch64") {
        "aarch64"
    } else {
        "unknown"
    }
}

/// Permissions an app may request. Anything else is rejected at install time,
/// so a signed manifest cannot grant itself a capability the OS does not know.
pub const ALLOWED_PERMISSIONS: &[&str] = &[
    "network",
    "storage.read",
    "storage.write",
    "display",
    "input",
    "audio",
    "camera",
    "location",
    "contacts",
    "sms",
    "phone",
    "sensors",
    "bluetooth",
    "wifi",
    "cellular",
];

pub fn check_compatibility(manifest: &Manifest) -> Result<(), String> {
    if manifest.arch != current_arch() {
        return Err(format!(
            "Incompatible architecture '{}': this system is '{}'",
            manifest.arch,
            current_arch()
        ));
    }
    match compare_versions(&manifest.min_os_version, CURRENT_OS_VERSION) {
        Some(std::cmp::Ordering::Greater) => {
            return Err(format!(
                "Package requires OS {} but this system is {}",
                manifest.min_os_version, CURRENT_OS_VERSION
            ));
        }
        None => {
            return Err(format!(
                "Invalid min_os_version '{}': must be dotted-numeric like 1.0.0",
                manifest.min_os_version
            ));
        }
        _ => {}
    }
    if manifest.permissions.len() > 32 {
        return Err("Too many permissions requested (max 32)".into());
    }
    for perm in &manifest.permissions {
        if !ALLOWED_PERMISSIONS.contains(&perm.as_str()) {
            return Err(format!("Unknown permission requested: '{perm}'"));
        }
    }
    Ok(())
}

/// A revoked publisher key is rejected even if it is also present in trusted/.
pub fn is_revoked(key_dir: &Path, key: &[u8]) -> bool {
    let revoked_dir = key_dir.join("revoked");
    let Ok(entries) = fs::read_dir(&revoked_dir) else {
        return false;
    };
    for entry in entries.flatten() {
        if !entry.file_type().map(|t| t.is_file()).unwrap_or(false) {
            continue;
        }
        if entry.path().extension().and_then(|e| e.to_str()) != Some("pub") {
            continue;
        }
        let Ok(contents) = fs::read_to_string(entry.path()) else {
            continue;
        };
        if hex::decode(contents.trim()).ok().as_deref() == Some(key) {
            return true;
        }
    }
    false
}

/// Only explicit local trust anchors authorize installation. A key embedded in
/// a manifest is not sufficient to establish who published a package.
pub fn trusted_publisher(key_dir: &Path, manifest: &Manifest) -> Result<(), String> {
    let expected = manifest.public_key_hex.as_deref().ok_or("Missing publisher key")?;
    let key = hex::decode(expected)?;
    if key.len() != 32 { return Err("Invalid publisher key length".into()); }
    if is_revoked(key_dir, &key) {
        return Err("Publisher key has been revoked".into());
    }
    let trust_dir = key_dir.join("trusted");
    let entries = fs::read_dir(&trust_dir)
        .map_err(|_| format!("No trusted publishers found in {}", trust_dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() { continue; }
        if entry.path().extension().and_then(|ext| ext.to_str()) != Some("pub") { continue; }
        let contents = fs::read_to_string(entry.path()).map_err(|e| e.to_string())?;
        if hex::decode(contents.trim()).ok().as_deref() == Some(key.as_slice()) {
            return Ok(());
        }
    }
    Err("Publisher is not trusted; add its verified public key to the trusted key directory".into())
}

pub fn regular_file(path: &Path) -> Result<(), String> {
    let metadata = fs::symlink_metadata(path).map_err(|e| e.to_string())?;
    if !metadata.file_type().is_file() { return Err(format!("Not a regular file: {}", path.display())); }
    Ok(())
}

/// Read only the package format we support; never follow directory or file
/// symlinks supplied by a package or an installed app.
pub fn read_package(source: &Path) -> Result<(Manifest, Vec<u8>), String> {
    if !fs::symlink_metadata(source).map_err(|e| e.to_string())?.file_type().is_dir() {
        return Err("Package source must be a directory (not a symlink)".into());
    }
    let manifest_path = source.join("manifest.json");
    regular_file(&manifest_path)?;
    if fs::metadata(&manifest_path).map_err(|e| e.to_string())?.len() > 64 * 1024 {
        return Err("Manifest is too large".into());
    }
    let manifest: Manifest = serde_json::from_slice(&fs::read(&manifest_path).map_err(|e| e.to_string())?)
        .map_err(|e| format!("Invalid manifest: {e}"))?;
    validate_app_id(&manifest.app_id)?;
    let executable = manifest.exec.strip_prefix("bin/")
        .ok_or("Executable must be directly inside bin/")?;
    validate_app_id(executable)?;
    if manifest.size_bytes > 128 * 1024 * 1024 { return Err("Payload exceeds 128 MiB limit".into()); }
    if !fs::symlink_metadata(source.join("bin")).map_err(|e| e.to_string())?.file_type().is_dir() {
        return Err("Package bin/ must be a directory (not a symlink)".into());
    }
    let payload_path = source.join("bin").join(executable);
    regular_file(&payload_path)?;
    let file = fs::File::open(&payload_path).map_err(|e| e.to_string())?;
    let mut payload = Vec::new();
    file.take(128 * 1024 * 1024 + 1).read_to_end(&mut payload).map_err(|e| e.to_string())?;
    verify_manifest(&manifest, &payload)?;
    check_compatibility(&manifest)?;
    Ok((manifest, payload))
}

pub fn verify_installed(app_id: &str, app_root: &Path, key_dir: &Path) -> Result<(), String> {
    validate_app_id(app_id)?;
    let (manifest, _) = read_package(&app_root.join(app_id))?;
    if manifest.app_id != app_id { return Err("Installed directory does not match signed app ID".into()); }
    trusted_publisher(key_dir, &manifest)?;
    Ok(())
}

/// Compare dotted numeric versions. Returns None when either side is not a
/// plain dotted-numeric version, so callers never invent an ordering.
pub fn compare_versions(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    let parse = |v: &str| -> Option<Vec<u64>> {
        if v.is_empty() { return None; }
        v.split('.').map(|p| p.parse::<u64>().ok()).collect()
    };
    let (mut left, mut right) = (parse(a)?, parse(b)?);
    let len = left.len().max(right.len());
    left.resize(len, 0);
    right.resize(len, 0);
    Some(left.cmp(&right))
}

pub fn journal_path(app_root: &Path, app_id: &str) -> PathBuf {
    app_root.join(format!(".nilpkg-journal-{}.json", app_id))
}

pub fn backup_path(app_root: &Path, app_id: &str) -> PathBuf {
    app_root.join(format!(".nilpkg-backup-{}", app_id))
}

/// Stage a verified package directory under the app root.
pub fn stage_package(
    app_root: &Path,
    manifest: &Manifest,
    executable: &str,
    payload: &[u8],
) -> Result<PathBuf, String> {
    let stage = app_root.join(format!(".nilpkg-{:016x}", rand::random::<u64>()));
    fs::create_dir(&stage).map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        fs::create_dir(stage.join("bin")).map_err(|e| e.to_string())?;
        let manifest_bytes = serde_json::to_vec_pretty(manifest).map_err(|e| e.to_string())?;
        write_file_atomic(&stage.join("manifest.json"), &manifest_bytes)?;
        let dest = stage.join("bin").join(executable);
        write_file_atomic(&dest, payload)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&dest, fs::Permissions::from_mode(0o755)).map_err(|e| e.to_string())?;
        }
        // Flush the whole staged tree before it becomes live so a power loss
        // cannot leave a package directory with truncated contents.
        sync_tree(&stage)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_dir_all(&stage);
    }
    result?;
    Ok(stage)
}

/// Complete an interrupted upgrade. Called before any install/upgrade so a
/// crash between the two renames cannot leave an app permanently missing.
pub fn recover_pending(app_root: &Path) -> Result<(), String> {
    let entries = match fs::read_dir(app_root) {
        Ok(e) => e,
        Err(_) => return Ok(()),
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(app_id) = name.strip_prefix(".nilpkg-journal-").and_then(|n| n.strip_suffix(".json")) else {
            continue;
        };
        let target = app_root.join(app_id);
        let backup = backup_path(app_root, app_id);
        if target.exists() {
            // New version already in place; the backup is now redundant.
            let _ = fs::remove_dir_all(&backup);
        } else if backup.exists() {
            fs::rename(&backup, &target)
                .map_err(|e| format!("Could not restore interrupted upgrade of {app_id}: {e}"))?;
            eprintln!("[nilpkg] Recovered interrupted upgrade of {app_id} (previous version restored)");
        }
        let _ = fs::remove_file(entry.path());
    }
    // Drop orphaned staging directories from a previous crashed run.
    if let Ok(entries) = fs::read_dir(app_root) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with(".nilpkg-")
                && !name.starts_with(".nilpkg-backup-")
                && !name.starts_with(".nilpkg-journal-")
            {
                let _ = fs::remove_dir_all(entry.path());
            }
        }
    }
    Ok(())
}

/// Replace an installed package with a newer signed version. The swap uses a
/// journal plus two renames so an interruption is recoverable rather than
/// leaving the app half-installed.
pub fn upgrade_local(source: &Path, app_root: &Path, key_dir: &Path, allow_downgrade: bool) -> Result<(), String> {
    let (manifest, payload) = read_package(source)?;
    let executable = manifest.exec.strip_prefix("bin/").expect("validated by read_package");
    trusted_publisher(key_dir, &manifest)?;

    // Serialize with other nilpkg processes before touching the app root.
    let _lock = PackageLock::acquire(app_root)?;
    // Complete any previously interrupted swap before deciding what is installed.
    recover_pending(app_root)?;
    let target = app_root.join(&manifest.app_id);
    if !target.exists() {
        return Err(format!("{} is not installed; use 'nilpkg install' first", manifest.app_id));
    }
    let installed = read_package(&target)?.0;
    if installed.version == manifest.version {
        return Err(format!("Version {} is already installed", manifest.version));
    }
    if !allow_downgrade {
        match compare_versions(&manifest.version, &installed.version) {
            Some(std::cmp::Ordering::Less) => {
                return Err(format!(
                    "Refusing to downgrade {} from {} to {} (pass --allow-downgrade to force)",
                    manifest.app_id, installed.version, manifest.version
                ));
            }
            None => {
                return Err(format!(
                    "Cannot compare versions '{}' and '{}'; pass --allow-downgrade to force",
                    installed.version, manifest.version
                ));
            }
            _ => {}
        }
    }

    let stage = stage_package(app_root, &manifest, executable, &payload)?;
    let backup = backup_path(app_root, &manifest.app_id);
    let journal = journal_path(app_root, &manifest.app_id);

    // Journal first: if we die after moving the old version aside, recovery
    // knows to put it back. The journal is fsynced before the swap begins so
    // the recovery information cannot itself be lost to power failure.
    let journal_bytes = serde_json::to_vec(&manifest).map_err(|e| e.to_string())?;
    write_file_atomic(&journal, &journal_bytes)?;

    let _ = fs::remove_dir_all(&backup);
    if let Err(e) = fs::rename(&target, &backup) {
        let _ = fs::remove_dir_all(&stage);
        let _ = fs::remove_file(&journal);
        return Err(format!("Could not move current version aside: {e}"));
    }
    sync_dir(app_root)?;
    if let Err(e) = fs::rename(&stage, &target) {
        // Restore the previous version so the app is never left missing.
        let _ = fs::remove_dir_all(&stage);
        let restored = fs::rename(&backup, &target);
        let _ = fs::remove_file(&journal);
        return Err(match restored {
            Ok(_) => format!("Upgrade failed and was rolled back: {e}"),
            Err(re) => format!("Upgrade failed ({e}) AND rollback failed ({re}); backup kept at {}", backup.display()),
        });
    }
    sync_dir(app_root)?;
    let _ = fs::remove_file(&journal);
    sync_dir(app_root)?;
    // Keep the backup for explicit rollback.
    println!(
        "[nilpkg] Upgraded {} from {} to {}",
        manifest.app_id, installed.version, manifest.version
    );
    println!("[nilpkg] Previous version kept at {} (use 'nilpkg rollback {}' to revert)", backup.display(), manifest.app_id);
    Ok(())
}

/// Restore the version replaced by the most recent upgrade.
pub fn rollback_package(app_id: &str, app_root: &Path) -> Result<(), String> {
    validate_app_id(app_id)?;
    // Serialize with other nilpkg processes before touching the app root.
    let _lock = PackageLock::acquire(app_root)?;
    // If a previous swap was interrupted, restore that state before rolling back.
    recover_pending(app_root)?;
    let target = app_root.join(app_id);
    let backup = backup_path(app_root, app_id);
    if !backup.exists() {
        return Err(format!("No previous version of {app_id} is available to roll back to"));
    }
    let previous = read_package(&backup)?.0;
    let swap = app_root.join(format!(".nilpkg-swap-{:016x}", rand::random::<u64>()));
    if target.exists() {
        fs::rename(&target, &swap).map_err(|e| format!("Could not move current version aside: {e}"))?;
    }
    if let Err(e) = fs::rename(&backup, &target) {
        let _ = fs::rename(&swap, &target);
        return Err(format!("Rollback failed: {e}"));
    }
    if swap.exists() {
        // Keep the version we just replaced so rollback can be undone.
        let _ = fs::remove_dir_all(&backup);
        fs::rename(&swap, &backup).map_err(|e| format!("Rolled back, but could not retain the newer version: {e}"))?;
    }
    println!("[nilpkg] Rolled back {app_id} to version {}", previous.version);
    Ok(())
}

pub fn install_local(source: &Path, app_root: &Path, key_dir: &Path) -> Result<(), String> {
    if source.is_file() {
        let temp_dir = app_root.join(format!(".staging_{}", rand::random::<u64>()));
        fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
        let res = extract::extract_archive(source, &temp_dir)
            .and_then(|()| install_local(&temp_dir, app_root, key_dir));
        let _ = fs::remove_dir_all(&temp_dir);
        return res;
    }

    let (manifest, payload) = read_package(source)?;
    let executable = manifest.exec.strip_prefix("bin/").expect("validated by read_package");
    trusted_publisher(key_dir, &manifest)?;

    // Serialize with other nilpkg processes before touching the app root.
    let _lock = PackageLock::acquire(app_root)?;
    fs::create_dir_all(app_root).map_err(|e| e.to_string())?;
    recover_pending(app_root)?;
    let target = app_root.join(&manifest.app_id);
    if fs::symlink_metadata(&target).is_ok() {
        return Err(format!(
            "{} is already installed; use 'nilpkg upgrade <directory>' to update it",
            manifest.app_id
        ));
    }
    let stage = stage_package(app_root, &manifest, executable, &payload)?;
    // Re-check under the lock so two processes cannot both believe the ID is free.
    if fs::symlink_metadata(&target).is_ok() {
        let _ = fs::remove_dir_all(&stage);
        return Err("Package already installed".into());
    }
    if let Err(e) = fs::rename(&stage, &target) {
        let _ = fs::remove_dir_all(&stage);
        return Err(e.to_string());
    }
    sync_dir(app_root)?;
    println!("[nilpkg] Installed {} into {}", manifest.app_id, target.display());
    Ok(())
}

/// Pack a binary into a signed `.nilax` package directory or archive file.
pub fn pack_package(
    app_id: &str,
    name: &str,
    version: &str,
    description: &str,
    permissions: Vec<String>,
    executable_path: &Path,
    output_path: &Path,
    signing_key: &SigningKey,
) -> Result<Manifest, String> {
    validate_app_id(app_id)?;
    if !executable_path.is_file() {
        return Err(format!("Executable not found at {}", executable_path.display()));
    }

    let payload = fs::read(executable_path).map_err(|e| format!("Could not read executable: {e}"))?;
    let sha256 = compute_sha256(&payload);

    let mut manifest = Manifest {
        name: name.to_string(),
        app_id: app_id.to_string(),
        version: version.to_string(),
        arch: current_arch().to_string(),
        min_os_version: "1.0.0".to_string(),
        description: description.to_string(),
        permissions,
        exec: format!("bin/{app_id}"),
        sha256,
        size_bytes: payload.len() as u64,
        signature_hex: None,
        public_key_hex: None,
        archive_sha256: None,
    };

    sign_manifest(&mut manifest, signing_key);

    let is_archive = output_path.extension().and_then(|e| e.to_str()) == Some("nilax")
        || output_path.extension().and_then(|e| e.to_str()) == Some("tar");

    if is_archive {
        let manifest_bytes = serde_json::to_vec_pretty(&manifest)
            .map_err(|e| format!("Could not serialize manifest: {e}"))?;

        let file = fs::File::create(output_path)
            .map_err(|e| format!("Could not create archive file {}: {e}", output_path.display()))?;
        let mut builder = tar::Builder::new(file);

        let mut m_hdr = tar::Header::new_gnu();
        m_hdr.set_size(manifest_bytes.len() as u64);
        m_hdr.set_mode(0o644);
        m_hdr.set_cksum();
        builder.append_data(&mut m_hdr, "manifest.json", &manifest_bytes[..])
            .map_err(|e| format!("Could not write manifest to archive: {e}"))?;

        let mut p_hdr = tar::Header::new_gnu();
        p_hdr.set_size(payload.len() as u64);
        p_hdr.set_mode(0o755);
        p_hdr.set_cksum();
        builder.append_data(&mut p_hdr, format!("bin/{app_id}"), &payload[..])
            .map_err(|e| format!("Could not write executable to archive: {e}"))?;

        builder.finish().map_err(|e| format!("Could not finish archive: {e}"))?;
    } else {
        let bin_dir = output_path.join("bin");
        fs::create_dir_all(&bin_dir).map_err(|e| format!("Could not create bin directory: {e}"))?;

        let manifest_json = serde_json::to_string_pretty(&manifest)
            .map_err(|e| format!("Could not serialize manifest: {e}"))?;
        fs::write(output_path.join("manifest.json"), manifest_json)
            .map_err(|e| format!("Could not write manifest.json: {e}"))?;

        fs::write(bin_dir.join(app_id), &payload)
            .map_err(|e| format!("Could not write binary: {e}"))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(bin_dir.join(app_id), fs::Permissions::from_mode(0o755));
        }
    }

    Ok(manifest)
}

pub fn install_demo(pkg_identifier: &str) -> Result<(), String> {
    validate_app_id(pkg_identifier)?;
    println!("[nilpkg] Installing package: {}", pkg_identifier);
    let app_root = get_app_dir();
    fs::create_dir_all(&app_root).map_err(|e| format!("Could not create app dir: {}", e))?;

    let app_id = match pkg_identifier {
        "firefox" | "fenix" | "org.mozilla.fenix" => "org.mozilla.fenix",
        "signal" | "com.signal.android" => "com.signal.android",
        "vlc" | "org.videolan.vlc" => "org.videolan.vlc",
        "notes" | "org.onuron.notes" => "org.onuron.notes",
        "calc" | "org.onuron.calc" => "org.onuron.calc",
        "music" | "org.onuron.music" => "org.onuron.music",
        other => other,
    };

    let target_dir = app_root.join(app_id);
    let temp_dir = app_root.join(format!("{}.tmp", app_id));
    if fs::symlink_metadata(&target_dir).is_ok() || fs::symlink_metadata(&temp_dir).is_ok() {
        return Err("Destination already exists; refusing to overwrite it".into());
    }

    eprintln!("[nilpkg] WARNING: demo installer only; creates a sample launcher, not the requested application. The generated signing key is not a trusted publisher identity.");
    println!("[nilpkg] [*] Generating signed demo manifest...");
    let dummy_payload = format!("#!/bin/shnecho 'Launching {} for Onuron OS'n", app_id).into_bytes();
    let payload_sha = compute_sha256(&dummy_payload);

    let (signing_key, verifying_key) = generate_keypair();

    let mut manifest = Manifest {
        name: app_id.to_string(),
        app_id: app_id.to_string(),
        version: "1.0.0".to_string(),
        arch: if cfg!(target_arch = "x86_64") { "x86_64".into() } else { "aarch64".into() },
        min_os_version: "1.0.0".to_string(),
        description: format!("Demo launcher only (not the real application): {}", app_id),
        permissions: vec!["network".into(), "storage.read".into(), "display".into()],
        exec: format!("bin/{}", app_id),
        sha256: payload_sha,
        size_bytes: dummy_payload.len() as u64,
        signature_hex: None,
        public_key_hex: None,
        archive_sha256: None,
    };

    // Sign manifest with developer key
    sign_manifest(&mut manifest, &signing_key);

    println!("[nilpkg] [*] Verifying Ed25519 digital signature & SHA-256 payload integrity...");
    verify_manifest(&manifest, &dummy_payload)?;
    println!("[nilpkg] [✓] Ed25519 Signature Verified (Public Key: {}...)", &hex::encode(verifying_key.to_bytes())[..16]);

    fs::create_dir_all(temp_dir.join("bin")).map_err(|e| e.to_string())?;

    let manifest_json = serde_json::to_string_pretty(&manifest).map_err(|e| e.to_string())?;
    fs::write(temp_dir.join("manifest.json"), manifest_json).map_err(|e| e.to_string())?;
    fs::write(temp_dir.join(&manifest.exec), &dummy_payload).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(temp_dir.join(&manifest.exec), fs::Permissions::from_mode(0o755))
            .map_err(|e| e.to_string())?;
    }

    // Initial install only; do not remove a previously installed version.
    fs::rename(&temp_dir, &target_dir).map_err(|e| e.to_string())?;

    println!("[nilpkg] [✓ SUCCESS] Installed '{}' atomically into: {}", app_id, target_dir.display());
    println!("[nilpkg] SHA-256 Digest: {}", manifest.sha256);
    println!("[nilpkg] Permissions bound: {:?}", manifest.permissions);
    Ok(())
}

pub fn list_packages() {
    let app_root = get_app_dir();
    println!("=========================================================");
    println!("           Onuron OS Installed Packages (nilpkg)         ");
    println!("=========================================================");
    println!("App Directory: {}", app_root.display());
    println!("---------------------------------------------------------");

    let mut count = 0;
    if let Ok(entries) = fs::read_dir(&app_root) {
        for entry in entries.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let manifest_file = entry.path().join("manifest.json");
                    if manifest_file.exists() {
                        if let Ok(data) = fs::read_to_string(manifest_file) {
                            if let Ok(m) = serde_json::from_str::<Manifest>(&data) {
                                let sig_status = if m.signature_hex.is_some() { "Signature present (not reverified)" } else { "Unsigned" };
                                println!(" • {:<24} (v{}) [{}] — {}", m.app_id, m.version, sig_status, m.description);
                                count += 1;
                                continue;
                            }
                        }
                    }
                    println!(" • {}", name);
                    count += 1;
                }
            }
        }
    }

    if count == 0 {
        println!("  (No packages currently installed)");
    }
    println!("---------------------------------------------------------");
    println!("Total installed: {} packages", count);
}

pub fn remove_package(pkg_identifier: &str) -> Result<(), String> {
    validate_app_id(pkg_identifier)?;
    let app_root = get_app_dir();
    let target = app_root.join(pkg_identifier);
    if target.exists() {
        fs::remove_dir_all(&target).map_err(|e| e.to_string())?;
        println!("[nilpkg] [✓] Package '{}' successfully removed.", pkg_identifier);
        Ok(())
    } else {
        Err(format!("Package '{}' is not installed.", pkg_identifier))
    }
}

pub fn keygen_command() -> Result<(), String> {
    let key_dir = get_key_dir();
    fs::create_dir_all(&key_dir).map_err(|e| format!("Failed to create key dir: {}", e))?;

    let (signing_key, verifying_key) = generate_keypair();
    let priv_path = key_dir.join("developer.sec");
    let pub_path = key_dir.join("developer.pub");

    if pub_path.exists() { return Err("Public key already exists; refusing to replace keypair".into()); }

    // Prompt for a passphrase and encrypt the private key at rest.
    let passphrase = read_passphrase("Enter passphrase to encrypt private key: ")?;
    if passphrase.is_empty() {
        return Err("Passphrase cannot be empty".into());
    }
    let confirm = read_passphrase("Confirm passphrase: ")?;
    if passphrase != confirm {
        return Err("Passphrases do not match".into());
    }

    let record = encrypt_private_key(&signing_key.to_bytes(), &passphrase);
    write_encrypted_key(&priv_path, &record)?;

    // Write the public key (plaintext is fine for a public key).
    use std::io::Write;
    fs::OpenOptions::new().write(true).create_new(true).open(&pub_path)
        .and_then(|mut file| file.write_all(hex::encode(verifying_key.to_bytes()).as_bytes()))
        .map_err(|e| format!("Failed to write public key: {e}"))?;

    println!("=========================================================");
    println!("       Onuron OS Developer Keypair Generated             ");
    println!("=========================================================");
    println!("Private Key (encrypted): {}", priv_path.display());
    println!("Public Key:  {}", pub_path.display());
    println!("Public Key Fingerprint: {}", hex::encode(verifying_key.to_bytes()));
    println!("=========================================================");
    Ok(())
}

/// Add a publisher key to the trust store.
pub fn trust_command(keyfile: &str) -> Result<(), String> {
    let key_dir = get_key_dir();
    let trusted_dir = key_dir.join("trusted");
    fs::create_dir_all(&trusted_dir).map_err(|e| format!("Could not create trusted dir: {e}"))?;

    let contents = fs::read_to_string(keyfile)
        .map_err(|e| format!("Could not read keyfile {}: {e}", keyfile))?;
    let key_bytes = hex::decode(contents.trim())
        .map_err(|e| format!("Invalid hex in keyfile: {e}"))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "Invalid public key length {}: expected 32 bytes (64 hex chars)",
            key_bytes.len()
        ));
    }

    // Use the hex as the filename so it is unique and identifiable.
    let file_name = format!("{}.pub", contents.trim());
    let dest = trusted_dir.join(&file_name);
    if dest.exists() {
        return Err(format!("Key already trusted: {}", dest.display()));
    }
    fs::write(&dest, &contents)
        .map_err(|e| format!("Could not write trusted key: {e}"))?;
    println!("[nilpkg] Trusted publisher key: {}", dest.display());
    Ok(())
}

/// Revoke a publisher key, moving it from trusted/ to revoked/.
pub fn revoke_command(keyfile: &str) -> Result<(), String> {
    let key_dir = get_key_dir();
    let trusted_dir = key_dir.join("trusted");
    let revoked_dir = key_dir.join("revoked");
    fs::create_dir_all(&revoked_dir).map_err(|e| format!("Could not create revoked dir: {e}"))?;

    let contents = fs::read_to_string(keyfile)
        .map_err(|e| format!("Could not read keyfile {}: {e}", keyfile))?;
    let key_bytes = hex::decode(contents.trim())
        .map_err(|e| format!("Invalid hex in keyfile: {e}"))?;
    if key_bytes.len() != 32 {
        return Err(format!(
            "Invalid public key length {}: expected 32 bytes (64 hex chars)",
            key_bytes.len()
        ));
    }

    let file_name = format!("{}.pub", contents.trim());
    let src = trusted_dir.join(&file_name);
    if !src.exists() {
        return Err(format!("Key is not in the trust store: {}", src.display()));
    }
    let dest = revoked_dir.join(&file_name);
    if dest.exists() {
        return Err(format!("Key already revoked: {}", dest.display()));
    }
    fs::rename(&src, &dest)
        .map_err(|e| format!("Could not revoke key: {e}"))?;
    println!("[nilpkg] Revoked publisher key: {} -> {}", src.display(), dest.display());
    Ok(())
}

// ─── Module for basic hex encode/decode without external dependency ───────────
pub mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        let mut s = String::with_capacity(bytes.as_ref().len() * 2);
        for b in bytes.as_ref() {
            s.push_str(&format!("{:02x}", b));
        }
        s
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if !s.is_ascii() {
            return Err("Hex string must contain only ASCII hex digits".into());
        }
        if s.len() % 2 != 0 {
            return Err("Hex string must have an even length".into());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_install_requires_trust_and_never_overwrites() {
        let root = std::env::temp_dir().join(format!("nilpkg-test-{:016x}", rand::random::<u64>()));
        let source = root.join("source");
        let apps = root.join("apps");
        let keys = root.join("keys");
        fs::create_dir_all(source.join("bin")).unwrap();
        fs::create_dir_all(keys.join("trusted")).unwrap();
        let payload = b"#!/bin/shnecho hellon";
        fs::write(source.join("bin/hello"), payload).unwrap();
        let (signer, verifier) = generate_keypair();
        let mut manifest = Manifest {
            name: "Hello".into(), app_id: "org.onuron.hello".into(), version: "1.0".into(),
            arch: "x86_64".into(), min_os_version: "1.0".into(), description: "test".into(),
            permissions: vec![], exec: "bin/hello".into(), sha256: compute_sha256(payload),
            size_bytes: payload.len() as u64, signature_hex: None, public_key_hex: None, archive_sha256: None,
        };
        sign_manifest(&mut manifest, &signer);
        let write_manifest = |m: &Manifest| fs::write(source.join("manifest.json"), serde_json::to_vec(m).unwrap()).unwrap();
        write_manifest(&manifest);
        assert!(install_local(&source, &apps, &keys).is_err());
        fs::write(keys.join("trusted/test.pub"), hex::encode(verifier.to_bytes())).unwrap();
        manifest.exec = "../outside".into();
        sign_manifest(&mut manifest, &signer);
        write_manifest(&manifest);
        assert!(install_local(&source, &apps, &keys).is_err());
        manifest.exec = "bin/hello".into();
        sign_manifest(&mut manifest, &signer);
        write_manifest(&manifest);
        fs::write(source.join("bin/hello"), b"tampered").unwrap();
        assert!(install_local(&source, &apps, &keys).is_err());
        fs::write(source.join("bin/hello"), payload).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::symlink;
            let other = root.join("other");
            fs::create_dir(&other).unwrap();
            fs::write(other.join("hello"), payload).unwrap();
            fs::rename(source.join("bin"), source.join("original-bin")).unwrap();
            symlink(&other, source.join("bin")).unwrap();
            assert!(install_local(&source, &apps, &keys).is_err());
            fs::remove_file(source.join("bin")).unwrap();
            fs::rename(source.join("original-bin"), source.join("bin")).unwrap();
        }
        install_local(&source, &apps, &keys).unwrap();
        assert!(install_local(&source, &apps, &keys).is_err());
        assert_eq!(fs::read(apps.join("org.onuron.hello/bin/hello")).unwrap(), payload);
        verify_installed("org.onuron.hello", &apps, &keys).unwrap();
        fs::write(apps.join("org.onuron.hello/bin/hello"), b"changed after installation").unwrap();
        assert!(verify_installed("org.onuron.hello", &apps, &keys).is_err());
        fs::write(apps.join("org.onuron.hello/bin/hello"), payload).unwrap();
        verify_installed("org.onuron.hello", &apps, &keys).unwrap();
        fs::remove_file(keys.join("trusted/test.pub")).unwrap();
        assert!(verify_installed("org.onuron.hello", &apps, &keys).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn version_comparison_is_numeric_and_rejects_garbage() {
        use std::cmp::Ordering;
        assert_eq!(compare_versions("1.0.0", "1.0.0"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.2.0", "1.10.0"), Some(Ordering::Less));
        assert_eq!(compare_versions("2", "1.9.9"), Some(Ordering::Greater));
        assert_eq!(compare_versions("1.0", "1.0.0"), Some(Ordering::Equal));
        assert_eq!(compare_versions("1.0.0-beta", "1.0.0"), None);
        assert_eq!(compare_versions("", "1"), None);
    }

    #[test]
    fn install_rejects_incompatible_arch_os_permission_and_revoked_keys() {
        let root = std::env::temp_dir().join(format!("nilpkg-compat-{:016x}", rand::random::<u64>()));
        let apps = root.join("apps");
        let keys = root.join("keys");
        fs::create_dir_all(keys.join("trusted")).unwrap();
        fs::create_dir_all(keys.join("revoked")).unwrap();
        let (signer, verifier) = generate_keypair();
        let pub_hex = hex::encode(verifier.to_bytes());
        fs::write(keys.join("trusted/pub.pub"), &pub_hex).unwrap();

        // Wrong architecture.
        let wrong_arch = if current_arch() == "x86_64" { "aarch64" } else { "x86_64" };
        let bad_arch = root.join("bad-arch");
        write_package_full(&bad_arch, &signer, "org.onuron.app", "1.0.0", b"p", wrong_arch, CURRENT_OS_VERSION, &[]);
        assert!(install_local(&bad_arch, &apps, &keys).unwrap_err().contains("Incompatible architecture"));

        // Future OS requirement.
        let bad_os = root.join("bad-os");
        write_package_full(&bad_os, &signer, "org.onuron.app", "1.0.0", b"p", current_arch(), "99.0.0", &[]);
        assert!(install_local(&bad_os, &apps, &keys).unwrap_err().contains("requires OS"));

        // Unknown permission.
        let bad_perm = root.join("bad-perm");
        write_package_full(&bad_perm, &signer, "org.onuron.app", "1.0.0", b"p", current_arch(), CURRENT_OS_VERSION, &["network", "mind.read"]);
        assert!(install_local(&bad_perm, &apps, &keys).unwrap_err().contains("Unknown permission"));

        // Known permissions are accepted.
        let good = root.join("good");
        write_package_full(&good, &signer, "org.onuron.app", "1.0.0", b"p", current_arch(), CURRENT_OS_VERSION, &["network", "camera"]);
        install_local(&good, &apps, &keys).unwrap();

        // Revoking the publisher key breaks both verify and future upgrades.
        fs::write(keys.join("revoked/revoked.pub"), &pub_hex).unwrap();
        assert!(verify_installed("org.onuron.app", &apps, &keys).unwrap_err().contains("revoked"));
        let v2 = root.join("v2");
        write_package_full(&v2, &signer, "org.onuron.app", "2.0.0", b"p2", current_arch(), CURRENT_OS_VERSION, &[]);
        assert!(upgrade_local(&v2, &apps, &keys, false).unwrap_err().contains("revoked"));

        fs::remove_dir_all(root).unwrap();
    }

    /// Helper mirroring the on-disk package layout used by install/upgrade.
    fn write_package(dir: &Path, signer: &SigningKey, app_id: &str, version: &str, payload: &[u8]) {
        write_package_full(dir, signer, app_id, version, payload, current_arch(), CURRENT_OS_VERSION, &[]);
    }

    fn write_package_full(
        dir: &Path,
        signer: &SigningKey,
        app_id: &str,
        version: &str,
        payload: &[u8],
        arch: &str,
        min_os: &str,
        permissions: &[&str],
    ) {
        fs::create_dir_all(dir.join("bin")).unwrap();
        fs::write(dir.join("bin/app"), payload).unwrap();
        let mut manifest = Manifest {
            name: app_id.into(), app_id: app_id.into(), version: version.into(),
            arch: arch.into(), min_os_version: min_os.into(), description: "test".into(),
            permissions: permissions.iter().map(|s| s.to_string()).collect(),
            exec: "bin/app".into(), sha256: compute_sha256(payload),
            size_bytes: payload.len() as u64, signature_hex: None, public_key_hex: None, archive_sha256: None,
        };
        sign_manifest(&mut manifest, signer);
        fs::write(dir.join("manifest.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
    }

    #[test]
    fn upgrade_replaces_version_and_rollback_restores_it() {
        let root = std::env::temp_dir().join(format!("nilpkg-upg-{:016x}", rand::random::<u64>()));
        let apps = root.join("apps");
        let keys = root.join("keys");
        fs::create_dir_all(keys.join("trusted")).unwrap();
        let (signer, verifier) = generate_keypair();
        fs::write(keys.join("trusted/pub.pub"), hex::encode(verifier.to_bytes())).unwrap();

        let v1 = root.join("v1");
        let v2 = root.join("v2");
        write_package(&v1, &signer, "org.onuron.app", "1.0.0", b"v1-payload");
        write_package(&v2, &signer, "org.onuron.app", "2.0.0", b"v2-payload");

        install_local(&v1, &apps, &keys).unwrap();
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v1-payload");

        upgrade_local(&v2, &apps, &keys, false).unwrap();
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v2-payload");
        // The new version must still verify, and the backup must be retained.
        verify_installed("org.onuron.app", &apps, &keys).unwrap();
        assert!(backup_path(&apps, "org.onuron.app").exists());

        rollback_package("org.onuron.app", &apps).unwrap();
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v1-payload");
        verify_installed("org.onuron.app", &apps, &keys).unwrap();

        // A second rollback swaps back to the newer version.
        rollback_package("org.onuron.app", &apps).unwrap();
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v2-payload");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn upgrade_rejects_same_version_downgrade_and_untrusted_signer() {
        let root = std::env::temp_dir().join(format!("nilpkg-guard-{:016x}", rand::random::<u64>()));
        let apps = root.join("apps");
        let keys = root.join("keys");
        fs::create_dir_all(keys.join("trusted")).unwrap();
        let (signer, verifier) = generate_keypair();
        fs::write(keys.join("trusted/pub.pub"), hex::encode(verifier.to_bytes())).unwrap();

        let v1 = root.join("v1");
        let v2 = root.join("v2");
        let v0 = root.join("v0");
        write_package(&v1, &signer, "org.onuron.app", "1.0.0", b"v1");
        write_package(&v2, &signer, "org.onuron.app", "2.0.0", b"v2");
        write_package(&v0, &signer, "org.onuron.app", "0.9.0", b"v0");
        install_local(&v1, &apps, &keys).unwrap();

        assert!(upgrade_local(&v1, &apps, &keys, false).unwrap_err().contains("already installed"));
        assert!(upgrade_local(&v0, &apps, &keys, false).unwrap_err().contains("downgrade"));
        // Forcing allows the downgrade but never skips verification.
        upgrade_local(&v0, &apps, &keys, true).unwrap();
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v0");

        // An untrusted publisher is rejected for upgrades too.
        let (other, _) = generate_keypair();
        let evil = root.join("evil");
        write_package(&evil, &other, "org.onuron.app", "9.9.9", b"evil");
        assert!(upgrade_local(&evil, &apps, &keys, false).is_err());
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v0");

        // Upgrading something that is not installed must not create it.
        let fresh = root.join("fresh");
        write_package(&fresh, &signer, "org.onuron.missing", "1.0.0", b"new");
        assert!(upgrade_local(&fresh, &apps, &keys, false).unwrap_err().contains("not installed"));

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn interrupted_upgrade_is_recovered_on_next_operation() {
        let root = std::env::temp_dir().join(format!("nilpkg-recover-{:016x}", rand::random::<u64>()));
        let apps = root.join("apps");
        let keys = root.join("keys");
        fs::create_dir_all(keys.join("trusted")).unwrap();
        let (signer, verifier) = generate_keypair();
        fs::write(keys.join("trusted/pub.pub"), hex::encode(verifier.to_bytes())).unwrap();

        let v1 = root.join("v1");
        write_package(&v1, &signer, "org.onuron.app", "1.0.0", b"v1-payload");
        install_local(&v1, &apps, &keys).unwrap();

        // Simulate a crash after the old version was moved aside: journal on
        // disk, target missing, backup holding the only copy.
        let target = apps.join("org.onuron.app");
        fs::rename(&target, backup_path(&apps, "org.onuron.app")).unwrap();
        fs::write(journal_path(&apps, "org.onuron.app"), b"{}").unwrap();

        // Also leave an orphaned staging directory behind.
        let orphan = apps.join(".nilpkg-deadbeef");
        fs::create_dir_all(orphan.join("bin")).unwrap();

        recover_pending(&apps).unwrap();
        assert_eq!(fs::read(apps.join("org.onuron.app/bin/app")).unwrap(), b"v1-payload");
        assert!(!journal_path(&apps, "org.onuron.app").exists());
        assert!(!orphan.exists(), "orphaned staging directory should be cleaned up");
        verify_installed("org.onuron.app", &apps, &keys).unwrap();

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn failed_install_leaves_no_staging_or_partial_package() {
        let root = std::env::temp_dir().join(format!("nilpkg-stage-{:016x}", rand::random::<u64>()));
        let apps = root.join("apps");
        let keys = root.join("keys");
        fs::create_dir_all(keys.join("trusted")).unwrap();
        let (signer, verifier) = generate_keypair();
        fs::write(keys.join("trusted/pub.pub"), hex::encode(verifier.to_bytes())).unwrap();

        let pkg = root.join("pkg");
        write_package(&pkg, &signer, "org.onuron.app", "1.0.0", b"payload");
        install_local(&pkg, &apps, &keys).unwrap();

        // Second install of the same ID must fail without leaving .nilpkg-* junk.
        assert!(install_local(&pkg, &apps, &keys).is_err());
        let leftovers: Vec<String> = fs::read_dir(&apps).unwrap().flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with(".nilpkg-") && !n.starts_with(".nilpkg-backup-") && !n.starts_with(".nilpkg-journal-"))
            .collect();
        assert!(leftovers.is_empty(), "staging leftovers: {leftovers:?}");

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn package_ids_cannot_escape_root_or_inject_shell_commands() {
        for id in ["", ".", "..", "../victim", "/tmp/victim", "C:victim", "a/b", "a b",
            "a..b", "a.", "a'$(id)", "a\nb", "a\tb", "CON", "nul.txt", "COM1", "lpt9.txt", "é"] {
            assert!(validate_app_id(id).is_err(), "accepted {id:?}");
        }
        assert!(validate_app_id(&"a".repeat(129)).is_err());
        for id in ["notes", "org.onuron.notes", "my-app_2", "com.signal.android"] {
            assert!(validate_app_id(id).is_ok(), "rejected {id:?}");
        }
    }

    #[test]
    fn hex_decoder_rejects_malformed_input_without_panicking() {
        for value in ["a", "gg", "a€", "💥", "éé"] {
            assert!(hex::decode(value).is_err());
        }
        assert_eq!(hex::decode("00aAfF").unwrap(), vec![0, 170, 255]);
    }

    #[test]
    fn test_compute_sha256() {
        let data = b"hello onuron os";
        let hash = compute_sha256(data);
        assert_eq!(hash.len(), 64);
        // Known SHA-256 for "hello onuron os"
        let expected = "ef792bac3fb790992f7b14e8ad265f91cd33274408f85f1f7b8e5bd9fb540a2d";
        assert_eq!(hash, expected);
    }

    #[test]
    fn test_ed25519_sign_and_verify_valid() {
        let (signing_key, _verifying_key) = generate_keypair();
        let payload = b"binary_app_executable_code";
        let payload_sha = compute_sha256(payload);

        let mut manifest = Manifest {
            name: "org.onuron.demo".into(),
            app_id: "org.onuron.demo".into(),
            version: "1.0.0".into(),
            arch: "aarch64".into(),
            min_os_version: "1.0.0".into(),
            description: "Test Application".into(),
            permissions: vec!["storage.read".into()],
            exec: "bin/demo".into(),
            sha256: payload_sha,
            size_bytes: payload.len() as u64,
            signature_hex: None,
            public_key_hex: None,
            archive_sha256: None,
        };

        sign_manifest(&mut manifest, &signing_key);
        assert!(manifest.signature_hex.is_some());
        assert!(manifest.public_key_hex.is_some());

        let verify_result = verify_manifest(&manifest, payload);
        assert!(verify_result.is_ok(), "Verification should succeed for valid signature");

        // Even a correctly signed manifest must accurately describe payload size.
        manifest.size_bytes += 1;
        sign_manifest(&mut manifest, &signing_key);
        assert!(verify_manifest(&manifest, payload).unwrap_err().contains("size_bytes"));
        manifest.size_bytes -= 1;
        sign_manifest(&mut manifest, &signing_key);
        manifest.permissions.push("storage.write".into());
        assert!(verify_manifest(&manifest, payload).is_err());
    }

    #[test]
    fn test_tampered_payload_rejected() {
        let (signing_key, _) = generate_keypair();
        let payload = b"original_app_payload";
        let mut manifest = Manifest {
            name: "org.onuron.tamper".into(),
            app_id: "org.onuron.tamper".into(),
            version: "1.0.0".into(),
            arch: "x86_64".into(),
            min_os_version: "1.0.0".into(),
            description: "Tamper Test".into(),
            permissions: vec![],
            exec: "bin/tamper".into(),
            sha256: compute_sha256(payload),
            size_bytes: payload.len() as u64,
            signature_hex: None,
            public_key_hex: None,
            archive_sha256: None,
        };

        sign_manifest(&mut manifest, &signing_key);

        // Tampered payload
        let tampered_payload = b"malicious_injected_code";
        let verify_result = verify_manifest(&manifest, tampered_payload);
        assert!(verify_result.is_err(), "Verification must fail when payload is tampered");
    }

    #[test]
    fn test_unsigned_manifest_rejected() {
        let payload = b"unsigned_code";
        let manifest = Manifest {
            name: "org.onuron.unsigned".into(),
            app_id: "org.onuron.unsigned".into(),
            version: "1.0.0".into(),
            arch: "x86_64".into(),
            min_os_version: "1.0.0".into(),
            description: "Unsigned Test".into(),
            permissions: vec![],
            exec: "bin/unsigned".into(),
            sha256: compute_sha256(payload),
            size_bytes: payload.len() as u64,
            signature_hex: None,
            public_key_hex: None,
            archive_sha256: None,
        };

        let verify_result = verify_manifest(&manifest, payload);
        assert!(verify_result.is_err(), "Verification must fail for unsigned manifest");
    }

    #[test]
    fn test_pack_package_and_archive_roundtrip() {
        let (signer, _verifier) = generate_keypair();
        let temp = std::env::temp_dir().join(format!("nilpkg-packtest-{:016x}", rand::random::<u64>()));
        fs::create_dir_all(&temp).unwrap();
        let bin_path = temp.join("dummy_bin");
        fs::write(&bin_path, b"test executable binary payload").unwrap();

        // 1. Pack into directory
        let dir_out = temp.join("packed_dir");
        let manifest = pack_package(
            "org.onuron.packtest",
            "Pack Test App",
            "1.0.0",
            "Testing package packing",
            vec!["display".to_string()],
            &bin_path,
            &dir_out,
            &signer,
        ).expect("pack_package to dir");

        assert_eq!(manifest.app_id, "org.onuron.packtest");
        assert!(manifest.signature_hex.is_some());
        assert!(manifest.public_key_hex.is_some());

        // Read and verify packed directory
        let (read_manifest, payload) = read_package(&dir_out).expect("read_package");
        assert_eq!(read_manifest.app_id, "org.onuron.packtest");
        assert_eq!(payload, b"test executable binary payload");

        // 2. Pack into .nilax archive
        let archive_out = temp.join("test_app.nilax");
        let archive_manifest = pack_package(
            "org.onuron.archtest",
            "Archive Test App",
            "1.0.0",
            "Testing archive packing",
            vec!["display".to_string()],
            &bin_path,
            &archive_out,
            &signer,
        ).expect("pack_package to .nilax archive");

        assert_eq!(archive_manifest.app_id, "org.onuron.archtest");
        assert!(archive_out.is_file());

        // Extract and verify archive
        let extract_dest = temp.join("extracted_archive");
        extract::extract_archive(&archive_out, &extract_dest).expect("extract_archive");
        let (read_arc_manifest, arc_payload) = read_package(&extract_dest).expect("read extracted archive");
        assert_eq!(read_arc_manifest.app_id, "org.onuron.archtest");
        assert_eq!(arc_payload, b"test executable binary payload");

        let _ = fs::remove_dir_all(&temp);
    }
}
