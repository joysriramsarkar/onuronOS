// services/nilkeyd/src/lib.rs — Encryption-at-rest logic for nilkeyd
//
// Key hierarchy
// -------------
//   device secret : 32 random bytes stored at a path outside /data, created with
//                   mode 0o400 when no TPM is available.
//   KEK           : PBKDF2-SHA256(PIN, salt, iterations) mixed with the device
//                   secret via SHA-256 domain separation.
//   master key    : 32 random bytes, generated once and stored only *wrapped*
//                   (AES-256-GCM under the KEK) in the key record file.
//
// A wrong PIN cannot decrypt the record. Changing the PIN re-wraps the same
// master key, so previously encrypted data keeps working without re-encryption.
//
// Recovery: losing the device secret makes /data unrecoverable. Back up the
// device secret and the key record together; there is no other recovery path.

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use ring::aead;
use ring::pbkdf2;
use sha2::{Digest, Sha256};

pub mod fscrypt;
pub use fscrypt::{apply_fscrypt_policy, FscryptResult};

/// Master/KEK length in bytes.
pub const KEY_LEN: usize = 32;
/// Default PBKDF2 iteration count for production records.
pub const PBKDF2_ITERATIONS: u32 = 100_000;
/// Current key-record format version.
pub const RECORD_FORMAT_VERSION: u32 = 1;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const TAG_LEN: usize = 16;
const FINGERPRINT_LEN: usize = 8;

/// On-disk wrapped-key record. All binary fields are lowercase hex.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct KeyRecord {
    pub format_version: u32,
    pub device_secret_fingerprint: String,
    pub kdf_salt: String,
    pub kdf_iterations: u32,
    pub nonce: String,
    pub ciphertext: String,
}

/// An AES-256-GCM encrypted data blob (nonce prepended by convention).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct EncryptedBlob {
    pub nonce: Vec<u8>,
    pub ciphertext: Vec<u8>,
}

/// A freshly unwrapped master key. Kept opaque so callers do not accidentally
/// log it.
#[derive(Clone, PartialEq, Eq)]
pub struct MasterKey(pub [u8; KEY_LEN]);

impl std::fmt::Debug for MasterKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("MasterKey([redacted])")
    }
}

/// Errors from unwrapping / decrypting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnwrapError {
    /// No device secret file (or a record is present without one).
    MissingDeviceSecret,
    /// Device secret does not match the fingerprint stored in the record.
    DeviceSecretMismatch,
    /// AEAD authentication failed: wrong PIN or tampered record.
    DecryptionFailed,
    /// The record is malformed or has an unsupported version.
    InvalidFormat(String),
    Io(String),
}

impl std::fmt::Display for UnwrapError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UnwrapError::MissingDeviceSecret => write!(f, "device secret is missing"),
            UnwrapError::DeviceSecretMismatch => write!(f, "device secret does not match key record"),
            UnwrapError::DecryptionFailed => write!(f, "authentication failed (wrong PIN or tampered record)"),
            UnwrapError::InvalidFormat(m) => write!(f, "invalid key record: {}", m),
            UnwrapError::Io(m) => write!(f, "i/o error: {}", m),
        }
    }
}

impl std::error::Error for UnwrapError {}

impl From<std::io::Error> for UnwrapError {
    fn from(e: std::io::Error) -> Self {
        UnwrapError::Io(e.to_string())
    }
}

/// Errors from initializing a record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitError {
    /// A record already exists and `force` was not set.
    RecordExists(PathBuf),
    Io(String),
}

impl std::fmt::Display for InitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InitError::RecordExists(p) => write!(f, "key record already exists at {}", p.display()),
            InitError::Io(m) => write!(f, "i/o error: {}", m),
        }
    }
}

impl std::error::Error for InitError {}

impl From<std::io::Error> for InitError {
    fn from(e: std::io::Error) -> Self {
        InitError::Io(e.to_string())
    }
}

pub fn default_device_secret_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/metadata/onuron/device.secret")
    }
    #[cfg(not(unix))]
    {
        std::env::temp_dir().join("onuron-device.secret")
    }
}

