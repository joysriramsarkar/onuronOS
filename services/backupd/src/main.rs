// services/backupd/src/main.rs — Authenticated Encrypted Backup & Restore Daemon (backupd)
// Manages system snapshot creation, package state backup, encrypted .nilbak container
// packaging with authenticated integrity protection, and restore flows.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const BACKUP_SOCK_PATH: &str = "/run/nilos/backup.sock";
pub const BACKUP_STORAGE_DIR: &str = "/data/backup";
pub const NILBAK_MAGIC: &[u8; 8] = b"NILBAK1\0";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BackupManifest {
    pub backup_id: String,
    pub timestamp: u64,
    pub os_version: String,
    pub app_ids: Vec<String>,
    pub payload_size_bytes: u64,
    pub checksum_sha256: String,
    pub encrypted: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BackupStatus {
    pub last_backup_id: Option<String>,
    pub last_backup_timestamp: Option<u64>,
    pub total_backups_count: usize,
    pub backup_storage_used_bytes: u64,
}

/// Computes SHA-256 hash using RFC 6234 standard algorithm
pub fn compute_sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
        0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];

    let k: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    let bit_len = (data.len() as u64) * 8;
    let mut msg = data.to_vec();
    msg.push(0x80);
    while (msg.len() % 64) != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&bit_len.to_be_bytes());

    for chunk in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, w_chunk) in chunk.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes(w_chunk.try_into().unwrap());
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16].wrapping_add(s0).wrapping_add(w[i - 7]).wrapping_add(s1);
        }

        let mut a = h[0];
        let mut b = h[1];
        let mut c = h[2];
        let mut d = h[3];
        let mut e = h[4];
        let mut f = h[5];
        let mut g = h[6];
        let mut hh = h[7];

        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1).wrapping_add(ch).wrapping_add(k[i]).wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);

            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }

        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }

    let mut out = [0u8; 32];
    for (i, val) in h.iter().enumerate() {
        out[i * 4..(i + 1) * 4].copy_from_slice(&val.to_be_bytes());
    }
    out
}

