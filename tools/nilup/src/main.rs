// tools/nilup/src/main.rs — Onuron OS OTA System Update & Root Key Authority CLI

use std::env;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use nilpkg::{generate_keypair, hex};
use nilupd::{sign_update, verify_image, UpdateManifest};
use sha2::{Digest, Sha256};

fn print_help() {
    println!("=========================================================");
    println!("     Onuron OS Release Authority & Update Tool (nilup)    ");
    println!("=========================================================");
    println!("Usage:");
    println!("  nilup keygen <out_key_path>");
    println!("        Generate an Ed25519 signing keypair for OS releases.");
    println!("  nilup sign <image_path> <out_update_dir> --key <key_file>");
    println!("             [--name <name>] [--version <ver>] [--target-slot <A|B>]");
    println!("             [--current-hash <sha256>]");
    println!("        Sign an OS system image and produce a verified A/B update directory.");
    println!("  nilup verify <update_dir> --key-dir <trusted_key_dir>");
    println!("        Verify an update directory (image + update.json) against trusted publisher keys.");
    println!("  nilup info <update_dir_or_json>");
    println!("        Display parsed update manifest information.");
    println!("=========================================================");
}

fn file_sha256(path: &Path) -> Result<String, String> {
    let mut file = fs::File::open(path)
        .map_err(|e| format!("Could not open {}: {e}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| format!("Read error: {e}"))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn cmd_keygen(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: nilup keygen <out_key_path>".into());
    }
    let key_path = Path::new(&args[0]);
    let pub_path = key_path.with_extension("pub");

    let (signing_key, verifying_key) = generate_keypair();
    let priv_hex = hex::encode(signing_key.to_bytes());
    let pub_hex = hex::encode(verifying_key.to_bytes());

    if let Some(parent) = key_path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("Could not create directory {}: {e}", parent.display()))?;
    }

    fs::write(key_path, format!("{}\n", priv_hex))
        .map_err(|e| format!("Could not write private key to {}: {e}", key_path.display()))?;
    fs::write(&pub_path, format!("{}\n", pub_hex))
        .map_err(|e| format!("Could not write public key to {}: {e}", pub_path.display()))?;

    println!("[nilup] Successfully generated release keypair:");
    println!("  Private key: {}", key_path.display());
    println!("  Public key:  {}", pub_path.display());
    println!("  Public Key Hex: {}", pub_hex);
    Ok(())
}

fn cmd_sign(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        return Err("Usage: nilup sign <image_path> <out_update_dir> --key <key_file> [--name <name>] [--version <ver>] [--target-slot <slot>] [--current-hash <hash>]".into());
    }
    let image_path = PathBuf::from(&args[0]);
    let update_dir = PathBuf::from(&args[1]);

    let mut key_file: Option<PathBuf> = None;
    let mut name = "OnuronOS System Update".to_string();
    let mut version = "1.0.0".to_string();
    let mut target_slot = "B".to_string();
    let mut current_hash = String::new();

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--key" => {
                i += 1;
                key_file = args.get(i).map(PathBuf::from);
            }
            "--name" => {
                i += 1;
                if let Some(n) = args.get(i) {
                    name = n.clone();
                }
            }
            "--version" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    version = v.clone();
                }
            }
            "--target-slot" => {
                i += 1;
                if let Some(s) = args.get(i) {
                    target_slot = s.to_uppercase();
                }
            }
            "--current-hash" => {
                i += 1;
                if let Some(h) = args.get(i) {
                    current_hash = h.clone();
                }
            }
            other => return Err(format!("Unknown option '{other}'")),
        }
        i += 1;
    }

    let key_path = key_file.ok_or("Missing required --key option")?;
    let key_text = fs::read_to_string(&key_path)
        .map_err(|e| format!("Could not read key {}: {e}", key_path.display()))?;
    let raw_key = hex::decode(key_text.trim())
        .map_err(|e| format!("Invalid hex private key: {e}"))?;
    let key_bytes: [u8; 32] = raw_key
        .try_into()
        .map_err(|_| "Private key must be exactly 32 bytes (64 hex characters)")?;
    let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);

    if !image_path.is_file() {
        return Err(format!("Image file does not exist: {}", image_path.display()));
    }

    fs::create_dir_all(&update_dir)
        .map_err(|e| format!("Could not create update directory {}: {e}", update_dir.display()))?;

    let dest_image = update_dir.join("image");
    if image_path != dest_image {
        fs::copy(&image_path, &dest_image)
            .map_err(|e| format!("Could not copy image to {}: {e}", dest_image.display()))?;
    }

    let meta = fs::metadata(&dest_image)
        .map_err(|e| format!("Could not stat image: {e}"))?;
    let image_size = meta.len();

    println!("[nilup] Computing SHA-256 for {} ({} bytes)...", dest_image.display(), image_size);
    let image_sha256 = file_sha256(&dest_image)?;

    let mut manifest = UpdateManifest {
        name,
        version,
        target_slot,
        current_image_sha256: current_hash,
        image_sha256,
        image_size,
        signature_hex: None,
        public_key_hex: None,
    };

    sign_update(&mut manifest, &signing_key);

    let manifest_path = update_dir.join("update.json");
    let json_text = serde_json::to_string_pretty(&manifest)
        .map_err(|e| format!("Failed to serialize manifest: {e}"))?;
    fs::write(&manifest_path, json_text)
        .map_err(|e| format!("Could not write manifest to {}: {e}", manifest_path.display()))?;

    println!("[nilup] Successfully packaged and signed update directory -> {}", update_dir.display());
    if let Some(sig) = &manifest.signature_hex {
        println!("  Signature:  {}", sig);
    }
    if let Some(pubk) = &manifest.public_key_hex {
        println!("  Publisher:  {}", pubk);
    }
    Ok(())
}