pub fn default_record_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/data/nilos/enc_key.json")
    }
    #[cfg(not(unix))]
    {
        std::env::temp_dir().join("onuron-enc_key.json")
    }
}

pub fn default_master_key_path() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/run/nilos/master.key")
    }
    #[cfg(not(unix))]
    {
        std::env::temp_dir().join("onuron-master.key")
    }
}

pub fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

pub fn hex_decode(hex: &str) -> Result<Vec<u8>, UnwrapError> {
    if !hex.len().is_multiple_of(2) {
        return Err(UnwrapError::InvalidFormat("odd-length hex".into()));
    }
    let mut out = Vec::with_capacity(hex.len() / 2);
    let bytes = hex.as_bytes();
    for pair in bytes.chunks(2) {
        let hi = hex_nibble(pair[0])?;
        let lo = hex_nibble(pair[1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

fn hex_nibble(c: u8) -> Result<u8, UnwrapError> {
    match c {
        b'0'..=b'9' => Ok(c - b'0'),
        b'a'..=b'f' => Ok(c - b'a' + 10),
        b'A'..=b'F' => Ok(c - b'A' + 10),
        _ => Err(UnwrapError::InvalidFormat("non-hex character".into())),
    }
}

/// Best-effort wiping of sensitive buffers.
fn zeroize(buf: &mut [u8]) {
    for b in buf.iter_mut() {
        // SAFETY: writing through a valid mutable reference.
        unsafe { std::ptr::write_volatile(b, 0) };
    }
    std::sync::atomic::compiler_fence(std::sync::atomic::Ordering::SeqCst);
}

/// SHA-256 fingerprint of the device secret, first 8 bytes hex.
pub fn device_secret_fingerprint(secret: &[u8]) -> String {
    let digest = Sha256::digest(secret);
    hex_encode(&digest[..FINGERPRINT_LEN])
}

/// Read the device secret, creating a fresh random one (mode 0o400 on Unix) if
/// absent. Never logs the bytes.
pub fn load_or_create_device_secret(path: &Path) -> std::io::Result<Vec<u8>> {
    if path.exists() {
        let bytes = std::fs::read(path)?;
        #[cfg(unix)]
        {
            if let Ok(meta) = std::fs::metadata(path) {
                let mode = meta.permissions().mode() & 0o777;
                if mode != 0o400 {
                    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o400));
                }
            }
        }
        return Ok(bytes);
    }

    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }

    let mut secret = vec![0u8; KEY_LEN];
    use ring::rand::{SecureRandom, SystemRandom};
    SystemRandom::new()
        .fill(&mut secret)
        .map_err(|_| std::io::Error::other("system RNG failed"))?;
    std::fs::write(path, &secret)?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o400))?;
    }
    Ok(secret)
}

/// Read an existing device secret without creating one.
pub fn load_device_secret(path: &Path) -> Result<Vec<u8>, UnwrapError> {
    if !path.exists() {
        return Err(UnwrapError::MissingDeviceSecret);
    }
    let bytes = std::fs::read(path).map_err(UnwrapError::from)?;
    if bytes.len() != KEY_LEN {
        return Err(UnwrapError::InvalidFormat("device secret has wrong length".into()));
    }
    Ok(bytes)
}

/// Derive the KEK from the PIN, KDF salt/iterations, and device secret.
fn derive_kek(pin: &str, salt: &[u8], iterations: u32, device_secret: &[u8]) -> [u8; KEY_LEN] {
    let mut pin_component = [0u8; KEY_LEN];
    pbkdf2::derive(
        pbkdf2::PBKDF2_HMAC_SHA256,
        std::num::NonZeroU32::new(iterations).unwrap_or(std::num::NonZeroU32::new(1).unwrap()),
        salt,
        pin.as_bytes(),
        &mut pin_component,
    );

    // Domain-separated mix with the device secret.
    let mut hasher = Sha256::new();
    hasher.update(b"nilkeyd/kek/v1");
    hasher.update(pin_component);
    hasher.update(device_secret);
    let digest = hasher.finalize();

    let mut kek = [0u8; KEY_LEN];
    kek.copy_from_slice(&digest[..KEY_LEN]);
    zeroize(&mut pin_component);
    kek
}

