// services/nilupd/src/lib.rs — Onuron OS A/B system-update library.
//
// `nilupd` is the system-update counterpart to `nilpkg`. It verifies a signed
// update image against an explicitly trusted publisher key *before* touching
// the installation, then applies it with A/B slot semantics and the same
// crash-safe write pattern used by `pkg/nilpkg/src/durable.rs`.
//
// ── Trust model (C3, scoped) ────────────────────────────────────────────────
//
// This crate implements the *update-image* trust anchor:
//
//   1. The image payload must match the SHA-256 recorded in the manifest.
//   2. The manifest must carry a valid Ed25519 signature over its canonical
//      bytes (all fields except `signature_hex` / `public_key_hex`).
//   3. That publisher key must be present in the on-disk trust store
//      (`<key_dir>/trusted/*.pub`) and not revoked (`<key_dir>/revoked/`),
//      reusing nilpkg's trust machinery.
//   4. The manifest declares the hash of the *currently running* image
//      (`current_image_sha256`) and the new image hash. `apply_update` refuses
//      unless the running hash matches, so a stale update cannot be replayed
//      onto a different system image.
//
// WHAT IS NOT IMPLEMENTED: this is not measured boot and there is no TPM
// attestation / PCR measurement / verified-boot chain. The "running image
// hash" is read from a local marker file (`running.json`); a real boot chain
// would have the bootloader (or a TPM quote) provide it. Full measured boot is
// a documented follow-up, not a claimed feature. See docs/improvement-plan.md
// section C3: "Full measured boot / TPM attestation is a planned follow-up."

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use nilprotocol::{Frame, MessageType};

use nilpkg::durable::{sync_dir, sync_tree, write_file_atomic};
use nilpkg::lockfile::PackageLock;
// Reuse nilpkg's proven signing/trust machinery rather than duplicating it.
use nilpkg::{compute_sha256, hex, is_revoked, trusted_publisher, verify_signature};

/// The two A/B slots. Exactly one is active at a time.
pub const SLOTS: [&str; 2] = ["A", "B"];

const ACTIVE_FILE: &str = "active";
const RUNNING_FILE: &str = "running.json";
const PENDING_FILE: &str = "pending.json";
const SLOT_DIR: &str = "slots";
const SLOT_META_FILE: &str = "meta.json";
const IMAGE_NAME: &str = "image";
const STAGING_NAME: &str = ".image.staging";
/// Manifests are tiny; anything larger is rejected before parsing.
const MAX_MANIFEST_BYTES: u64 = 64 * 1024;
/// Conservative cap on an update image so a malformed manifest cannot make the
/// updater copy an unbounded file. A real system image is well under this.
const MAX_IMAGE_BYTES: u64 = 2 * 1024 * 1024 * 1024;

/// The signed description of a system update.
///
/// Every field except `name` is `#[serde(default)]` so that older or newer
/// manifests remain parseable instead of failing the whole update.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct UpdateManifest {
    pub name: String,
    /// Human-readable version of the new image.
    #[serde(default)]
    pub version: String,
    /// Slot the new image must be written to. Must equal the currently inactive
    /// slot ("A" or "B").
    #[serde(default)]
    pub target_slot: String,
    /// SHA-256 of the image the boot chain is *currently* running. This is the
    /// update trust anchor: apply refuses when it does not match.
    #[serde(default)]
    pub current_image_sha256: String,
    /// SHA-256 of the new image payload.
    #[serde(default)]
    pub image_sha256: String,
    /// Size of the new image in bytes.
    #[serde(default)]
    pub image_size: u64,
    /// Hardware/target device profile identifier (e.g. "oneplus-fajita", "qemu-x86_64").
    #[serde(default)]
    pub target_device: String,
    /// Monotonically increasing security rollback index (prevents downgrade attacks).
    #[serde(default)]
    pub rollback_index: u32,
    /// Hex-encoded Ed25519 signature over the canonical manifest (all fields
    /// except the two signature/key fields).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signature_hex: Option<String>,
    /// Hex-encoded Ed25519 public key of the publisher.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub public_key_hex: Option<String>,
}

impl UpdateManifest {
    /// The exact byte string the signature covers: this manifest with the
    /// signature and key fields cleared.
    pub fn canonical_bytes(&self) -> Result<Vec<u8>, String> {
        let mut canonical = self.clone();
        canonical.signature_hex = None;
        canonical.public_key_hex = None;
        serde_json::to_vec(&canonical).map_err(|e| format!("Could not serialize manifest: {e}"))
    }
}

/// Sign an update manifest in place with an Ed25519 key (mirrors
/// `nilpkg::sign_manifest`).
pub fn sign_update(manifest: &mut UpdateManifest, signing_key: &SigningKey) {
    manifest.signature_hex = None;
    manifest.public_key_hex = None;
    let bytes = manifest
        .canonical_bytes()
        .expect("serializing an UpdateManifest is infallible");
    let sig = signing_key.sign(&bytes);
    manifest.signature_hex = Some(hex::encode(sig.to_bytes()));
    manifest.public_key_hex = Some(hex::encode(signing_key.verifying_key().to_bytes()));
}

/// Small pointer recorded next to the image in a committed slot.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct SlotMeta {
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub image_sha256: String,
    #[serde(default)]
    pub target_device: String,
    #[serde(default)]
    pub rollback_index: u32,
}

/// The image the boot chain is currently running. In a full measured-boot
/// design this would be produced by the bootloader / TPM; here it is the
/// updater's record and the anchor the next update is checked against.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub struct RunningState {
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub image_sha256: String,
    #[serde(default)]
    pub target_device: String,
    #[serde(default)]
    pub rollback_index: u32,
}

/// Journal written after the new image is staged but before the slot flip, so
/// an interrupted update is recoverable.
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
struct PendingUpdate {
    #[serde(default)]
    from_slot: String,
    #[serde(default)]
    target_slot: String,
    #[serde(default)]
    version: String,
    #[serde(default)]
    image_sha256: String,
}

/// State of an individual boot slot according to the boot control system.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BootSlotInfo {
    pub slot: String,
    pub is_active: bool,
    pub is_successful: bool,
    pub tries_remaining: u8,
    pub is_bootable: bool,
}

/// Boot control status view.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BootControlStatus {
    pub current_slot: String,
    pub fallback_slot: String,
    pub is_simulated: bool,
    pub backend_name: String,
    pub slots: Vec<BootSlotInfo>,
}

