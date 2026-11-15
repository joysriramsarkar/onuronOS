// pkg/nilpkg/src/keymgmt.rs — Publisher key management and private-key encryption.
//
// `keygen` writes the private key encrypted with a passphrase-derived key
// instead of in cleartext. `trust` and `revoke` manage the trust store.
//
// `enc2$` uses PBKDF2-HMAC-SHA256 (100,000 rounds, 16-byte random salt) to
// derive independent encryption and MAC keys, encrypts with an HMAC-SHA256
// counter keystream, and authenticates `(salt || ciphertext)` with a full
// 32-byte HMAC-SHA256 tag (Encrypt-then-MAC, constant-time verification).
// Legacy `enc1$` records remain readable for backwards compatibility.

use rand::rngs::OsRng;
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::Path;

const SALT_LEN: usize = 16;
const KEY_LEN: usize = 32;
const V1_KDF_ROUNDS: u32 = 10_000;
const V1_TAG_LEN: usize = 4;
const V2_KDF_ROUNDS: u32 = 100_000;
const V2_TAG_LEN: usize = 32;

fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    const BLOCK_SIZE: usize = 64;
    let mut k_block = [0u8; BLOCK_SIZE];
    if key.len() > BLOCK_SIZE {
        let hashed = Sha256::digest(key);
        k_block[..32].copy_from_slice(&hashed);
    } else {
        k_block[..key.len()].copy_from_slice(key);
    }
    let mut ipad = [0x36u8; BLOCK_SIZE];
    let mut opad = [0x5cu8; BLOCK_SIZE];
    for i in 0..BLOCK_SIZE {
        ipad[i] ^= k_block[i];
        opad[i] ^= k_block[i];
    }
    let mut inner = Sha256::new();
    inner.update(ipad);
    inner.update(data);
    let inner_digest = inner.finalize();

    let mut outer = Sha256::new();
    outer.update(opad);
    outer.update(inner_digest);
    outer.finalize().into()
}

fn pbkdf2_hmac_sha256(password: &[u8], salt: &[u8], rounds: u32, out: &mut [u8]) {
    let mut block_idx = 1u32;
    let mut offset = 0usize;
    while offset < out.len() {
        let mut init_data = Vec::with_capacity(salt.len() + 4);
        init_data.extend_from_slice(salt);
        init_data.extend_from_slice(&block_idx.to_be_bytes());
        let mut u = hmac_sha256(password, &init_data);
        let mut t = u;
        for _ in 1..rounds {
            u = hmac_sha256(password, &u);
            for (ti, ui) in t.iter_mut().zip(u.iter()) {
                *ti ^= *ui;
            }
        }
        let take = (out.len() - offset).min(32);
        out[offset..offset + take].copy_from_slice(&t[..take]);
        offset += take;
        block_idx += 1;
    }
}

fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

fn derive_v2_keys(passphrase: &str, salt: &[u8]) -> ([u8; KEY_LEN], [u8; KEY_LEN]) {
    let mut derived = [0u8; 64];
    pbkdf2_hmac_sha256(passphrase.as_bytes(), salt, V2_KDF_ROUNDS, &mut derived);
    let mut enc_key = [0u8; KEY_LEN];
    let mut mac_key = [0u8; KEY_LEN];
    enc_key.copy_from_slice(&derived[..KEY_LEN]);
    mac_key.copy_from_slice(&derived[KEY_LEN..]);
    (enc_key, mac_key)
}

fn v2_stream_xor(enc_key: &[u8; KEY_LEN], salt: &[u8], input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for (counter, chunk) in input.chunks(32).enumerate() {
        let mut label = Vec::with_capacity(20 + salt.len() + 4);
        label.extend_from_slice(b"nilpkg-key-stream-v2");
        label.extend_from_slice(salt);
        label.extend_from_slice(&(counter as u32).to_be_bytes());
        let pad = hmac_sha256(enc_key, &label);
        for (b, k) in chunk.iter().zip(pad.iter()) {
            out.push(b ^ k);
        }
    }
    out
}

fn v2_mac(mac_key: &[u8; KEY_LEN], salt: &[u8], ciphertext: &[u8]) -> [u8; V2_TAG_LEN] {
    let mut msg = Vec::with_capacity(4 + salt.len() + ciphertext.len());
    msg.extend_from_slice(b"enc2");
    msg.extend_from_slice(salt);
    msg.extend_from_slice(ciphertext);
    hmac_sha256(mac_key, &msg)
}

fn derive_key_v1(passphrase: &str, salt: &[u8]) -> [u8; KEY_LEN] {
    let mut hasher = Sha256::new();
    hasher.update(salt);
    hasher.update(passphrase.as_bytes());
    let mut key: [u8; KEY_LEN] = hasher.finalize().into();
    for _ in 1..V1_KDF_ROUNDS {
        key = Sha256::digest(key).into();
    }
    key
}