/// Decoded `(kdf_salt, nonce, ciphertext)` fields of a key record.
type ValidatedRecord = (Vec<u8>, Vec<u8>, Vec<u8>);

fn validate_record(record: &KeyRecord) -> Result<ValidatedRecord, UnwrapError> {
    if record.format_version != RECORD_FORMAT_VERSION {
        return Err(UnwrapError::InvalidFormat(format!(
            "unsupported version {}",
            record.format_version
        )));
    }
    if record.device_secret_fingerprint.len() != FINGERPRINT_LEN * 2
        || hex_decode(&record.device_secret_fingerprint).is_err()
    {
        return Err(UnwrapError::InvalidFormat("bad device fingerprint".into()));
    }
    if record.kdf_iterations == 0 {
        return Err(UnwrapError::InvalidFormat("non-positive iterations".into()));
    }
    let salt = hex_decode(&record.kdf_salt)?;
    if salt.len() < SALT_LEN {
        return Err(UnwrapError::InvalidFormat("kdf salt too short".into()));
    }
    let nonce = hex_decode(&record.nonce)?;
    if nonce.len() != NONCE_LEN {
        return Err(UnwrapError::InvalidFormat("bad nonce length".into()));
    }
    let ciphertext = hex_decode(&record.ciphertext)?;
    if ciphertext.len() < TAG_LEN {
        return Err(UnwrapError::InvalidFormat("ciphertext too short".into()));
    }
    Ok((salt, nonce, ciphertext))
}

fn wrap_master_key(
    master: &[u8; KEY_LEN],
    kek: &[u8; KEY_LEN],
) -> Result<(Vec<u8>, Vec<u8>), UnwrapError> {
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, kek)
        .map_err(|_| UnwrapError::InvalidFormat("bad KEK length".into()))?;
    let sealing_key = aead::LessSafeKey::new(unbound);

    let mut nonce = [0u8; NONCE_LEN];
    use ring::rand::{SecureRandom, SystemRandom};
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| UnwrapError::Io("system RNG failed".into()))?;

    let mut in_out = master.to_vec();
    let tag = sealing_key
        .seal_in_place_separate_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::empty(),
            &mut in_out,
        )
        .map_err(|_| UnwrapError::InvalidFormat("seal failed".into()))?;
    in_out.extend_from_slice(tag.as_ref());
    Ok((nonce.to_vec(), in_out))
}

fn build_record(
    master: &[u8; KEY_LEN],
    pin: &str,
    device_secret: &[u8],
    salt: &[u8],
    iterations: u32,
) -> Result<KeyRecord, UnwrapError> {
    let mut kek = derive_kek(pin, salt, iterations, device_secret);
    let (nonce, ciphertext) = wrap_master_key(master, &kek)?;
    zeroize(&mut kek);
    Ok(KeyRecord {
        format_version: RECORD_FORMAT_VERSION,
        device_secret_fingerprint: device_secret_fingerprint(device_secret),
        kdf_salt: hex_encode(salt),
        kdf_iterations: iterations,
        nonce: hex_encode(&nonce),
        ciphertext: hex_encode(&ciphertext),
    })
}

/// Unwrap the master key. Returns `Err` (never panics) on a wrong PIN, a
/// tampered record, or a missing/mismatched device secret.
pub fn unwrap_master_key(
    record: &KeyRecord,
    pin: &str,
    device_secret: &[u8],
) -> Result<[u8; KEY_LEN], UnwrapError> {
    let (salt, nonce, mut ciphertext) = validate_record(record)?;

    if device_secret_fingerprint(device_secret) != record.device_secret_fingerprint {
        return Err(UnwrapError::DeviceSecretMismatch);
    }

    let mut kek = derive_kek(pin, &salt, record.kdf_iterations, device_secret);
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, &kek)
        .map_err(|_| UnwrapError::InvalidFormat("bad KEK length".into()))?;
    zeroize(&mut kek);
    let opening_key = aead::LessSafeKey::new(unbound);

    let nonce_arr: [u8; NONCE_LEN] = nonce
        .as_slice()
        .try_into()
        .map_err(|_| UnwrapError::InvalidFormat("bad nonce length".into()))?;
    let plaintext = opening_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_arr),
            aead::Aad::empty(),
            &mut ciphertext,
        )
        .map_err(|_| UnwrapError::DecryptionFailed)?;

    if plaintext.len() != KEY_LEN {
        return Err(UnwrapError::InvalidFormat("unexpected master key length".into()));
    }
    let mut master = [0u8; KEY_LEN];
    master.copy_from_slice(plaintext);
    Ok(master)
}