/// Abstraction for platform boot-control operations.
pub trait BootControl: Send + Sync {
    fn get_current_slot(&self) -> Result<String, String>;
    fn mark_boot_successful(&self) -> Result<(), String>;
    fn set_active_boot_slot(&self, slot: &str) -> Result<(), String>;
    fn record_boot_attempt(&self) -> Result<String, String>;
    fn get_status(&self) -> Result<BootControlStatus, String>;
    fn is_simulated(&self) -> bool;
}

const BOOT_CONTROL_FILE: &str = "boot_control.json";
pub const DEFAULT_MAX_BOOT_TRIES: u8 = 3;

#[derive(Serialize, Deserialize, Debug, Clone)]
struct BootControlState {
    current_slot: String,
    fallback_slot: String,
    slots: std::collections::HashMap<String, BootSlotInfo>,
}

/// Simulated boot control implementation for testing and QEMU development.
pub struct SimulatedBootControl {
    path: std::path::PathBuf,
    install_root: std::path::PathBuf,
}

impl SimulatedBootControl {
    pub fn new(install_root: &std::path::Path) -> Self {
        Self {
            path: install_root.join(BOOT_CONTROL_FILE),
            install_root: install_root.to_path_buf(),
        }
    }

    fn read_or_init_state(&self) -> Result<BootControlState, String> {
        if self.path.exists() {
            let data = std::fs::read(&self.path).map_err(|e| e.to_string())?;
            serde_json::from_slice(&data).map_err(|e| format!("Invalid boot_control.json: {e}"))
        } else {
            let active = read_active(&self.install_root).unwrap_or_else(|_| "A".to_string());
            let other = if active == "A" { "B" } else { "A" };
            let mut slots = std::collections::HashMap::new();
            slots.insert(
                active.clone(),
                BootSlotInfo {
                    slot: active.clone(),
                    is_active: true,
                    is_successful: true,
                    tries_remaining: DEFAULT_MAX_BOOT_TRIES,
                    is_bootable: true,
                },
            );
            slots.insert(
                other.to_string(),
                BootSlotInfo {
                    slot: other.to_string(),
                    is_active: false,
                    is_successful: false,
                    tries_remaining: DEFAULT_MAX_BOOT_TRIES,
                    is_bootable: false,
                },
            );
            let state = BootControlState {
                current_slot: active.clone(),
                fallback_slot: active,
                slots,
            };
            self.save_state(&state)?;
            Ok(state)
        }
    }

    fn save_state(&self, state: &BootControlState) -> Result<(), String> {
        let bytes = serde_json::to_vec_pretty(state).map_err(|e| e.to_string())?;
        write_file_atomic(&self.path, &bytes)?;
        sync_dir(&self.install_root)
    }
}

impl BootControl for SimulatedBootControl {
    fn get_current_slot(&self) -> Result<String, String> {
        let state = self.read_or_init_state()?;
        Ok(state.current_slot)
    }

    fn mark_boot_successful(&self) -> Result<(), String> {
        let mut state = self.read_or_init_state()?;
        let curr = state.current_slot.clone();
        if let Some(info) = state.slots.get_mut(&curr) {
            info.is_successful = true;
            info.tries_remaining = DEFAULT_MAX_BOOT_TRIES;
            info.is_bootable = true;
        }
        state.fallback_slot = curr;
        self.save_state(&state)
    }

    fn set_active_boot_slot(&self, slot: &str) -> Result<(), String> {
        if !SLOTS.contains(&slot) {
            return Err(format!("Unknown slot: {slot}"));
        }
        let mut state = self.read_or_init_state()?;
        let prev = state.current_slot.clone();
        state.fallback_slot = prev;
        state.current_slot = slot.to_string();

        for (name, info) in state.slots.iter_mut() {
            if name == slot {
                info.is_active = true;
                info.is_successful = false;
                info.tries_remaining = DEFAULT_MAX_BOOT_TRIES;
                info.is_bootable = true;
            } else {
                info.is_active = false;
            }
        }
        self.save_state(&state)
    }

    fn record_boot_attempt(&self) -> Result<String, String> {
        let mut state = self.read_or_init_state()?;
        let curr = state.current_slot.clone();
        let fallback = state.fallback_slot.clone();

        let need_rollback = if let Some(info) = state.slots.get_mut(&curr) {
            if !info.is_successful {
                if info.tries_remaining > 0 {
                    info.tries_remaining -= 1;
                }
                if info.tries_remaining == 0 {
                    info.is_bootable = false;
                    info.is_active = false;
                    true
                } else {
                    false
                }
            } else {
                false
            }
        } else {
            false
        };

        if need_rollback {
            eprintln!(
                "[nilupd:boot_control] Candidate slot {curr} failed boot limit! Automatic rollback to {fallback}"
            );
            state.current_slot = fallback.clone();
            if let Some(fb_info) = state.slots.get_mut(&fallback) {
                fb_info.is_active = true;
            }
            self.save_state(&state)?;
            // Also synchronize active pointer
            write_file_atomic(&self.install_root.join(ACTIVE_FILE), fallback.as_bytes())?;
            return Err(format!(
                "Candidate slot {curr} boot attempts exhausted (0 remaining). Rolled back to slot {fallback}"
            ));
        }

        self.save_state(&state)?;
        Ok(state.current_slot)
    }

    fn get_status(&self) -> Result<BootControlStatus, String> {
        let state = self.read_or_init_state()?;
        let mut slots_vec = Vec::new();
        for slot in SLOTS {
            if let Some(info) = state.slots.get(slot) {
                slots_vec.push(info.clone());
            }
        }
        Ok(BootControlStatus {
            current_slot: state.current_slot,
            fallback_slot: state.fallback_slot,
            is_simulated: true,
            backend_name: "SimulatedBootControl (QEMU / Test Backend)".into(),
            slots: slots_vec,
        })
    }

    fn is_simulated(&self) -> bool {
        true
    }
}

/// Per-slot view for `nilupd status`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SlotStatus {
    pub slot: String,
    pub active: bool,
    pub version: Option<String>,
    pub image_sha256: Option<String>,
}

/// Whole-install view for `nilupd status`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Status {
    pub active_slot: String,
    pub running_image_sha256: String,
    pub pending: bool,
    pub slots: Vec<SlotStatus>,
    #[serde(default)]
    pub boot_control: Option<BootControlStatus>,
}

/// Default install root for the system image. Development hosts keep it under
/// the user profile; the device uses a fixed path.
pub fn get_install_root() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        let base = std::env::var_os("USERPROFILE")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| std::path::PathBuf::from("."));
        base.join(".onuron").join("system")
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::path::PathBuf::from("/data/onuron/system")
    }
}

// ── Paths and small readers ─────────────────────────────────────────────────

fn slot_dir(install_root: &std::path::Path, slot: &str) -> std::path::PathBuf {
    install_root.join(SLOT_DIR).join(slot)
}