/// Authentication tag for a legacy `enc1$` plaintext key.
fn auth_tag_v1(plaintext: &[u8]) -> [u8; V1_TAG_LEN] {
    let digest = Sha256::digest(plaintext);
    let mut tag = [0u8; V1_TAG_LEN];
    tag.copy_from_slice(&digest[..V1_TAG_LEN]);
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
    if !s.is_ascii() || !s.len().is_multiple_of(2) {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

/// Encrypt a private key with a passphrase. Returns `enc2$<salt_hex>$<tag_hex>$<ciphertext_hex>`.
pub fn encrypt_private_key(private_key: &[u8], passphrase: &str) -> String {
    let mut salt = [0u8; SALT_LEN];
    OsRng.fill_bytes(&mut salt);
    let (enc_key, mac_key) = derive_v2_keys(passphrase, &salt);
    let ciphertext = v2_stream_xor(&enc_key, &salt, private_key);
    let tag = v2_mac(&mac_key, &salt, &ciphertext);
    format!(
        "enc2${}${}${}",
        hex_encode(&salt),
        hex_encode(&tag),
        hex_encode(&ciphertext)
    )
}

fn decrypt_v1(salt_hex: &str, tag_hex: &str, ct_hex: &str, passphrase: &str) -> Option<Vec<u8>> {
    let salt = hex_decode(salt_hex)?;
    let expected_tag = hex_decode(tag_hex)?;
    let ciphertext = hex_decode(ct_hex)?;
    if salt.len() != SALT_LEN || expected_tag.len() != V1_TAG_LEN {
        return None;
    }
    let key = derive_key_v1(passphrase, &salt);
    let plaintext: Vec<u8> = ciphertext
        .iter()
        .enumerate()
        .map(|(i, b)| b ^ key[i % KEY_LEN])
        .collect();
    // Ed25519 private keys are exactly 32 bytes.
    if plaintext.len() != KEY_LEN {
        return None;
    }
    let actual_tag = auth_tag_v1(&plaintext);
    if !ct_eq(&actual_tag, &expected_tag) {
        return None;
    }
    Some(plaintext)
}

fn decrypt_v2(salt_hex: &str, tag_hex: &str, ct_hex: &str, passphrase: &str) -> Option<Vec<u8>> {
    let salt = hex_decode(salt_hex)?;
    let expected_tag = hex_decode(tag_hex)?;
    let ciphertext = hex_decode(ct_hex)?;
    if salt.len() != SALT_LEN || expected_tag.len() != V2_TAG_LEN || ciphertext.len() != KEY_LEN {
        return None;
    }
    let (enc_key, mac_key) = derive_v2_keys(passphrase, &salt);
    let actual_tag = v2_mac(&mac_key, &salt, &ciphertext);
    if !ct_eq(&actual_tag, &expected_tag) {
        return None;
    }
    let plaintext = v2_stream_xor(&enc_key, &salt, &ciphertext);
    if plaintext.len() != KEY_LEN {
        return None;
    }
    Some(plaintext)
}

/// Decrypt a private key (`enc2$` or legacy `enc1$`). Returns None if the
/// format is wrong, the passphrase is incorrect, or the ciphertext/tag was tampered with.
pub fn decrypt_private_key(record: &str, passphrase: &str) -> Option<Vec<u8>> {
    let mut parts = record.split('$');
    let version = parts.next()?;
    let salt_hex = parts.next()?;
    let tag_hex = parts.next()?;
    let ct_hex = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    match version {
        "enc2" => decrypt_v2(salt_hex, tag_hex, ct_hex, passphrase),
        "enc1" => decrypt_v1(salt_hex, tag_hex, ct_hex, passphrase),
        _ => None,
    }
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
        let tty = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/tty");
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
        assert!(record.starts_with("enc2$"));
        assert!(
            !record.contains("4242"),
            "plaintext must not appear in record"
        );
        let decrypted = decrypt_private_key(&record, passphrase).unwrap();
        assert_eq!(decrypted, private_key);
    }

    #[test]
    fn legacy_v1_record_still_decrypts() {
        let private_key = [0x55u8; 32];
        let passphrase = "legacy-pass";
        let salt = [0x12u8; SALT_LEN];
        let key = derive_key_v1(passphrase, &salt);
        let tag = auth_tag_v1(&private_key);
        let ct: Vec<u8> = private_key
            .iter()
            .enumerate()
            .map(|(i, b)| b ^ key[i % KEY_LEN])
            .collect();
        let record = format!(
            "enc1${}${}${}",
            hex_encode(&salt),
            hex_encode(&tag),
            hex_encode(&ct)
        );
        assert_eq!(
            decrypt_private_key(&record, passphrase).unwrap(),
            private_key
        );
    }

    #[test]
    fn tampered_tag_is_rejected() {
        let private_key = [0x77u8; 32];
        let record = encrypt_private_key(&private_key, "pass");
        let mut parts: Vec<String> = record.split('$').map(|s| s.to_string()).collect();
        let mut tag_chars: Vec<char> = parts[2].chars().collect();
        tag_chars[0] = if tag_chars[0] == '0' { '1' } else { '0' };
        parts[2] = tag_chars.into_iter().collect();
        let tampered = parts.join("$");
        assert!(decrypt_private_key(&tampered, "pass").is_none());
    }

    #[test]
    fn rfc4231_and_rfc6070_vectors_match() {
        // RFC 4231 Test Case 2: key = "Jefe", data = "what do ya want for nothing?"
        let mac = hmac_sha256(b"Jefe", b"what do ya want for nothing?");
        assert_eq!(
            hex_encode(&mac),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
        // RFC 6070 / PBKDF2-HMAC-SHA256 vector (password="password", salt="salt", c=1, dkLen=32)
        let mut dk = [0u8; 32];
        pbkdf2_hmac_sha256(b"password", b"salt", 1, &mut dk);
        assert_eq!(
            hex_encode(&dk),
            "120fb6cffcf8b32c43e7225256c4f837a86548c92ccc35480805987cb70be17b"
        );
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
            assert!(
                decrypt_private_key(bad, "pass").is_none(),
                "accepted {bad:?}"
            );
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