pub fn read_record(path: &Path) -> Result<KeyRecord, UnwrapError> {
    let content = std::fs::read_to_string(path).map_err(UnwrapError::from)?;
    serde_json::from_str(&content).map_err(|e| UnwrapError::InvalidFormat(e.to_string()))
}

pub fn write_record(path: &Path, record: &KeyRecord) -> Result<(), UnwrapError> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(UnwrapError::from)?;
        }
    }
    let content = serde_json::to_string_pretty(record)
        .map_err(|e| UnwrapError::InvalidFormat(e.to_string()))?;
    std::fs::write(path, content).map_err(UnwrapError::from)?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Create the device secret (if absent) plus a fresh wrapped master key.
pub fn init_record(
    record_path: &Path,
    device_secret_path: &Path,
    pin: &str,
    force: bool,
) -> Result<KeyRecord, InitError> {
    if record_path.exists() && !force {
        return Err(InitError::RecordExists(record_path.to_path_buf()));
    }
    let device_secret = load_or_create_device_secret(device_secret_path)?;

    let mut master = [0u8; KEY_LEN];
    use ring::rand::{SecureRandom, SystemRandom};
    SystemRandom::new()
        .fill(&mut master)
        .map_err(|_| InitError::Io("system RNG failed".into()))?;
    let mut salt = [0u8; SALT_LEN];
    SystemRandom::new()
        .fill(&mut salt)
        .map_err(|_| InitError::Io("system RNG failed".into()))?;

    let record = build_record(&master, pin, &device_secret, &salt, PBKDF2_ITERATIONS)
        .map_err(|e| InitError::Io(e.to_string()))?;
    zeroize(&mut master);
    write_record(record_path, &record).map_err(|e| InitError::Io(e.to_string()))?;
    Ok(record)
}

/// Unwrap the master key and write it to `out_path` (mode 0o600 on Unix).
/// Never prints the key unless `--print-key` was explicitly requested.
pub fn unwrap_to_file(
    record_path: &Path,
    device_secret_path: &Path,
    pin: &str,
    out_path: &Path,
    print_key: bool,
) -> Result<(), UnwrapError> {
    let record = read_record(record_path)?;
    let device_secret = load_device_secret(device_secret_path)?;
    let master = unwrap_master_key(&record, pin, &device_secret)?;

    if let Some(parent) = out_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(UnwrapError::from)?;
        }
    }
    std::fs::write(out_path, master).map_err(UnwrapError::from)?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(out_path, std::fs::Permissions::from_mode(0o600))?;
    }

    if print_key {
        println!(
            "WARNING: --print-key is for debugging only; exposing the master key compromises all encrypted data."
        );
        println!("master key: {}", hex_encode(&master));
    }
    Ok(())
}

/// Re-wrap the master key under a new PIN. The master key bytes are unchanged.
pub fn rewrap_record(
    record_path: &Path,
    device_secret_path: &Path,
    old_pin: &str,
    new_pin: &str,
) -> Result<MasterKey, UnwrapError> {
    let record = read_record(record_path)?;
    let device_secret = load_device_secret(device_secret_path)?;
    let master = unwrap_master_key(&record, old_pin, &device_secret)?;

    let salt = hex_decode(&record.kdf_salt)?;
    let new_record = build_record(&master, new_pin, &device_secret, &salt, record.kdf_iterations)
        .map_err(|e| UnwrapError::InvalidFormat(e.to_string()))?;
    write_record(record_path, &new_record)?;
    Ok(MasterKey(master))
}