fn slot_image_path(install_root: &std::path::Path, slot: &str) -> std::path::PathBuf {
    slot_dir(install_root, slot).join(IMAGE_NAME)
}

fn staging_path(install_root: &std::path::Path, slot: &str) -> std::path::PathBuf {
    slot_dir(install_root, slot).join(STAGING_NAME)
}

fn slot_meta_path(install_root: &std::path::Path, slot: &str) -> std::path::PathBuf {
    slot_dir(install_root, slot).join(SLOT_META_FILE)
}

/// Return the slot that is not `active`.
fn other_slot(active: &str) -> Result<&'static str, String> {
    match active {
        "A" => Ok("B"),
        "B" => Ok("A"),
        other => Err(format!("Invalid active slot marker '{other}'")),
    }
}

fn read_active(install_root: &std::path::Path) -> Result<String, String> {
    let path = install_root.join(ACTIVE_FILE);
    let raw = std::fs::read_to_string(&path)
        .map_err(|e| format!("Install root is not initialized ({}): {e}", path.display()))?;
    let slot = raw.trim().to_string();
    if !SLOTS.contains(&slot.as_str()) {
        return Err(format!("Invalid active slot marker '{slot}' in {}", path.display()));
    }
    Ok(slot)
}

fn read_running(install_root: &std::path::Path) -> Result<RunningState, String> {
    let path = install_root.join(RUNNING_FILE);
    let data = std::fs::read(&path)
        .map_err(|e| format!("Could not read running image state ({}): {e}", path.display()))?;
    serde_json::from_slice(&data)
        .map_err(|e| format!("Invalid running image state ({}): {e}", path.display()))
}

fn read_slot_meta(install_root: &std::path::Path, slot: &str) -> Option<SlotMeta> {
    let data = std::fs::read(slot_meta_path(install_root, slot)).ok()?;
    serde_json::from_slice(&data).ok()
}

/// Ensure `path` is a regular file and never a symlink or directory.
fn regular_file(path: &std::path::Path) -> Result<(), String> {
    let meta = std::fs::symlink_metadata(path)
        .map_err(|e| format!("Could not stat {}: {e}", path.display()))?;
    if !meta.file_type().is_file() {
        return Err(format!("Not a regular file: {}", path.display()));
    }
    Ok(())
}

/// Stream a file through SHA-256 so a large image is never fully buffered.
fn hash_file(path: &std::path::Path) -> Result<(String, u64), String> {
    let mut file = std::fs::File::open(path)
        .map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut total: u64 = 0;
    loop {
        let n = std::io::Read::read(&mut file, &mut buf)
            .map_err(|e| format!("Could not read {}: {e}", path.display()))?;
        if n == 0 {
            break;
        }
        total += n as u64;
        if total > MAX_IMAGE_BYTES {
            return Err("Update image exceeds the size limit".into());
        }
        hasher.update(&buf[..n]);
    }
    Ok((format!("{:064x}", hasher.finalize()), total))
}

/// Durably copy `src` to `dest` by writing a temp file, fsyncing it, renaming
/// it into place and fsyncing the containing directory. Streaming, so it never
/// holds a whole system image in memory.
fn copy_file_atomic(src: &std::path::Path, dest: &std::path::Path) -> Result<(), String> {
    let parent = dest
        .parent()
        .ok_or_else(|| format!("{} has no parent directory", dest.display()))?;
    std::fs::create_dir_all(parent)
        .map_err(|e| format!("Could not create {}: {e}", parent.display()))?;
    let file_name = dest
        .file_name()
        .ok_or_else(|| format!("{} has no file name", dest.display()))?
        .to_string_lossy()
        .into_owned();
    let tmp = parent.join(format!(".{file_name}.tmp.{}", std::process::id()));

    {
        let mut reader =
            std::fs::File::open(src).map_err(|e| format!("Could not open {}: {e}", src.display()))?;
        let mut writer =
            std::fs::File::create(&tmp).map_err(|e| format!("Could not create {}: {e}", tmp.display()))?;
        std::io::copy(&mut reader, &mut writer)
            .map_err(|e| format!("Could not copy {} -> {}: {e}", src.display(), tmp.display()))?;
        writer
            .sync_all()
            .map_err(|e| format!("Could not fsync {}: {e}", tmp.display()))?;
    }

    if let Err(e) = std::fs::rename(&tmp, dest) {
        let _ = std::fs::remove_file(&tmp);
        return Err(format!("Could not replace {}: {e}", dest.display()));
    }
    sync_dir(parent)
}

// ── C3: update-image verification ───────────────────────────────────────────

/// Build the throwaway `nilpkg::Manifest` used only to reach nilpkg's
/// `trusted_publisher` check. Only `public_key_hex` is consulted.
fn publisher_manifest(public_key_hex: &str) -> nilpkg::Manifest {
    nilpkg::Manifest {
        name: "onuron-system-update".into(),
        app_id: "org.onuron.system".into(),
        version: String::new(),
        arch: nilpkg::current_arch().into(),
        min_os_version: String::new(),
        description: String::new(),
        permissions: vec![],
        exec: "bin/image".into(),
        sha256: String::new(),
        size_bytes: 0,
        signature_hex: None,
        public_key_hex: Some(public_key_hex.to_string()),
        archive_sha256: None,
    }
}

