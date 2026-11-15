// services/nilupd/src/main.rs — Onuron OS A/B System Update daemon/CLI.
// Thin CLI over the `nilupd` library crate.

use std::path::Path;
use std::process::ExitCode;

use nilpkg::get_key_dir;
use nilupd::{
    apply_update, boot_status, fetch_update, get_install_root, mark_boot_successful,
    record_boot_attempt, rollback, status, verify_image, FetchUpdateOptions,
};

fn print_help() {
    println!("=========================================================");
    println!("     Onuron OS A/B System Update (nilupd)                ");
    println!("=========================================================");
    println!("Usage:");
    println!("  nilupd verify <update-dir>   — Verify image digest, Ed25519 signature and publisher trust");
    println!("  nilupd apply <update-dir>    — Verify and atomically apply to the inactive slot");
    println!("  nilupd fetch <manifest-url> [--image <url>] [--apply] [--allow-insecure]");
    println!("                               — Fetch signed OTA update over HTTPS, verify, and optionally apply");
    println!("  nilupd rollback              — Flip back to the previously active slot");
    println!("  nilupd status                — Show the active slot and running image hash");
    println!("  nilupd boot-status           — Show bootloader/boot-control slot state & tries remaining");
    println!("  nilupd boot-mark-success     — Confirm current boot candidate successful (clears rollback)");
    println!("  nilupd boot-attempt          — Record candidate boot attempt (tests auto-rollback limit)");
    println!("  nilupd daemon                — Recover any interrupted update, then run");
    println!("  nilupd help                  — Show this message");
    println!("=========================================================");
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args[1] == "help" || args[1] == "--help" {
        print_help();
        return ExitCode::SUCCESS;
    }

    let failure = |label: &str, e: String| -> ExitCode {
        eprintln!("[ERROR] {label} failed: {e}");
        ExitCode::FAILURE
    };

    match args[1].as_str() {
        "verify" => {
            let Some(dir) = args.get(2) else {
                eprintln!("Usage: nilupd verify <update-dir>");
                return ExitCode::FAILURE;
            };
            match verify_image(Path::new(dir), &get_key_dir()) {
                Ok(manifest) => {
                    println!(
                        "[nilupd] Verified '{}' v{} targeting slot {} (image {}…)",
                        manifest.name,
                        manifest.version,
                        manifest.target_slot,
                        &manifest.image_sha256[..16.min(manifest.image_sha256.len())]
                    );
                    println!("[nilupd] Note: update-image trust only; measured boot / TPM is not implemented.");
                    ExitCode::SUCCESS
                }
                Err(e) => failure("Verification", e),
            }
        }
        "apply" => {
            let Some(dir) = args.get(2) else {
                eprintln!("Usage: nilupd apply <update-dir>");
                return ExitCode::FAILURE;
            };
            match apply_update(Path::new(dir), &get_install_root(), &get_key_dir()) {
                Ok(()) => ExitCode::SUCCESS,
                Err(e) => failure("Update", e),
            }
        }
        "fetch" => {
            let Some(manifest_url) = args.get(2) else {
                eprintln!("Usage: nilupd fetch <manifest-url> [--image <url>] [--apply] [--allow-insecure]");
                return ExitCode::FAILURE;
            };
            let mut image_url: Option<String> = None;
            let mut do_apply = false;
            let mut allow_insecure = false;
            let mut i = 3;
            while i < args.len() {
                match args[i].as_str() {
                    "--image" => {
                        let Some(v) = args.get(i + 1) else {
                            eprintln!("--image requires a URL");
                            return ExitCode::FAILURE;
                        };
                        image_url = Some(v.clone());
                        i += 2;
                    }
                    "--apply" => {
                        do_apply = true;
                        i += 1;
                    }
                    "--allow-insecure" => {
                        allow_insecure = true;
                        i += 1;
                    }
                    other => {
                        eprintln!("Unknown fetch option '{other}'");
                        return ExitCode::FAILURE;
                    }
                }
            }
            let staging_root = get_install_root().join("staging");
            let opts = FetchUpdateOptions { allow_insecure };
            match fetch_update(
                manifest_url,
                image_url.as_deref(),
                &staging_root,
                &get_key_dir(),
                &opts,
            ) {
                Ok(fetched) => {
                    println!(
                        "[nilupd] Fetched and verified '{}' v{} -> {}",
                        fetched.manifest.name,
                        fetched.manifest.version,
                        fetched.staging_dir.display()
                    );
                    if do_apply {
                        let res = apply_update(
                            &fetched.staging_dir,
                            &get_install_root(),
                            &get_key_dir(),
                        );
                        let _ = std::fs::remove_dir_all(&fetched.staging_dir);
                        match res {
                            Ok(()) => ExitCode::SUCCESS,
                            Err(e) => failure("Apply fetched update", e),
                        }
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => failure("Fetch", e),
            }
        }
        "rollback" => match rollback(&get_install_root()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => failure("Rollback", e),
        },
        "status" => match status(&get_install_root()) {
            Ok(s) => {
                println!("[nilupd] Active slot: {}", s.active_slot);
                println!("[nilupd] Running image SHA-256: {}", s.running_image_sha256);
                println!(
                    "[nilupd] Pending update journal: {}",
                    if s.pending { "yes" } else { "no" }
                );
                for slot in &s.slots {
                    println!(
                        "  slot {}: {}{}",
                        slot.slot,
                        if slot.active { "active" } else { "inactive" },
                        match (&slot.version, &slot.image_sha256) {
                            (Some(v), Some(h)) => format!("  v{v}  {}", &h[..16.min(h.len())]),
                            _ => String::new(),
                        }
                    );
                }
                if let Some(bc) = &s.boot_control {
                    println!(
                        "[nilupd] Boot Control Backend: {} (simulated: {})",
                        bc.backend_name, bc.is_simulated
                    );
                    println!(
                        "  current boot slot: {}, fallback: {}",
                        bc.current_slot, bc.fallback_slot
                    );
                    for bs in &bc.slots {
                        println!(
                            "  slot {}: successful={}, tries_remaining={}, bootable={}",
                            bs.slot, bs.is_successful, bs.tries_remaining, bs.is_bootable
                        );
                    }
                }
                ExitCode::SUCCESS
            }
            Err(e) => failure("Status", e),
        },
        "boot-status" => match boot_status(&get_install_root()) {
            Ok(bc) => {
                println!("[nilupd] Boot Control Backend: {}", bc.backend_name);
                println!(
                    "  current slot: {}, fallback slot: {}",
                    bc.current_slot, bc.fallback_slot
                );
                for bs in &bc.slots {
                    println!(
                        "  slot {}: active={}, successful={}, tries_remaining={}, bootable={}",
                        bs.slot, bs.is_active, bs.is_successful, bs.tries_remaining, bs.is_bootable
                    );
                }
                ExitCode::SUCCESS
            }
            Err(e) => failure("Boot Status", e),
        },
        "boot-mark-success" => match mark_boot_successful(&get_install_root()) {
            Ok(()) => {
                println!("[nilupd] Boot marked successful. Rollback timer cleared.");
                ExitCode::SUCCESS
            }
            Err(e) => failure("Mark Boot Successful", e),
        },
        "boot-attempt" => match record_boot_attempt(&get_install_root()) {
            Ok(slot) => {
                println!("[nilupd] Recorded boot attempt for slot {slot}. Boot succeeded.");
                ExitCode::SUCCESS
            }
            Err(e) => failure("Boot Attempt", e),
        },
        "daemon" | "run" => {
            nilupd::run_daemon();
        }
        cmd => {
            eprintln!("Unknown command '{cmd}'. Type 'nilupd help' for usage.");
            ExitCode::FAILURE
        }
    }
}
