// shell/src/pin.rs — Device PIN storage and verification.
//
// The lockscreen PIN was previously written to disk in plaintext and compared
// with `==`. This module stores a salted, stretched SHA-256 record instead,
// compares hashes in constant time, and transparently migrates legacy
// plaintext files on the next successful unlock.

use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};

pub const PIN_MIN_LEN: usize = 4;
pub const PIN_MAX_LEN: usize = 8;
pub const PIN_ITERATIONS: u32 = 100_000;
pub const PIN_SALT_LEN: usize = 16;

#[derive(Debug, PartialEq)]
pub enum CheckResult {
    Verified,
    /// The candidate matched a legacy plaintext file. The caller should
    /// replace the file with `create_record(candidate)`.
    VerifiedNeedsMigration,
    Rejected,
}

/// A PIN is 4–8 ASCII digits. Length alone is not security, but it is the
/// device's documented policy and is enforced here as well as in the UI.
pub fn valid_pin(pin: &str) -> bool {
    (PIN_MIN_LEN..=PIN_MAX_LEN).contains(&pin.len())
        && !pin.is_empty()
        && pin.bytes().all(|b| b.is_ascii_digit())
}

fn hash_pin(pin: &str, salt: &[u8], iterations: u32) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(salt);
    digest.update(pin.as_bytes());
    let mut out: [u8; 32] = digest.finalize().into();
    for _ in 1..iterations.max(1) {
        out = Sha256::digest(out).into();
    }
    out
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{:02x}", b));
    }
    s
}

fn hex_decode(s: &str) -> Option<Vec<u8>> {
    if !s.is_ascii() || s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

/// Create a new `v1$iterations$salt$hash` record. The plaintext PIN never
/// appears in the output.
pub fn create_record(pin: &str) -> Result<String, String> {
    if !valid_pin(pin) {
        return Err("PIN must be 4–8 ASCII digits".into());
    }
    let mut salt = [0u8; PIN_SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let hash = hash_pin(pin, &salt, PIN_ITERATIONS);
    Ok(format!(
        "v1${}${}${}",
        PIN_ITERATIONS,
        hex_encode(&salt),
        hex_encode(&hash)
    ))
}

/// Check a candidate PIN against the stored file content.
pub fn check_pin(stored: &str, candidate: &str) -> CheckResult {
    let stored = stored.trim();
    let candidate = candidate.trim();
    if stored.is_empty() || candidate.is_empty() {
        return CheckResult::Rejected;
    }
    let mut parts = stored.split('$');
    match (parts.next(), parts.next(), parts.next(), parts.next(), parts.next()) {
        (Some("v1"), Some(iter), Some(salt_hex), Some(hash_hex), None) => {
            let iterations: u32 = match iter.parse() {
                Ok(n) if (1..=10_000_000).contains(&n) => n,
                _ => return CheckResult::Rejected,
            };
            let (Some(salt), Some(expected)) = (hex_decode(salt_hex), hex_decode(hash_hex)) else {
                return CheckResult::Rejected;
            };
            if salt.len() != PIN_SALT_LEN || expected.len() != 32 {
                return CheckResult::Rejected;
            }
            let actual = hash_pin(candidate, &salt, iterations);
            if constant_time_eq(&actual, &expected) {
                CheckResult::Verified
            } else {
                CheckResult::Rejected
            }
        }
        // Legacy plaintext file: accept once so the device is not bricked,
        // but force the caller to re-hash immediately.
        _ if valid_pin(stored) && constant_time_eq(stored.as_bytes(), candidate.as_bytes()) => {
            CheckResult::VerifiedNeedsMigration
        }
        _ => CheckResult::Rejected,
    }
}

/// Write the PIN record with owner-only permissions on Unix.
pub fn write_pin_file(path: &str, record: &str) -> Result<(), String> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent).map_err(|e| format!("Could not create PIN dir: {e}"))?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).mode(0o600);
        use std::io::Write;
        let mut file = options.open(path).map_err(|e| format!("Could not write PIN file: {e}"))?;
        file.write_all(record.as_bytes())
            .map_err(|e| format!("Could not write PIN file: {e}"))?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        std::fs::write(path, record).map_err(|e| format!("Could not write PIN file: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_pin_enforces_digit_policy() {
        assert!(valid_pin("1234"));
        assert!(valid_pin("12345678"));
        assert!(!valid_pin("123"));
        assert!(!valid_pin("123456789"));
        assert!(!valid_pin("12a4"));
        assert!(!valid_pin(""));
        assert!(!valid_pin(" 1234"));
    }

    #[test]
    fn create_and_verify_roundtrip() {
        let record = create_record("4826").unwrap();
        assert!(!record.split('$').any(|part| part == "4826"), "plaintext PIN must not appear as a field in the record");
        assert!(record.starts_with("v1$"));
        assert_eq!(check_pin(&record, "4826"), CheckResult::Verified);
        assert_eq!(check_pin(&record, "4827"), CheckResult::Rejected);
        assert_eq!(check_pin(&record, ""), CheckResult::Rejected);
    }

    #[test]
    fn salts_are_unique_so_records_differ() {
        let a = create_record("1111").unwrap();
        let b = create_record("1111").unwrap();
        assert_ne!(a, b);
        assert_eq!(check_pin(&a, "1111"), CheckResult::Verified);
        assert_eq!(check_pin(&b, "1111"), CheckResult::Verified);
    }

    #[test]
    fn tampered_records_are_rejected() {
        let record = create_record("9999").unwrap();
        let mut parts: Vec<String> = record.split('$').map(|s| s.to_string()).collect();
        // Flip a salt character.
        let mut salt: Vec<char> = parts[2].chars().collect();
        salt[0] = if salt[0] == 'a' { 'b' } else { 'a' };
        parts[2] = String::from_iter(salt);
        let tampered = parts.join("$");
        assert_eq!(check_pin(&tampered, "9999"), CheckResult::Rejected);
        // Garbage is rejected, never panics.
        for bad in ["", "v1", "v1$x$y$z$extra", "v1$abc$00$00", "v1$0$00$00"] {
            assert_eq!(check_pin(bad, "9999"), CheckResult::Rejected, "accepted {bad:?}");
        }
    }

    #[test]
    fn legacy_plaintext_file_migrates_on_next_unlock() {
        assert_eq!(check_pin("1234", "1234"), CheckResult::VerifiedNeedsMigration);
        assert_eq!(check_pin("1234", "0000"), CheckResult::Rejected);
        // The migrated record verifies normally.
        let migrated = create_record("1234").unwrap();
        assert_eq!(check_pin(&migrated, "1234"), CheckResult::Verified);
    }

    #[test]
    fn invalid_pins_cannot_create_records() {
        for bad in ["", "123", "123456789", "abcd", "12 4"] {
            assert!(create_record(bad).is_err(), "created record for {bad:?}");
        }
    }
}