/// Verify the update image in `update_dir` *before anything is written to the
/// installation*.
///
/// Checks, in order:
///   1. `update.json` and `image` exist as regular files (no symlinks).
///   2. The image size and SHA-256 match the manifest.
///   3. The Ed25519 signature over the canonical manifest is valid.
///   4. The publisher key is trusted (and not revoked).
///   5. `target_slot` names a valid slot.
///
/// NOTE: this is the update-image trust anchor only. There is no TPM
/// attestation / measured boot here; see the module header. Full measured boot
/// is a documented planned follow-up.
pub fn verify_image(
    update_dir: &std::path::Path,
    key_dir: &std::path::Path,
) -> Result<UpdateManifest, String> {
    let dir_meta = std::fs::symlink_metadata(update_dir)
        .map_err(|e| format!("Update directory {} is not readable: {e}", update_dir.display()))?;
    if !dir_meta.file_type().is_dir() {
        return Err("Update directory must be a directory (not a symlink or file)".into());
    }

    let manifest_path = update_dir.join("update.json");
    regular_file(&manifest_path)?;
    if std::fs::metadata(&manifest_path).map_err(|e| e.to_string())?.len() > MAX_MANIFEST_BYTES {
        return Err("Update manifest is too large".into());
    }
    let manifest: UpdateManifest = serde_json::from_slice(
        &std::fs::read(&manifest_path).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("Invalid update manifest: {e}"))?;

    if !SLOTS.contains(&manifest.target_slot.as_str()) {
        return Err(format!(
            "Update targets unknown slot '{}' (expected A or B)",
            manifest.target_slot
        ));
    }

    let image_path = update_dir.join("image");
    regular_file(&image_path)?;

    // 1. Payload size and SHA-256 integrity (streamed).
    let (actual_sha, actual_size) = hash_file(&image_path)?;
    if actual_size != manifest.image_size {
        return Err(format!(
            "Image size mismatch: manifest says {} bytes, payload is {actual_size} bytes",
            manifest.image_size
        ));
    }
    if manifest.image_sha256 != actual_sha {
        return Err(format!(
            "Image SHA-256 mismatch: expected {}, got {actual_sha}",
            manifest.image_sha256
        ));
    }

    // 2. Signature presence and Ed25519 verification over canonical bytes.
    let sig_hex = manifest
        .signature_hex
        .as_ref()
        .ok_or("Update is unsigned (missing signature_hex)")?;
    let pub_hex = manifest
        .public_key_hex
        .as_ref()
        .ok_or("Update is missing public_key_hex")?;

    let sig_bytes = hex::decode(sig_hex).map_err(|e| format!("Invalid signature hex: {e}"))?;
    let pub_bytes = hex::decode(pub_hex).map_err(|e| format!("Invalid public key hex: {e}"))?;
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
        .map_err(|e| format!("Invalid Ed25519 public key bytes: {e}"))?;

    let canonical = manifest.canonical_bytes()?;
    if !verify_signature(&verifying_key, &canonical, &signature) {
        return Err("Ed25519 signature verification failed; update may be tampered".into());
    }

    // 3. Explicit local trust: the embedded key proves nothing on its own.
    if is_revoked(key_dir, &pub_bytes) {
        return Err("Publisher key has been revoked".into());
    }
    trusted_publisher(key_dir, &publisher_manifest(pub_hex))?;

    Ok(manifest)
}

// ── A/B slot application ────────────────────────────────────────────────────