fn cmd_verify(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: nilup verify <update_dir> --key-dir <trusted_key_dir>".into());
    }
    let update_dir = PathBuf::from(&args[0]);

    let mut key_dir: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--key-dir" => {
                i += 1;
                key_dir = args.get(i).map(PathBuf::from);
            }
            other => return Err(format!("Unknown option '{other}'")),
        }
        i += 1;
    }

    let key_dir = key_dir.ok_or("Missing required --key-dir option")?;
    let manifest = verify_image(&update_dir, &key_dir)?;

    println!("[nilup] Verification SUCCESSFUL:");
    println!("  Directory: {}", update_dir.display());
    println!("  Version:   {}", manifest.version);
    println!("  Target:    Slot {}", manifest.target_slot);
    println!("  SHA-256:   {}", manifest.image_sha256);
    if let Some(pubk) = &manifest.public_key_hex {
        println!("  Publisher: {}", pubk);
    }
    Ok(())
}

fn cmd_info(args: &[String]) -> Result<(), String> {
    if args.is_empty() {
        return Err("Usage: nilup info <update_dir_or_json>".into());
    }
    let path = Path::new(&args[0]);
    let manifest_path = if path.is_dir() {
        path.join("update.json")
    } else {
        path.to_path_buf()
    };

    let text = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("Could not read manifest {}: {e}", manifest_path.display()))?;
    let manifest: UpdateManifest = serde_json::from_str(&text)
        .map_err(|e| format!("Malformed manifest JSON: {e}"))?;

    println!("=========================================================");
    println!("Update Manifest: {}", manifest.name);
    println!("=========================================================");
    println!("Version:          {}", manifest.version);
    println!("Target Slot:      {}", manifest.target_slot);
    println!("Image Size:       {} bytes", manifest.image_size);
    println!("Image SHA-256:    {}", manifest.image_sha256);
    println!("Base SHA-256:     {}", if manifest.current_image_sha256.is_empty() { "(unrestricted / full image)" } else { &manifest.current_image_sha256 });
    println!("Publisher PubKey: {}", manifest.public_key_hex.as_deref().unwrap_or("none"));
    println!("Signature:        {}", manifest.signature_hex.as_deref().unwrap_or("none"));
    println!("=========================================================");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "help" || args[1] == "--help" {
        print_help();
        return ExitCode::SUCCESS;
    }

    let res = match args[1].as_str() {
        "keygen" => cmd_keygen(&args[2..]),
        "sign" => cmd_sign(&args[2..]),
        "verify" => cmd_verify(&args[2..]),
        "info" => cmd_info(&args[2..]),
        cmd => Err(format!("Unknown command '{cmd}'. Run 'nilup --help' for usage.")),
    };

    match res {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[ERROR] {e}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keygen_sign_verify_roundtrip() {
        let temp = std::env::temp_dir().join(format!(
            "nilup-test-{:x}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let key_path = temp.join("release.key");
        let image_path = temp.join("source.img");
        let update_dir = temp.join("update_pkg");
        let key_dir = temp.join("keys");
        let trusted_dir = key_dir.join("trusted");

        fs::create_dir_all(&trusted_dir).unwrap();
        fs::write(&image_path, b"ONURON_OS_SYSTEM_IMAGE_PAYLOAD_V1").unwrap();

        // 1. Keygen
        cmd_keygen(&[key_path.to_string_lossy().to_string()]).unwrap();
        assert!(key_path.is_file());
        let pub_key_path = key_path.with_extension("pub");
        assert!(pub_key_path.is_file());

        // Copy pub key to trusted key dir
        let pub_hex = fs::read_to_string(&pub_key_path).unwrap();
        fs::write(trusted_dir.join("release.pub"), pub_hex.trim()).unwrap();

        // 2. Sign
        cmd_sign(&[
            image_path.to_string_lossy().to_string(),
            update_dir.to_string_lossy().to_string(),
            "--key".to_string(),
            key_path.to_string_lossy().to_string(),
            "--name".to_string(),
            "OnuronOS v1.1".to_string(),
            "--version".to_string(),
            "1.1.0".to_string(),
            "--target-slot".to_string(),
            "B".to_string(),
        ])
        .unwrap();
        assert!(update_dir.join("update.json").is_file());
        assert!(update_dir.join("image").is_file());

        // 3. Info
        cmd_info(&[update_dir.to_string_lossy().to_string()]).unwrap();

        // 4. Verify
        cmd_verify(&[
            update_dir.to_string_lossy().to_string(),
            "--key-dir".to_string(),
            key_dir.to_string_lossy().to_string(),
        ])
        .unwrap();

        // 5. Tampered image fails verify
        fs::write(update_dir.join("image"), b"TAMPERED_PAYLOAD").unwrap();
        let res = cmd_verify(&[
            update_dir.to_string_lossy().to_string(),
            "--key-dir".to_string(),
            key_dir.to_string_lossy().to_string(),
        ]);
        assert!(res.is_err(), "Verification must fail on tampered image payload");

        let _ = fs::remove_dir_all(temp);
    }
}
