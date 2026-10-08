// pkg/nilpkg/src/keymgmt.rs — Publisher key management and private-key encryption.
//
// `keygen` writes the private key encrypted with a passphrase-derived key
// instead of in cleartext. `trust` and `revoke` manage the trust store.
//
// The encryption here is a simple XOR cipher with a SHA-256-derived key and
// random salt. It is NOT production-grade authenticated encryption — it prevents
// casual exposure of the key at rest but does not resist a determined attacker.
// Production deployments should replace this with age or NaCl secretbox.

use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const KDF_ROUNDS: u32 = 10_000;
/// Tag length: first 4 bytes of SHA-256(plaintext), stored to detect wrong
/// passphrases and tampering. XOR alone cannot do this.
const TAG_LEN: usize = 4;

fn derive_key(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(passphrase.as_bytes());
    let mut key: [u8; KEY_LEN] = hasher.finalize().into();
    for _ in 1..KDF_ROUNDS {
        key = Sha256::digest(key).into();
    }
    key
}

/// Authentication tag for a plaintext key.
fn auth_tag(plaintext: &[u8]) -> [u8; TAG_LEN] {
    let digest = Sha256::digest(plaintext);
    let mut tag = [0u8; TAG_LEN];
    tag.copy_from_slice(&digest[..TAG_LEN]);
    tag
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

/// Encrypt a private key with a passphrase. Returns `enc1$<salt_hex>$<tag_hex>$<ciphertext_hex>`.
pub fn encrypt_private_key(private_key: &[u8], passphrase: &str) -> String {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let key = derive_key(passphrase, &salt);
    let tag = auth_tag(private_key);
    let ciphertext: Vec<u8> = private_key
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % KEY_LEN])
        .collect();
    format!(
        "enc1${}${}${}",
        hex_encode(&salt),
        hex_encode(&tag),
        hex_encode(&ciphertext)
    )
}

/// Decrypt a private key. Returns None if the format is wrong, the passphrase
/// is incorrect, or the ciphertext was tampered with (detected via the tag).
pub fn decrypt_private_key(record: &str, passphrase: &str) -> Option<Vec<u8>> {
    let mut parts = record.split('$');
    let version = parts.next()?;
    if version != "enc1" {
        return None;
    }
    let salt_hex = parts.next()?;
    let tag_hex = parts.next()?;
    let ct_hex = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let salt = hex_decode(salt_hex)?;
    let expected_tag = hex_decode(tag_hex)?;
    let ciphertext = hex_decode(ct_hex)?;
    if salt.len() != SALT_LEN || expected_tag.len() != TAG_LEN {
        return None;
    }
    let key = derive_key(passphrase, &salt);
    let plaintext: Vec<u8> = ciphertext
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % KEY_LEN])
        .collect();
    // Ed25519 private keys are exactly 32 bytes.
    if plaintext.len() != KEY_LEN {
        return None;
    }
    // Verify the authentication tag: a wrong passphrase or tampered ciphertext
    // will (with overwhelming probability) produce a mismatched tag.
    let actual_tag = auth_tag(&plaintext);
    if actual_tag.as_slice() != expected_tag.as_slice() {
        return None;
    }
    Some(plaintext)
}

/// Write an encrypted private key file with owner-only permissions on Unix.
pub fn write_encrypted_key(path: &Path, record: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Could not create key dir: {e}"))?;
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = fs::OpenOptions::new();
        options.write(true).create(true).truncate(true).mode(0o600);
        let mut file = options
            .open(path)
            .map_err(|e| format!("Could not write private key: {e}"))?;
        file.write_all(record.as_bytes())
            .map_err(|e| format!("Could not write private key: {e}"))?;
        return Ok(());
    }
    #[cfg(not(unix))]
    {
        fs::write(path, record).map_err(|e| format!("Could not write private key: {e}"))?;
        Ok(())
    }
}

/// Read a passphrase from the terminal without echoing it.
/// Falls back to stdin if /dev/tty is unavailable.
pub fn read_passphrase(prompt: &str) -> Result<String, String> {
    print!("{prompt}");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        let tty = fs::OpenOptions::new().read(true).write(true).open("/dev/tty");
        if let Ok(tty) = tty {
            let fd = tty.as_raw_fd();
            let mut termios: libc::termios = unsafe { std::mem::zeroed() };
            if unsafe { libc::tcgetattr(fd, &mut termios) } == 0 {
                let original = termios;
                termios.c_lflag &= !libc::ECHO;
                unsafe { libc::tcsetattr(fd, libc::TCSANOW, &termios) };
                let mut line = String::new();
                let result = std::io::stdin().read_line(&mut line);
                unsafe { libc::tcsetattr(fd, libc::TCSANOW, &original) };
                println!();
                result.map_err(|e| e.to_string())?;
                return Ok(line.trim().to_string());
            }
        }
    }
    let mut line = String::new();
    std::io::stdin()
        .read_line(&mut line)
        .map_err(|e| e.to_string())?;
    Ok(line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encrypt_decrypt_roundtrip() {
        let private_key = [0x42u8; 32];
        let passphrase = "correct horse battery staple";
        let record = encrypt_private_key(&private_key, passphrase);
        assert!(record.starts_with("enc1$"));
        assert!(!record.contains("4242"), "plaintext must not appear in record");
        let decrypted = decrypt_private_key(&record, passphrase).unwrap();
        assert_eq!(decrypted, private_key);
    }

    #[test]
    fn wrong_passphrase_is_rejected() {
        let private_key = [0xABu8; 32];
        let record = encrypt_private_key(&private_key, "right");
        assert!(decrypt_private_key(&record, "wrong").is_none());
    }

    #[test]
    fn tampered_record_is_rejected() {
        let private_key = [0xCDu8; 32];
        let record = encrypt_private_key(&private_key, "pass");
        // Flip a character in the ciphertext.
        let mut chars: Vec<char> = record.chars().collect();
        let last = chars.len() - 1;
        chars[last] = if chars[last] == 'a' { 'b' } else { 'a' };
        let tampered: String = chars.into_iter().collect();
        assert!(decrypt_private_key(&tampered, "pass").is_none());
    }

    #[test]
    fn invalid_formats_are_rejected() {
        for bad in ["", "enc1", "enc1$a$b$c", "enc2$aa$bb", "enc1$zz$bb"] {
            assert!(decrypt_private_key(bad, "pass").is_none(), "accepted {bad:?}");
        }
    }

    #[test]
    fn salts_are_unique() {
        let key = [0x11u8; 32];
        let a = encrypt_private_key(&key, "same");
        let b = encrypt_private_key(&key, "same");
        assert_ne!(a, b, "each encryption must use a fresh salt");
    }
}