/// Prepare an install root with a single active slot ("A") holding
/// `initial_image`. Used for first-time provisioning and by tests.
pub fn initialize_install_root(
    install_root: &std::path::Path,
    initial_image: &[u8],
) -> Result<(), String> {
    std::fs::create_dir_all(install_root)
        .map_err(|e| format!("Could not create install root: {e}"))?;
    if install_root.join(ACTIVE_FILE).exists() {
        return Err("Install root already initialized".into());
    }
    std::fs::create_dir_all(slot_dir(install_root, "A")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(slot_dir(install_root, "B")).map_err(|e| e.to_string())?;

    let hash = compute_sha256(initial_image);
    write_file_atomic(&slot_image_path(install_root, "A"), initial_image)?;
    let meta = SlotMeta {
        version: "0.0.0".into(),
        image_sha256: hash.clone(),
        target_device: String::new(),
        rollback_index: 0,
    };
    write_file_atomic(
        &slot_meta_path(install_root, "A"),
        &serde_json::to_vec(&meta).map_err(|e| e.to_string())?,
    )?;
    let running = RunningState {
        slot: "A".into(),
        image_sha256: hash,
        target_device: String::new(),
        rollback_index: 0,
    };
    write_file_atomic(
        &install_root.join(RUNNING_FILE),
        &serde_json::to_vec(&running).map_err(|e| e.to_string())?,
    )?;
    write_file_atomic(&install_root.join(ACTIVE_FILE), b"A")?;
    let boot_ctrl = SimulatedBootControl::new(install_root);
    let _ = boot_ctrl.mark_boot_successful();
    sync_dir(install_root)
}

/// Verify an update and confirm it belongs to this installation: it must target
/// the inactive slot and its expected current-image hash must match the running
/// image. This is the C3 boot-chain anchor check.
fn prepare_update(
    update_dir: &std::path::Path,
    install_root: &std::path::Path,
    key_dir: &std::path::Path,
) -> Result<UpdateManifest, String> {
    let manifest = verify_image(update_dir, key_dir)?;

    let active = read_active(install_root)?;
    let inactive = other_slot(&active)?;
    if manifest.target_slot != inactive {
        return Err(format!(
            "Update targets slot {} but the inactive slot is {inactive}",
            manifest.target_slot
        ));
    }

    let running = read_running(install_root)?;
    if manifest.current_image_sha256 != running.image_sha256 {
        return Err(format!(
            "Running image hash does not match the update's expected current image \
             (running {}, manifest expects {}); refusing to apply",
            running.image_sha256, manifest.current_image_sha256
        ));
    }

    // Anti-rollback enforcement (Section 32.1)
    if manifest.rollback_index < running.rollback_index {
        return Err(format!(
            "Anti-rollback violation: manifest rollback_index ({}) is less than running rollback_index ({}); downgrade rejected",
            manifest.rollback_index, running.rollback_index
        ));
    }

    // Target device match enforcement
    if !manifest.target_device.is_empty()
        && !running.target_device.is_empty()
        && manifest.target_device != running.target_device
    {
        return Err(format!(
            "Target device mismatch: update is built for '{}', but running device is '{}'",
            manifest.target_device, running.target_device
        ));
    }

    Ok(manifest)
}

/// Stage a verified image into the inactive slot's staging file and write the
/// pending journal. The previously committed `image` in that slot is left
/// untouched, so an interruption before the flip loses nothing.
fn stage_prepared(
    update_dir: &std::path::Path,
    install_root: &std::path::Path,
    manifest: &UpdateManifest,
) -> Result<(), String> {
    let active = read_active(install_root)?;
    let target = manifest.target_slot.clone();
    let staging = staging_path(install_root, &target);

    // Copy the image so the source can disappear after verification.
    copy_file_atomic(&update_dir.join("image"), &staging)?;
    // Defence in depth: re-hash the staged copy before it can ever go live.
    let (staged_sha, _) = hash_file(&staging)?;
    if staged_sha != manifest.image_sha256 {
        let _ = std::fs::remove_file(&staging);
        return Err("Staged image failed re-verification; aborting".into());
    }
    sync_tree(&slot_dir(install_root, &target))?;

    let pending = PendingUpdate {
        from_slot: active,
        target_slot: target,
        version: manifest.version.clone(),
        image_sha256: manifest.image_sha256.clone(),
    };
    write_file_atomic(
        &install_root.join(PENDING_FILE),
        &serde_json::to_vec(&pending).map_err(|e| e.to_string())?,
    )?;
    sync_dir(install_root)
}

/// Commit a staged update: rename the staging file over the slot image, record
/// slot metadata, then flip the active-slot pointer with an atomic rename and
/// clear the journal. The previously active slot is never touched.
fn commit_prepared(
    install_root: &std::path::Path,
    manifest: &UpdateManifest,
) -> Result<(), String> {
    let target = manifest.target_slot.clone();
    let staging = staging_path(install_root, &target);
    let image = slot_image_path(install_root, &target);

    if !staging.exists() {
        return Err("No staged image found to commit".into());
    }

    // 1. Make the staged image live in the inactive slot.
    std::fs::rename(&staging, &image)
        .map_err(|e| format!("Could not move staged image into slot {target}: {e}"))?;

    // Read-back verification (Section 32.2 & 32.4)
    let (readback_sha, _) = hash_file(&image)?;
    if readback_sha != manifest.image_sha256 {
        return Err(format!(
            "Destination image read-back verification failed: expected {}, got {readback_sha}",
            manifest.image_sha256
        ));
    }

    let meta = SlotMeta {
        version: manifest.version.clone(),
        image_sha256: manifest.image_sha256.clone(),
        target_device: manifest.target_device.clone(),
        rollback_index: manifest.rollback_index,
    };
    write_file_atomic(
        &slot_meta_path(install_root, &target),
        &serde_json::to_vec(&meta).map_err(|e| e.to_string())?,
    )?;
    sync_tree(&slot_dir(install_root, &target))?;
    sync_dir(&install_root.join(SLOT_DIR))?;

    // 2. Point the active-slot marker at the new image via an atomic rename.
    write_file_atomic(&install_root.join(ACTIVE_FILE), target.as_bytes())?;

    // 3. Update boot control slot candidates.
    let boot_ctrl = SimulatedBootControl::new(install_root);
    let _ = boot_ctrl.set_active_boot_slot(&target);

    // 4. Record the running image (see module header: not TPM-measured).
    let running = RunningState {
        slot: target.clone(),
        image_sha256: manifest.image_sha256.clone(),
        target_device: manifest.target_device.clone(),
        rollback_index: manifest.rollback_index,
    };
    write_file_atomic(
        &install_root.join(RUNNING_FILE),
        &serde_json::to_vec(&running).map_err(|e| e.to_string())?,
    )?;

    // 5. The transaction is complete; drop the journal.
    let _ = std::fs::remove_file(install_root.join(PENDING_FILE));
    sync_dir(install_root)
}

/// Verify and stage an update without flipping the slot. Exposed so a crash
/// between staging and the flip can be simulated and recovered, and so callers
/// can stage ahead of a maintenance window.
pub fn stage_update(
    update_dir: &std::path::Path,
    install_root: &std::path::Path,
    key_dir: &std::path::Path,
) -> Result<UpdateManifest, String> {
    let _lock = PackageLock::acquire(install_root)?;
    recover_pending(install_root)?;
    let manifest = prepare_update(update_dir, install_root, key_dir)?;
    stage_prepared(update_dir, install_root, &manifest)?;
    Ok(manifest)
}

/// Verify and atomically apply a signed system update.
///
/// The image is verified before anything is written, staged into the inactive
/// slot, fsynced, and only then made active by an atomic rename of the slot
/// pointer. A crash at any point leaves the previously active slot bootable and
/// `recover_pending` cleans up on the next run.
pub fn apply_update(
    update_dir: &std::path::Path,
    install_root: &std::path::Path,
    key_dir: &std::path::Path,
) -> Result<(), String> {
    let _lock = PackageLock::acquire(install_root)?;
    recover_pending(install_root)?;
    let manifest = prepare_update(update_dir, install_root, key_dir)?;
    stage_prepared(update_dir, install_root, &manifest)?;
    commit_prepared(install_root, &manifest)?;
    eprintln!(
        "[nilupd] Applied update {} to slot {} (previous slot left intact for rollback)",
        manifest.version, manifest.target_slot
    );
    Ok(())
}

/// Roll back to the inactive slot. Because A/B keeps the previous image, this
/// is a pointer flip and is itself reversible: a second rollback returns to
/// the slot that was active before.
pub fn rollback(install_root: &std::path::Path) -> Result<(), String> {
    let _lock = PackageLock::acquire(install_root)?;
    recover_pending(install_root)?;

    let active = read_active(install_root)?;
    let previous = other_slot(&active)?;
    let image = slot_image_path(install_root, previous);
    if !image.exists() {
        return Err(format!(
            "No image in slot {previous} to roll back to; nothing to restore"
        ));
    }
    // Confirm the fallback image is intact before making it active.
    let meta = read_slot_meta(install_root, previous)
        .ok_or_else(|| format!("Slot {previous} has no metadata; refusing to roll back"))?;
    let (sha, _) = hash_file(&image)?;
    if !meta.image_sha256.is_empty() && sha != meta.image_sha256 {
        return Err(format!(
            "Slot {previous} image does not match its recorded hash; refusing to roll back"
        ));
    }

    // Flip the pointer, then record what is now running.
    write_file_atomic(&install_root.join(ACTIVE_FILE), previous.as_bytes())?;
    let running = RunningState {
        slot: previous.to_string(),
        image_sha256: meta.image_sha256,
        target_device: meta.target_device,
        rollback_index: meta.rollback_index,
    };
    write_file_atomic(
        &install_root.join(RUNNING_FILE),
        &serde_json::to_vec(&running).map_err(|e| e.to_string())?,
    )?;
    let boot_ctrl = SimulatedBootControl::new(install_root);
    let _ = boot_ctrl.set_active_boot_slot(previous);
    sync_dir(install_root)?;
    eprintln!("[nilupd] Rolled back to slot {previous}");
    Ok(())
}

/// Remove orphaned staging files left by a crash. Only ever touches the
/// updater's own dotted temporary names.
fn clean_staging(install_root: &std::path::Path) {
    for slot in SLOTS {
        let dir = slot_dir(install_root, slot);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name == STAGING_NAME || (name.starts_with('.') && name.contains(".tmp.")) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}

/// Complete an interrupted update. Called at the start of apply and rollback
/// (and once by the daemon) so a crash between staging and the slot flip cannot
/// leave a half-applied update. The previous slot always stays active unless the
/// flip itself already completed.
pub fn recover_pending(install_root: &std::path::Path) -> Result<(), String> {
    let pending_path = install_root.join(PENDING_FILE);
    if pending_path.exists() {
        let active = read_active(install_root).ok();
        let pending: Option<PendingUpdate> = std::fs::read(&pending_path)
            .ok()
            .and_then(|d| serde_json::from_slice(&d).ok());

        match pending {
            Some(pending) if active.as_deref() == Some(pending.target_slot.as_str()) => {
                // The flip completed before the crash; just repair running
                // state from the now-active slot and drop the journal.
                if let Some(meta) = read_slot_meta(install_root, &pending.target_slot) {
                    let running = RunningState {
                        slot: pending.target_slot.clone(),
                        image_sha256: meta.image_sha256,
                        target_device: meta.target_device,
                        rollback_index: meta.rollback_index,
                    };
                    if let Ok(bytes) = serde_json::to_vec(&running) {
                        let _ = write_file_atomic(&install_root.join(RUNNING_FILE), &bytes);
                    }
                }
                eprintln!(
                    "[nilupd] Recovered completed update to slot {}",
                    pending.target_slot
                );
            }
            _ => {
                eprintln!(
                    "[nilupd] Recovered interrupted update; previous slot remains active"
                );
            }
        }
        let _ = std::fs::remove_file(&pending_path);
        sync_dir(install_root)?;
    }
    clean_staging(install_root);
    Ok(())
}

/// Report the active slot, running image hash, pending state and per-slot info.
pub fn status(install_root: &std::path::Path) -> Result<Status, String> {
    let active_slot = read_active(install_root)?;
    let running = read_running(install_root)?;
    let mut slots = Vec::new();
    for slot in SLOTS {
        let meta = read_slot_meta(install_root, slot);
        slots.push(SlotStatus {
            slot: slot.to_string(),
            active: slot == active_slot,
            version: meta.as_ref().map(|m| m.version.clone()),
            image_sha256: meta.map(|m| m.image_sha256),
        });
    }
    let boot_ctrl = SimulatedBootControl::new(install_root);
    let boot_control = boot_ctrl.get_status().ok();
    Ok(Status {
        active_slot,
        running_image_sha256: running.image_sha256,
        pending: install_root.join(PENDING_FILE).exists(),
        slots,
        boot_control,
    })
}

/// Query the current boot-control status.
pub fn boot_status(install_root: &std::path::Path) -> Result<BootControlStatus, String> {
    let boot_ctrl = SimulatedBootControl::new(install_root);
    boot_ctrl.get_status()
}

/// Confirm that the current slot booted successfully to prevent automated rollback.
pub fn mark_boot_successful(install_root: &std::path::Path) -> Result<(), String> {
    let boot_ctrl = SimulatedBootControl::new(install_root);
    boot_ctrl.mark_boot_successful()
}

/// Record a boot attempt. If unverified tries are exhausted, automatically rolls back.
pub fn record_boot_attempt(install_root: &std::path::Path) -> Result<String, String> {
    let boot_ctrl = SimulatedBootControl::new(install_root);
    boot_ctrl.record_boot_attempt()
}

/// Handle framed IPC requests for system update queries and triggers.
pub fn handle_ipc_request(
    frame: &Frame,
    install_root: &std::path::Path,
    _key_dir: &std::path::Path,
) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),
        MessageType::UpdateGetStatus => match status(install_root) {
            Ok(s) => {
                let json = serde_json::to_vec(&s).unwrap_or_default();
                Frame::new(MessageType::UpdateStatusInfo, frame.request_id, json)
            }
            Err(e) => Frame::new(MessageType::ErrorResponse, frame.request_id, e.into_bytes()),
        },
        MessageType::UpdateRollback => match rollback(install_root) {
            Ok(()) => Frame::new(MessageType::Pong, frame.request_id, b"rolled_back".to_vec()),
            Err(e) => Frame::new(MessageType::ErrorResponse, frame.request_id, e.into_bytes()),
        },
        _ => Frame::new(
            MessageType::ErrorResponse,
            frame.request_id,
            b"unsupported message type".to_vec(),
        ),
    }
}