pub fn load_master_from_file(path: &Path) -> std::io::Result<[u8; KEY_LEN]> {
    let bytes = std::fs::read(path)?;
    if bytes.len() != KEY_LEN {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "master key has wrong length",
        ));
    }
    let mut master = [0u8; KEY_LEN];
    master.copy_from_slice(&bytes);
    Ok(master)
}

/// Encrypt an arbitrary data blob under the master key.
pub fn encrypt_data(master: &[u8; KEY_LEN], plaintext: &[u8]) -> Result<EncryptedBlob, UnwrapError> {
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, master)
        .map_err(|_| UnwrapError::InvalidFormat("bad master key".into()))?;
    let sealing_key = aead::LessSafeKey::new(unbound);

    let mut nonce = [0u8; NONCE_LEN];
    use ring::rand::{SecureRandom, SystemRandom};
    SystemRandom::new()
        .fill(&mut nonce)
        .map_err(|_| UnwrapError::Io("system RNG failed".into()))?;

    let mut in_out = plaintext.to_vec();
    let tag = sealing_key
        .seal_in_place_separate_tag(
            aead::Nonce::assume_unique_for_key(nonce),
            aead::Aad::empty(),
            &mut in_out,
        )
        .map_err(|_| UnwrapError::InvalidFormat("seal failed".into()))?;
    in_out.extend_from_slice(tag.as_ref());
    Ok(EncryptedBlob {
        nonce: nonce.to_vec(),
        ciphertext: in_out,
    })
}

/// Decrypt a blob produced by [`encrypt_data`].
pub fn decrypt_data(master: &[u8; KEY_LEN], blob: &EncryptedBlob) -> Result<Vec<u8>, UnwrapError> {
    if blob.nonce.len() != NONCE_LEN || blob.ciphertext.len() < TAG_LEN {
        return Err(UnwrapError::InvalidFormat("bad blob".into()));
    }
    let unbound = aead::UnboundKey::new(&aead::AES_256_GCM, master)
        .map_err(|_| UnwrapError::InvalidFormat("bad master key".into()))?;
    let opening_key = aead::LessSafeKey::new(unbound);
    let nonce_arr: [u8; NONCE_LEN] = blob
        .nonce
        .as_slice()
        .try_into()
        .map_err(|_| UnwrapError::InvalidFormat("bad nonce".into()))?;
    let mut in_out = blob.ciphertext.clone();
    let plaintext = opening_key
        .open_in_place(
            aead::Nonce::assume_unique_for_key(nonce_arr),
            aead::Aad::empty(),
            &mut in_out,
        )
        .map_err(|_| UnwrapError::DecryptionFailed)?;
    Ok(plaintext.to_vec())
}



