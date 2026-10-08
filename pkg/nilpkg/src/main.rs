// pkg/nilpkg/src/main.rs — Onuron OS Official Package Manager (nilpkg)
// Thin CLI over the `nilpkg` library crate.

use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

use nilpkg::{
    fetch::{fetch_package, FetchOptions},
    generate_keypair, get_app_dir, get_cache_dir, get_key_dir, install_demo, install_local,
    keygen_command, list_packages, pack_package, remove_package, revoke_command, rollback_package,
    trust_command, upgrade_local, validate_app_id, verify_installed,
};

/// `nilpkg pack <binary> <output.nilax> --app-id <id> ...`
fn cmd_pack(args: &[String]) -> Result<(), String> {
    if args.len() < 2 {
        return Err("Usage: nilpkg pack <binary> <output.nilax> --app-id <id> [--name <name>] [--version <ver>] [--desc <desc>] [--perm <p1,p2>] [--key <keyfile>]".into());
    }
    let binary_path = PathBuf::from(&args[0]);
    let output_path = PathBuf::from(&args[1]);

    let mut app_id = None;
    let mut name = None;
    let mut version = "1.0.0".to_string();
    let mut description = "OnuronOS Native Application".to_string();
    let mut permissions = vec!["display".to_string(), "input".to_string()];
    let mut key_path: Option<PathBuf> = None;

    let mut i = 2;
    while i < args.len() {
        match args[i].as_str() {
            "--app-id" => {
                i += 1;
                app_id = args.get(i).cloned();
            }
            "--name" => {
                i += 1;
                name = args.get(i).cloned();
            }
            "--version" => {
                i += 1;
                if let Some(v) = args.get(i) {
                    version = v.clone();
                }
            }
            "--desc" => {
                i += 1;
                if let Some(d) = args.get(i) {
                    description = d.clone();
                }
            }
            "--perm" => {
                i += 1;
                if let Some(p) = args.get(i) {
                    permissions = p.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
            }
            "--key" => {
                i += 1;
                key_path = args.get(i).map(PathBuf::from);
            }
            other => return Err(format!("Unknown option '{other}'")),
        }
        i += 1;
    }

    let app_id = app_id.ok_or("Missing required --app-id argument")?;
    let name = name.unwrap_or_else(|| app_id.clone());

    let signing_key = if let Some(kp) = key_path {
        let text = std::fs::read_to_string(&kp).map_err(|e| format!("Could not read key {}: {e}", kp.display()))?;
        if text.starts_with("enc1$") {
            let pass = nilpkg::keymgmt::read_passphrase("Passphrase: ")?;
            let raw = nilpkg::keymgmt::decrypt_private_key(&text, &pass)
                .ok_or("Failed to decrypt private key (wrong passphrase or tampered file)")?;
            let raw_arr: [u8; 32] = raw.try_into().map_err(|_| "Invalid key size")?;
            ed25519_dalek::SigningKey::from_bytes(&raw_arr)
        } else {
            let hex_clean = text.trim();
            let raw = nilpkg::hex::decode(hex_clean).map_err(|e| format!("Invalid hex key: {e}"))?;
            let raw_arr: [u8; 32] = raw.try_into().map_err(|_| "Invalid key size")?;
            ed25519_dalek::SigningKey::from_bytes(&raw_arr)
        }
    } else {
        println!("[nilpkg] No --key provided; generating an ephemeral developer keypair...");
        let (k, _) = generate_keypair();
        k
    };

    let manifest = pack_package(
        &app_id,
        &name,
        &version,
        &description,
        permissions,
        &binary_path,
        &output_path,
        &signing_key,
    )?;

    println!(
        "[nilpkg] Successfully packed {} v{} -> {}",
        manifest.app_id,
        manifest.version,
        output_path.display()
    );
    if let Some(pubhex) = &manifest.public_key_hex {
        println!("[nilpkg] Signed with Publisher Public Key: {}", pubhex);
    }
    Ok(())
}

fn print_help() {
    println!("=========================================================");
    println!("     Onuron OS Official Package Manager (nilpkg v2.0)    ");
    println!("=========================================================");
    println!("Usage:");
    println!("  nilpkg install <package.nilax or dir> — Install signed package (trusted publisher required)");
    println!("  nilpkg pack <binary> <out.nilax>     — Pack and sign a native app into .nilax package");
    println!("                                         --app-id <id> [--name <n>] [--version <v>] [--key <k>]");
    println!("  nilpkg fetch <url>                   — Download over HTTPS, verify signature and digest");
    println!("                                         [--install] [--sha256 <hex>] [--allow-insecure] [--cache <dir>]");
    println!("  nilpkg upgrade <directory>           — Upgrade an installed package with rollback safety");
    println!("  nilpkg rollback <pkg_id>             — Restore the version replaced by the last upgrade");
    println!("  nilpkg demo <pkg_id>                 — Create a sample launcher, NOT a real app");
    println!("  nilpkg remove <pkg_id>               — Remove installed package");
    println!("  nilpkg list                          — List installed manifests (not reverified)");
    println!("  nilpkg verify <pkg_id>               — Recheck installed files, signature and publisher trust");
    println!("  nilpkg keygen                        — Generate developer Ed25519 signing keypair");
    println!("  nilpkg trust <keyfile>               — Trust a publisher public key");
    println!("  nilpkg revoke <keyfile>              — Revoke a trusted publisher key");
    println!("  nilpkg info <pkg_id>                 — View package manifest & Ed25519 signature");
    println!("=========================================================");
}

/// `nilpkg fetch <url>` — download, verify, optionally install.
fn cmd_fetch(args: &[String]) -> Result<(), String> {
    let mut url: Option<&str> = None;
    let mut install = false;
    let mut opts = FetchOptions::new();
    let mut cache_dir: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--allow-insecure" => opts.allow_insecure = true,
            "--install" => install = true,
            "--no-sidecar" => opts.sidecar = false,
            "--sha256" => {
                i += 1;
                let v = args.get(i).ok_or("--sha256 requires a hex digest")?;
                if v.len() != 64 || !v.bytes().all(|b| b.is_ascii_hexdigit()) {
                    return Err("--sha256 must be exactly 64 hex characters".into());
                }
                opts.expected_sha256 = Some(v.as_str());
            }
            "--cache" => {
                i += 1;
                let v = args.get(i).ok_or("--cache requires a directory path")?;
                cache_dir = Some(PathBuf::from(v));
            }
            other if other.starts_with("--") => return Err(format!("Unknown option '{other}'")),
            other => {
                if url.is_some() {
                    return Err("Only one URL may be given".into());
                }
                url = Some(other);
            }
        }
        i += 1;
    }

    let url = url.ok_or(
        "Usage: nilpkg fetch <url> [--install] [--sha256 <hex>] [--allow-insecure] [--cache <dir>]",
    )?;
    let cache = cache_dir.unwrap_or_else(get_cache_dir);

    let fetched = fetch_package(url, &cache, &get_key_dir(), &opts)?;
    println!(
        "[nilpkg] Verified {} v{} — archive at {}",
        fetched.manifest.app_id,
        fetched.manifest.version,
        fetched.path.display()
    );

    let result = if install {
        install_local(&fetched.staged_dir, &get_app_dir(), &get_key_dir())
    } else {
        println!("[nilpkg] Archive cached; pass --install to install it now.");
        Ok(())
    };
    // The staging extraction is never left behind, success or failure.
    fetched.cleanup();
    result
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args[1] == "help" || args[1] == "--help" {
        print_help();
        return ExitCode::SUCCESS;
    }

    let ok = |result: Result<(), String>, label: &str| match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("[ERROR] {label} failed: {e}");
            ExitCode::FAILURE
        }
    };

    match args[1].as_str() {
        "install" | "i" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg install <package.nilax or directory>");
                return ExitCode::FAILURE;
            }
            ok(
                install_local(std::path::Path::new(&args[2]), &get_app_dir(), &get_key_dir()),
                "Installation",
            )
        }
        "pack" | "build" => ok(cmd_pack(&args[2..]), "Pack"),
        "fetch" => ok(cmd_fetch(&args[2..]), "Fetch"),
        "demo" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg demo <package_id>");
                return ExitCode::FAILURE;
            }
            ok(install_demo(&args[2]), "Demo installation")
        }
        "upgrade" => {
            let allow_downgrade = args.iter().any(|a| a == "--allow-downgrade");
            let directory = args.iter().skip(2).find(|a| !a.starts_with("--"));
            let Some(directory) = directory else {
                eprintln!("Usage: nilpkg upgrade <directory> [--allow-downgrade]");
                return ExitCode::FAILURE;
            };
            ok(
                upgrade_local(
                    std::path::Path::new(directory),
                    &get_app_dir(),
                    &get_key_dir(),
                    allow_downgrade,
                ),
                "Upgrade",
            )
        }
        "rollback" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg rollback <package_id>");
                return ExitCode::FAILURE;
            }
            ok(rollback_package(&args[2], &get_app_dir()), "Rollback")
        }
        "trust" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg trust <keyfile>");
                return ExitCode::FAILURE;
            }
            ok(trust_command(&args[2]), "Trust")
        }
        "revoke" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg revoke <keyfile>");
                return ExitCode::FAILURE;
            }
            ok(revoke_command(&args[2]), "Revoke")
        }
        "remove" | "rm" | "uninstall" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg remove <package_id>");
                return ExitCode::FAILURE;
            }
            ok(remove_package(&args[2]), "Removal")
        }
        "list" | "ls" => {
            list_packages();
            ExitCode::SUCCESS
        }
        "keygen" => ok(keygen_command(), "Keygen"),
        "info" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg info <package_id>");
                return ExitCode::FAILURE;
            }
            if let Err(e) = validate_app_id(&args[2]) {
                eprintln!("[ERROR] {e}");
                return ExitCode::FAILURE;
            }
            let manifest_path = get_app_dir().join(&args[2]).join("manifest.json");
            match std::fs::read_to_string(manifest_path) {
                Ok(data) => {
                    println!("{data}");
                    ExitCode::SUCCESS
                }
                Err(_) => {
                    eprintln!("Package '{}' not found or has no manifest.", args[2]);
                    ExitCode::FAILURE
                }
            }
        }
        "verify" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg verify <package_id>");
                return ExitCode::FAILURE;
            }
            match verify_installed(&args[2], &get_app_dir(), &get_key_dir()) {
                Ok(()) => {
                    println!(
                        "[nilpkg] Verified {}: payload, signature and trusted publisher are valid",
                        args[2]
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("[ERROR] Verification failed: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        cmd => {
            eprintln!("Unknown command '{cmd}'. Type 'nilpkg help' for usage.");
            ExitCode::FAILURE
        }
    }
}