/// Run nilupd daemon: recover pending updates on startup, then serve IPC requests.
pub fn run_daemon() -> ! {
    println!("[nilupd] A/B System Image Updater daemon active.");
    let install_root = get_install_root();
    let key_dir = nilpkg::get_key_dir();
    let _ = &key_dir;
    let _ = std::fs::create_dir_all("/run/onuron");

    if let Err(e) = recover_pending(&install_root) {
        eprintln!("[nilupd] Recovery failed: {e}");
    } else {
        println!("[nilupd] Pending updates recovered (if any).");
    }

    #[cfg(unix)]
    {
        let root_ipc = install_root.clone();
        let key_ipc = key_dir.clone();
        std::thread::spawn(move || {
            let listener = match nilsd::first_listener_or_bind("/run/onuron/update.sock") {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[nilupd] Failed to bind IPC socket /run/onuron/update.sock: {e}");
                    return;
                }
            };
            println!("\x1b[1;32m[nilupd] [  OK  ]\x1b[0m IPC Server listening on /run/onuron/update.sock");

            for stream in listener.incoming() {
                if let Ok(mut sock) = stream {
                    let root = root_ipc.clone();
                    let key = key_ipc.clone();
                    std::thread::spawn(move || {
                        while let Ok(frame) = Frame::read_from(&mut sock) {
                            let resp = handle_ipc_request(&frame, &root, &key);
                            if let Err(e) = resp.write_to(&mut sock) {
                                eprintln!("[nilupd] IPC send error: {e}");
                                break;
                            }
                        }
                    });
                }
            }
        });
    }

    loop {
        std::thread::sleep(std::time::Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::SigningKey;

    fn temp_root(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("nilupd-{}-{}", std::process::id(), name))
    }

    struct Fixture {
        root: std::path::PathBuf,
        install: std::path::PathBuf,
        keys: std::path::PathBuf,
        signer: SigningKey,
        initial: Vec<u8>,
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.root);
        }
    }

    fn fixture(name: &str) -> Fixture {
        let root = temp_root(name);
        let _ = std::fs::remove_dir_all(&root);
        let keys = root.join("keys");
        std::fs::create_dir_all(keys.join("trusted")).unwrap();
        let (signer, verifier) = nilpkg::generate_keypair();
        std::fs::write(keys.join("trusted/system.pub"), hex::encode(verifier.to_bytes())).unwrap();
        let install = root.join("install");
        let initial = b"initial-system-image".to_vec();
        initialize_install_root(&install, &initial).unwrap();
        Fixture {
            root,
            install,
            keys,
            signer,
            initial,
        }
    }

    fn signed_update(
        dir: &std::path::Path,
        signer: &SigningKey,
        target_slot: &str,
        current_sha: &str,
        image: &[u8],
    ) -> UpdateManifest {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("image"), image).unwrap();
        let mut manifest = UpdateManifest {
            name: "onuron-system".into(),
            version: "2.0.0".into(),
            target_slot: target_slot.into(),
            current_image_sha256: current_sha.into(),
            image_sha256: compute_sha256(image),
            image_size: image.len() as u64,
            target_device: String::new(),
            rollback_index: 0,
            signature_hex: None,
            public_key_hex: None,
        };
        sign_update(&mut manifest, signer);
        std::fs::write(dir.join("update.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();
        manifest
    }

    fn active(fx: &Fixture) -> String {
        std::fs::read_to_string(fx.install.join(ACTIVE_FILE)).unwrap().trim().to_string()
    }

    #[test]
    fn valid_signed_update_applies_and_flips_the_slot() {
        let fx = fixture("apply");
        let update = fx.root.join("update");
        let new_image = b"new-system-image-v2";
        signed_update(
            &update,
            &fx.signer,
            "B",
            &compute_sha256(&fx.initial),
            new_image,
        );

        apply_update(&update, &fx.install, &fx.keys).unwrap();

        assert_eq!(active(&fx), "B", "active slot must flip to B");
        assert_eq!(std::fs::read(fx.install.join("slots/B/image")).unwrap(), new_image);
        // The previous slot is untouched, so rollback is always possible.
        assert_eq!(std::fs::read(fx.install.join("slots/A/image")).unwrap(), fx.initial);
        assert!(!fx.install.join(PENDING_FILE).exists(), "journal must be cleared");

        let st = status(&fx.install).unwrap();
        assert_eq!(st.active_slot, "B");
        assert_eq!(st.running_image_sha256, compute_sha256(new_image));
        assert!(!st.pending);
    }

    #[test]
    fn tampered_payload_is_rejected_and_install_root_unchanged() {
        let fx = fixture("tamper");
        let update = fx.root.join("update");
        signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), b"good-image");
        // Corrupt the payload after signing.
        std::fs::write(update.join("image"), b"tampered-image").unwrap();

        let err = apply_update(&update, &fx.install, &fx.keys).unwrap_err();
        assert!(
            err.contains("SHA-256") || err.contains("size mismatch"),
            "unexpected error: {err}"
        );
        assert_eq!(active(&fx), "A", "active slot must not change");
        assert!(
            !fx.install.join("slots/B/image").exists(),
            "no image may be written for a rejected update"
        );
        assert!(!fx.install.join(PENDING_FILE).exists());
    }

    #[test]
    fn untrusted_publisher_is_rejected() {
        let fx = fixture("untrusted");
        let update = fx.root.join("update");
        // Signed by a key that is NOT in the trust store.
        let (other_signer, _other_verify) = nilpkg::generate_keypair();
        signed_update(
            &update,
            &other_signer,
            "B",
            &compute_sha256(&fx.initial),
            b"image-from-stranger",
        );

        let err = apply_update(&update, &fx.install, &fx.keys).unwrap_err();
        assert!(err.contains("trusted"), "unexpected error: {err}");
        assert_eq!(active(&fx), "A");
        assert!(!fx.install.join("slots/B/image").exists());
    }

    #[test]
    fn running_image_hash_mismatch_is_refused() {
        let fx = fixture("anchor");
        let update = fx.root.join("update");
        signed_update(
            &update,
            &fx.signer,
            "B",
            &"0".repeat(64),
            b"image",
        );

        let err = apply_update(&update, &fx.install, &fx.keys).unwrap_err();
        assert!(err.contains("Running image hash"), "unexpected error: {err}");
        assert_eq!(active(&fx), "A");
    }

    #[test]
    fn crash_after_staging_before_flip_is_recovered_with_old_slot_active() {
        let fx = fixture("recover");
        let update = fx.root.join("update");
        signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), b"staged-but-not-live");

        // Stage only: this is exactly the state after a crash before commit.
        stage_update(&update, &fx.install, &fx.keys).unwrap();
        assert!(fx.install.join(PENDING_FILE).exists(), "journal should exist after staging");
        assert!(staging_path(&fx.install, "B").exists(), "staged file should exist");
        assert_eq!(active(&fx), "A", "slot must not flip during staging");
        assert!(!fx.install.join("slots/B/image").exists(), "no committed image yet");

        recover_pending(&fx.install).unwrap();

        assert!(!fx.install.join(PENDING_FILE).exists(), "journal must be cleared");
        assert!(!staging_path(&fx.install, "B").exists(), "staged file must be removed");
        assert_eq!(active(&fx), "A", "old slot must still be active after recovery");
        assert_eq!(std::fs::read(fx.install.join("slots/A/image")).unwrap(), fx.initial);

        // A later apply must still work after recovery.
        apply_update(&update, &fx.install, &fx.keys).unwrap();
        assert_eq!(active(&fx), "B");
    }

    #[test]
    fn rollback_restores_previous_slot_and_is_reversible() {
        let fx = fixture("rollback");
        let update = fx.root.join("update");
        let new_image = b"new-system-image-v2";
        signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), new_image);
        apply_update(&update, &fx.install, &fx.keys).unwrap();
        assert_eq!(active(&fx), "B");

        rollback(&fx.install).unwrap();
        assert_eq!(active(&fx), "A");
        assert_eq!(std::fs::read(fx.install.join("slots/A/image")).unwrap(), fx.initial);
        assert_eq!(status(&fx.install).unwrap().running_image_sha256, compute_sha256(&fx.initial));

        // Rollback is reversible: flipping again returns to the newer slot.
        rollback(&fx.install).unwrap();
        assert_eq!(active(&fx), "B");
        assert_eq!(std::fs::read(fx.install.join("slots/B/image")).unwrap(), new_image);
    }

    #[test]
    fn verify_image_accepts_a_valid_signed_update() {
        let fx = fixture("verify");
        let update = fx.root.join("update");
        let m = signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), b"payload");
        let verified = verify_image(&update, &fx.keys).unwrap();
        assert_eq!(verified, m);
    }

    #[test]
    fn test_update_ipc_status_and_rollback() {
        let fx = fixture("ipc");
        let update = fx.root.join("update");
        let new_image = b"image-for-ipc";
        signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), new_image);
        apply_update(&update, &fx.install, &fx.keys).unwrap();

        // 1. Ping
        let ping_frame = Frame::new(MessageType::Ping, 1, vec![]);
        let pong_frame = handle_ipc_request(&ping_frame, &fx.install, &fx.keys);
        assert_eq!(pong_frame.message_type, u16::from(MessageType::Pong));

        // 2. UpdateGetStatus
        let stat_req = Frame::new(MessageType::UpdateGetStatus, 2, vec![]);
        let stat_resp = handle_ipc_request(&stat_req, &fx.install, &fx.keys);
        assert_eq!(stat_resp.message_type, u16::from(MessageType::UpdateStatusInfo));
        let s: Status = serde_json::from_slice(&stat_resp.payload).unwrap();
        assert_eq!(s.active_slot, "B");

        // 3. UpdateRollback
        let rb_req = Frame::new(MessageType::UpdateRollback, 3, vec![]);
        let rb_resp = handle_ipc_request(&rb_req, &fx.install, &fx.keys);
        assert_eq!(rb_resp.message_type, u16::from(MessageType::Pong));
        assert_eq!(rb_resp.payload, b"rolled_back");

        // Verify status after rollback
        let stat_resp2 = handle_ipc_request(&stat_req, &fx.install, &fx.keys);
        let s2: Status = serde_json::from_slice(&stat_resp2.payload).unwrap();
        assert_eq!(s2.active_slot, "A");
    }

    #[test]
    fn test_anti_rollback_protection() {
        let fx = fixture("anti_rollback");
        let update = fx.root.join("update");

        // Set running state with security version 5
        let running = RunningState {
            slot: "A".into(),
            image_sha256: compute_sha256(&fx.initial),
            target_device: String::new(),
            rollback_index: 5,
        };
        std::fs::write(
            fx.install.join(RUNNING_FILE),
            serde_json::to_vec(&running).unwrap(),
        ).unwrap();

        // Create downgrade update with rollback_index 4
        std::fs::create_dir_all(&update).unwrap();
        let new_img = b"downgrade-image";
        std::fs::write(update.join("image"), new_img).unwrap();
        let mut manifest = UpdateManifest {
            name: "onuron-system".into(),
            version: "1.9.0".into(),
            target_slot: "B".into(),
            current_image_sha256: compute_sha256(&fx.initial),
            image_sha256: compute_sha256(new_img),
            image_size: new_img.len() as u64,
            target_device: String::new(),
            rollback_index: 4,
            signature_hex: None,
            public_key_hex: None,
        };
        sign_update(&mut manifest, &fx.signer);
        std::fs::write(update.join("update.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();

        let err = apply_update(&update, &fx.install, &fx.keys).unwrap_err();
        assert!(err.contains("Anti-rollback violation"), "Expected anti-rollback error, got: {err}");
        assert_eq!(active(&fx), "A");
    }

    #[test]
    fn test_target_device_mismatch() {
        let fx = fixture("target_mismatch");
        let update = fx.root.join("update");

        // Running device is oneplus-fajita
        let running = RunningState {
            slot: "A".into(),
            image_sha256: compute_sha256(&fx.initial),
            target_device: "oneplus-fajita".into(),
            rollback_index: 1,
        };
        std::fs::write(
            fx.install.join(RUNNING_FILE),
            serde_json::to_vec(&running).unwrap(),
        ).unwrap();

        // Update is built for qemu-x86_64
        std::fs::create_dir_all(&update).unwrap();
        let new_img = b"qemu-image";
        std::fs::write(update.join("image"), new_img).unwrap();
        let mut manifest = UpdateManifest {
            name: "onuron-system".into(),
            version: "2.0.0".into(),
            target_slot: "B".into(),
            current_image_sha256: compute_sha256(&fx.initial),
            image_sha256: compute_sha256(new_img),
            image_size: new_img.len() as u64,
            target_device: "qemu-x86_64".into(),
            rollback_index: 1,
            signature_hex: None,
            public_key_hex: None,
        };
        sign_update(&mut manifest, &fx.signer);
        std::fs::write(update.join("update.json"), serde_json::to_vec(&manifest).unwrap()).unwrap();

        let err = apply_update(&update, &fx.install, &fx.keys).unwrap_err();
        assert!(err.contains("Target device mismatch"), "Expected device mismatch error, got: {err}");
        assert_eq!(active(&fx), "A");
    }

    #[test]
    fn test_boot_control_retry_exhaustion_auto_rollback() {
        let fx = fixture("boot_ctrl_rollback");
        let update = fx.root.join("update");
        let new_img = b"failing-candidate-image";
        signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), new_img);
        apply_update(&update, &fx.install, &fx.keys).unwrap();
        assert_eq!(active(&fx), "B");

        let bc = boot_status(&fx.install).unwrap();
        assert_eq!(bc.current_slot, "B");
        assert_eq!(bc.fallback_slot, "A");
        let b_info = bc.slots.iter().find(|s| s.slot == "B").unwrap();
        assert_eq!(b_info.tries_remaining, 3);
        assert!(!b_info.is_successful);

        // Attempt 1
        let slot1 = record_boot_attempt(&fx.install).unwrap();
        assert_eq!(slot1, "B");
        let bc1 = boot_status(&fx.install).unwrap();
        assert_eq!(bc1.slots.iter().find(|s| s.slot == "B").unwrap().tries_remaining, 2);

        // Attempt 2
        let slot2 = record_boot_attempt(&fx.install).unwrap();
        assert_eq!(slot2, "B");
        let bc2 = boot_status(&fx.install).unwrap();
        assert_eq!(bc2.slots.iter().find(|s| s.slot == "B").unwrap().tries_remaining, 1);

        // Attempt 3: Exhaustion triggers automatic rollback!
        let err3 = record_boot_attempt(&fx.install).unwrap_err();
        assert!(err3.contains("exhausted") && err3.contains("Rolled back to slot A"), "got: {err3}");

        // Active slot must now be A
        assert_eq!(active(&fx), "A");
        let bc3 = boot_status(&fx.install).unwrap();
        assert_eq!(bc3.current_slot, "A");
        let b_info_after = bc3.slots.iter().find(|s| s.slot == "B").unwrap();
        assert!(!b_info_after.is_bootable, "Failed slot B should be marked unbootable");
    }

    #[test]
    fn test_boot_control_mark_success_persists() {
        let fx = fixture("boot_ctrl_success");
        let update = fx.root.join("update");
        signed_update(&update, &fx.signer, "B", &compute_sha256(&fx.initial), b"good-candidate");
        apply_update(&update, &fx.install, &fx.keys).unwrap();

        mark_boot_successful(&fx.install).unwrap();
        let bc = boot_status(&fx.install).unwrap();
        let b_info = bc.slots.iter().find(|s| s.slot == "B").unwrap();
        assert!(b_info.is_successful);
        assert_eq!(b_info.tries_remaining, 3);
        assert_eq!(bc.fallback_slot, "B");

        // Subsequent boot attempt does not decrement tries
        let slot = record_boot_attempt(&fx.install).unwrap();
        assert_eq!(slot, "B");
        let bc2 = boot_status(&fx.install).unwrap();
        assert_eq!(bc2.slots.iter().find(|s| s.slot == "B").unwrap().tries_remaining, 3);
    }
}