pub fn sha256_hex(data: &[u8]) -> String {
    let hash = compute_sha256(data);
    let mut hex = String::with_capacity(64);
    for b in hash {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

/// Generates authenticated ciphertext and 16-byte MAC authentication tag
pub fn encrypt_and_authenticate(payload: &[u8], key: &[u8; 32], nonce: &[u8; 12]) -> (Vec<u8>, [u8; 16]) {
    // Generate pseudorandom keystream from key + nonce
    let mut ciphertext = Vec::with_capacity(payload.len());
    let mut seed = [0u8; 44];
    seed[..32].copy_from_slice(key);
    seed[32..44].copy_from_slice(nonce);

    let mut current_hash = compute_sha256(&seed);
    for (i, &byte) in payload.iter().enumerate() {
        if i > 0 && i % 32 == 0 {
            current_hash = compute_sha256(&current_hash);
        }
        let stream_byte = current_hash[i % 32];
        ciphertext.push(byte ^ stream_byte);
    }

    // Compute HMAC-like tag over (nonce + ciphertext)
    let mut auth_input = Vec::new();
    auth_input.extend_from_slice(nonce);
    auth_input.extend_from_slice(&ciphertext);
    auth_input.extend_from_slice(key);

    let tag_full = compute_sha256(&auth_input);
    let mut tag = [0u8; 16];
    tag.copy_from_slice(&tag_full[..16]);

    (ciphertext, tag)
}

/// Verifies authentication tag and decrypts payload
pub fn verify_and_decrypt(
    ciphertext: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
    expected_tag: &[u8; 16],
) -> Result<Vec<u8>, String> {
    let mut auth_input = Vec::new();
    auth_input.extend_from_slice(nonce);
    auth_input.extend_from_slice(ciphertext);
    auth_input.extend_from_slice(key);

    let tag_full = compute_sha256(&auth_input);
    if &tag_full[..16] != expected_tag {
        return Err("Authentication tag verification failed: corrupted data or wrong key".to_string());
    }

    // Decrypt keystream (XOR is symmetric)
    let mut plaintext = Vec::with_capacity(ciphertext.len());
    let mut seed = [0u8; 44];
    seed[..32].copy_from_slice(key);
    seed[32..44].copy_from_slice(nonce);

    let mut current_hash = compute_sha256(&seed);
    for (i, &byte) in ciphertext.iter().enumerate() {
        if i > 0 && i % 32 == 0 {
            current_hash = compute_sha256(&current_hash);
        }
        let stream_byte = current_hash[i % 32];
        plaintext.push(byte ^ stream_byte);
    }

    Ok(plaintext)
}

/// Packages a complete `.nilbak` archive with authenticated encryption
pub fn package_nilbak(
    manifest: &BackupManifest,
    payload: &[u8],
    key: &[u8; 32],
    nonce: &[u8; 12],
) -> Result<Vec<u8>, String> {
    let (ciphertext, tag) = encrypt_and_authenticate(payload, key, nonce);

    let manifest_json = serde_json::to_string(manifest).map_err(|e| e.to_string())?;
    let manifest_bytes = manifest_json.as_bytes();
    let manifest_len = manifest_bytes.len() as u32;

    let mut archive = Vec::new();
    archive.extend_from_slice(NILBAK_MAGIC);      // 8 bytes
    archive.extend_from_slice(nonce);             // 12 bytes
    archive.extend_from_slice(&tag);              // 16 bytes
    archive.extend_from_slice(&manifest_len.to_le_bytes()); // 4 bytes
    archive.extend_from_slice(manifest_bytes);    // Manifest JSON
    archive.extend_from_slice(&ciphertext);       // Encrypted payload

    Ok(archive)
}

/// Verifies and unpacks a `.nilbak` archive
pub fn unpack_nilbak(archive: &[u8], key: &[u8; 32]) -> Result<(BackupManifest, Vec<u8>), String> {
    if archive.len() < 8 + 12 + 16 + 4 {
        return Err("Archive too short for .nilbak container".to_string());
    }

    if &archive[..8] != NILBAK_MAGIC {
        return Err("Invalid .nilbak magic header".to_string());
    }

    let mut nonce = [0u8; 12];
    nonce.copy_from_slice(&archive[8..20]);

    let mut tag = [0u8; 16];
    tag.copy_from_slice(&archive[20..36]);

    let mut len_bytes = [0u8; 4];
    len_bytes.copy_from_slice(&archive[36..40]);
    let manifest_len = u32::from_le_bytes(len_bytes) as usize;

    if archive.len() < 40 + manifest_len {
        return Err("Archive truncated before manifest completion".to_string());
    }

    let manifest_json_bytes = &archive[40..40 + manifest_len];
    let manifest: BackupManifest = serde_json::from_slice(manifest_json_bytes)
        .map_err(|e| format!("Invalid manifest JSON: {}", e))?;

    let ciphertext = &archive[40 + manifest_len..];
    let plaintext = verify_and_decrypt(ciphertext, key, &nonce, &tag)?;

    // Verify SHA-256 matches manifest
    let actual_hash = sha256_hex(&plaintext);
    if actual_hash != manifest.checksum_sha256 {
        return Err(format!(
            "Checksum mismatch: expected {}, got {}",
            manifest.checksum_sha256, actual_hash
        ));
    }

    Ok((manifest, plaintext))
}

pub struct BackupServiceState {
    pub last_backup: Option<BackupManifest>,
    pub storage_dir: PathBuf,
}

pub struct BackupService {
    pub state: Arc<Mutex<BackupServiceState>>,
}

impl BackupService {
    pub fn new() -> Self {
        let storage_dir = PathBuf::from(BACKUP_STORAGE_DIR);
        Self {
            state: Arc::new(Mutex::new(BackupServiceState {
                last_backup: None,
                storage_dir,
            })),
        }
    }

    pub fn create_backup(
        &self,
        app_ids: Vec<String>,
        payload_data: &[u8],
        key: &[u8; 32],
    ) -> Result<PathBuf, String> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let backup_id = format!("bak_{}", timestamp);
        let checksum = sha256_hex(payload_data);

        let manifest = BackupManifest {
            backup_id: backup_id.clone(),
            timestamp,
            os_version: "OnuronOS 1.0".to_string(),
            app_ids,
            payload_size_bytes: payload_data.len() as u64,
            checksum_sha256: checksum,
            encrypted: true,
        };

        let nonce = [0x5au8; 12];
        let archive = package_nilbak(&manifest, payload_data, key, &nonce)?;

        let mut st = self.state.lock().unwrap();
        let _ = fs::create_dir_all(&st.storage_dir);
        let out_file = st.storage_dir.join(format!("{}.nilbak", backup_id));
        fs::write(&out_file, archive).map_err(|e| e.to_string())?;

        st.last_backup = Some(manifest);
        println!("[backupd] Backup created successfully: {:?}", out_file);
        Ok(out_file)
    }

    pub fn get_status(&self) -> BackupStatus {
        let st = self.state.lock().unwrap();
        BackupStatus {
            last_backup_id: st.last_backup.as_ref().map(|m| m.backup_id.clone()),
            last_backup_timestamp: st.last_backup.as_ref().map(|m| m.timestamp),
            total_backups_count: if st.last_backup.is_some() { 1 } else { 0 },
            backup_storage_used_bytes: st.last_backup.as_ref().map(|m| m.payload_size_bytes).unwrap_or(0),
        }
    }
}

fn main() {
    println!("[backupd] Authenticated Encrypted Backup & Restore Daemon starting...");
    let service = BackupService::new();

    let status = service.get_status();
    println!("[backupd] Initial Status: total_backups={}", status.total_backups_count);

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha256_known_vector() {
        assert_eq!(
            sha256_hex(b"hello world"),
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
    }

    #[test]
    fn test_nilbak_package_and_unpack_roundtrip() {
        let key = [0x42u8; 32];
        let nonce = [0x12u8; 12];
        let payload = b"user_data_contacts_and_sms_database_content_2026";

        let manifest = BackupManifest {
            backup_id: "bak_test_100".to_string(),
            timestamp: 1792800000,
            os_version: "OnuronOS 1.0".to_string(),
            app_ids: vec!["org.onuron.phone".to_string(), "org.onuron.messages".to_string()],
            payload_size_bytes: payload.len() as u64,
            checksum_sha256: sha256_hex(payload),
            encrypted: true,
        };

        let archive = package_nilbak(&manifest, payload, &key, &nonce).expect("Package");
        assert!(archive.len() > payload.len());

        // Unpack with correct key
        let (unpacked_manifest, unpacked_payload) = unpack_nilbak(&archive, &key).expect("Unpack");
        assert_eq!(unpacked_manifest, manifest);
        assert_eq!(unpacked_payload, payload);

        // Fail with wrong key
        let wrong_key = [0x99u8; 32];
        assert!(unpack_nilbak(&archive, &wrong_key).is_err());
    }

    #[test]
    fn test_tamper_detection() {
        let key = [0x33u8; 32];
        let nonce = [0x77u8; 12];
        let payload = b"critical system settings";

        let manifest = BackupManifest {
            backup_id: "bak_tamper".to_string(),
            timestamp: 1000,
            os_version: "OnuronOS 1.0".to_string(),
            app_ids: vec![],
            payload_size_bytes: payload.len() as u64,
            checksum_sha256: sha256_hex(payload),
            encrypted: true,
        };

        let mut archive = package_nilbak(&manifest, payload, &key, &nonce).expect("Package");

        // Tamper with one byte in the ciphertext
        let last_idx = archive.len() - 1;
        archive[last_idx] ^= 0xff;

        let result = unpack_nilbak(&archive, &key);
        assert!(result.is_err());
    }
}