/// True when the wrapped key record and a same-boot unwrapped master key exist.
pub fn is_unlocked(record_path: &Path, master_path: &Path) -> bool {
    record_path.exists() && master_path.exists()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    const TEST_ITERS: u32 = 1_000;

    fn setup(dir: &Path) -> (PathBuf, PathBuf, PathBuf) {
        (
            dir.join("enc_key.json"),
            dir.join("device.secret"),
            dir.join("master.key"),
        )
    }

    fn fast_record(record_path: &Path, secret_path: &Path, pin: &str) -> KeyRecord {
        let secret = load_or_create_device_secret(secret_path).unwrap();
        let master = [7u8; KEY_LEN];
        let salt = [3u8; SALT_LEN];
        let record = build_record(&master, pin, &secret, &salt, TEST_ITERS).unwrap();
        write_record(record_path, &record).unwrap();
        record
    }

    #[test]
    fn init_unwrap_encrypt_decrypt_roundtrip() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, master_path) = setup(dir.path());
        init_record(&record_path, &secret_path, "1234", false).unwrap();
        unwrap_to_file(&record_path, &secret_path, "1234", &master_path, false).unwrap();
        let master = load_master_from_file(&master_path).unwrap();
        let blob = encrypt_data(&master, b"hello world").unwrap();
        assert_eq!(decrypt_data(&master, &blob).unwrap(), b"hello world");
    }

    #[test]
    fn init_refuses_overwrite_without_force() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        init_record(&record_path, &secret_path, "1234", false).unwrap();
        let err = init_record(&record_path, &secret_path, "9999", false).unwrap_err();
        assert!(matches!(err, InitError::RecordExists(_)));
        init_record(&record_path, &secret_path, "9999", true).unwrap();
    }

    #[test]
    fn device_secret_is_stable_across_reinit() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        init_record(&record_path, &secret_path, "1234", false).unwrap();
        let first = std::fs::read(&secret_path).unwrap();
        init_record(&record_path, &secret_path, "1234", true).unwrap();
        let second = std::fs::read(&secret_path).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn wrong_pin_rejected_at_unwrap() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, master_path) = setup(dir.path());
        fast_record(&record_path, &secret_path, "1234");
        let err =
            unwrap_to_file(&record_path, &secret_path, "0000", &master_path, false).unwrap_err();
        assert_eq!(err, UnwrapError::DecryptionFailed);
        assert!(!master_path.exists());
    }

    #[test]
    fn wrong_pin_cannot_decrypt_blob() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        let record = fast_record(&record_path, &secret_path, "1234");
        let secret = std::fs::read(&secret_path).unwrap();
        let master_ok = unwrap_master_key(&record, "1234", &secret).unwrap();
        let blob = encrypt_data(&master_ok, b"top secret").unwrap();

        let err = unwrap_master_key(&record, "0000", &secret).unwrap_err();
        assert_eq!(err, UnwrapError::DecryptionFailed);
        // A different master key must also fail to decrypt the blob.
        let other = [42u8; KEY_LEN];
        assert_eq!(
            decrypt_data(&other, &blob).unwrap_err(),
            UnwrapError::DecryptionFailed
        );
        assert_eq!(decrypt_data(&master_ok, &blob).unwrap(), b"top secret");
    }

    #[test]
    fn missing_device_secret_rejected() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        fast_record(&record_path, &secret_path, "1234");
        std::fs::remove_file(&secret_path).unwrap();
        let err = unwrap_to_file(&record_path, &secret_path, "1234", &dir.path().join("m"), false)
            .unwrap_err();
        assert_eq!(err, UnwrapError::MissingDeviceSecret);
    }

    #[test]
    fn swapped_device_secret_fingerprint_rejected() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        let record = fast_record(&record_path, &secret_path, "1234");
        let other_secret = vec![9u8; KEY_LEN];
        let err = unwrap_master_key(&record, "1234", &other_secret).unwrap_err();
        assert_eq!(err, UnwrapError::DeviceSecretMismatch);
    }

    #[test]
    fn rewrap_preserves_master_and_blobs_still_decrypt() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        let record = fast_record(&record_path, &secret_path, "1234");
        let secret = std::fs::read(&secret_path).unwrap();
        let master_before = unwrap_master_key(&record, "1234", &secret).unwrap();
        let blob = encrypt_data(&master_before, b"persistent").unwrap();

        let returned = rewrap_record(&record_path, &secret_path, "1234", "5678").unwrap();
        assert_eq!(returned.0, master_before);

        let reloaded = read_record(&record_path).unwrap();
        assert!(unwrap_master_key(&reloaded, "1234", &secret).is_err());
        let master_after = unwrap_master_key(&reloaded, "5678", &secret).unwrap();
        assert_eq!(master_after, master_before);
        assert_eq!(decrypt_data(&master_after, &blob).unwrap(), b"persistent");
    }

    #[test]
    fn tampered_ciphertext_rejected() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        let mut record = fast_record(&record_path, &secret_path, "1234");
        // Flip a hex digit inside the ciphertext only, keeping the JSON valid.
        let mut chars: Vec<char> = record.ciphertext.chars().collect();
        chars[0] = if chars[0] == 'a' { 'b' } else { 'a' };
        record.ciphertext = chars.into_iter().collect();
        let err =
            unwrap_master_key(&record, "1234", &std::fs::read(&secret_path).unwrap()).unwrap_err();
        assert_eq!(err, UnwrapError::DecryptionFailed);
    }

    #[test]
    fn bad_version_record_handled() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        let mut record = fast_record(&record_path, &secret_path, "1234");
        record.format_version = 99;
        let err =
            unwrap_master_key(&record, "1234", &std::fs::read(&secret_path).unwrap()).unwrap_err();
        assert!(matches!(err, UnwrapError::InvalidFormat(_)));
    }

    #[test]
    fn bad_salt_record_handled() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        let mut record = fast_record(&record_path, &secret_path, "1234");
        record.kdf_salt = "zzzz".to_string();
        let err =
            unwrap_master_key(&record, "1234", &std::fs::read(&secret_path).unwrap()).unwrap_err();
        assert!(matches!(err, UnwrapError::InvalidFormat(_)));

        record.kdf_salt = "00".to_string(); // valid hex but too short
        let err =
            unwrap_master_key(&record, "1234", &std::fs::read(&secret_path).unwrap()).unwrap_err();
        assert!(matches!(err, UnwrapError::InvalidFormat(_)));
    }

    #[test]
    fn bad_json_record_handled() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, _) = setup(dir.path());
        fast_record(&record_path, &secret_path, "1234");
        std::fs::write(&record_path, b"{ not json").unwrap();
        let err = read_record(&record_path).unwrap_err();
        assert!(matches!(err, UnwrapError::InvalidFormat(_)));
    }

    #[test]
    fn is_unlocked_tracks_files() {
        let dir = tempdir().unwrap();
        let (record_path, _, master_path) = setup(dir.path());
        assert!(!is_unlocked(&record_path, &master_path));
        std::fs::write(&record_path, "{}").unwrap();
        assert!(!is_unlocked(&record_path, &master_path));
        std::fs::write(&master_path, [0u8; KEY_LEN]).unwrap();
        assert!(is_unlocked(&record_path, &master_path));
    }

    #[test]
    fn hex_roundtrip() {
        let bytes = [0x00u8, 0x0f, 0xa5, 0xff];
        assert_eq!(hex_encode(&bytes), "000fa5ff");
        assert_eq!(hex_decode("000fa5ff").unwrap(), bytes);
        assert_eq!(hex_decode("00FA").unwrap(), vec![0x00, 0xfa]);
        assert!(hex_decode("0").is_err());
        assert!(hex_decode("gg").is_err());
    }

    #[test]
    fn fscrypt_derive_key_is_deterministic_and_sized() {
        let master = [1u8; KEY_LEN];
        let a = fscrypt::derive_fscrypt_key(&master);
        let b = fscrypt::derive_fscrypt_key(&master);
        assert_eq!(a, b);
        assert_eq!(a.len(), fscrypt::FSCRYPT_MAX_KEY_SIZE);
    }

    #[test]
    fn fscrypt_result_is_typed_never_panics() {
        let dir = tempdir().unwrap();
        let result = apply_fscrypt_policy(dir.path(), &[0u8; KEY_LEN]);
        // Either the filesystem applied it, or we got a non-empty reason; the
        // call must never panic.
        assert!(result.applied || !result.detail.is_empty());
    }

    #[cfg(not(target_os = "linux"))]
    #[test]
    fn fscrypt_unsupported_on_non_linux() {
        let dir = tempdir().unwrap();
        let result = apply_fscrypt_policy(dir.path(), &[0u8; KEY_LEN]);
        assert!(!result.applied, "fscrypt must be unsupported off Linux");
        assert!(
            result.detail.contains("unsupported") || result.detail.contains("not applied"),
            "unexpected reason: {}",
            result.detail
        );
    }

    #[cfg(unix)]
    #[test]
    fn record_and_device_secret_permissions() {
        let dir = tempdir().unwrap();
        let (record_path, secret_path, master_path) = setup(dir.path());
        init_record(&record_path, &secret_path, "1234", false).unwrap();
        unwrap_to_file(&record_path, &secret_path, "1234", &master_path, false).unwrap();

        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&secret_path), 0o400);
        assert_eq!(mode(&record_path), 0o600);
        assert_eq!(mode(&master_path), 0o600);
    }
}