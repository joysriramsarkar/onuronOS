// services/nilupd/src/main.rs — Onuron OS A/B System Update daemon/CLI.
// Thin CLI over the `nilupd` library crate.

use std::path::Path;
use std::process::ExitCode;

use nilupd::{
    apply_update, get_install_root, recover_pending, rollback, status, verify_image,
};
use nilpkg::get_key_dir;

fn print_help() {
    println!("=========================================================");
    println!("     Onuron OS A/B System Update (nilupd)                ");
    println!("=========================================================");
    println!("Usage:");
    println!("  nilupd verify <update-dir>   — Verify image digest, Ed25519 signature and publisher trust");
    println!("  nilupd apply <update-dir>    — Verify and atomically apply to the inactive slot");
    println!("  nilupd rollback              — Flip back to the previously active slot");
    println!("  nilupd status                — Show the active slot and running image hash");
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
        "rollback" => match rollback(&get_install_root()) {
            Ok(()) => ExitCode::SUCCESS,
            Err(e) => failure("Rollback", e),
        },
        "status" => match status(&get_install_root()) {
            Ok(s) => {
                println!("[nilupd] Active slot: {}", s.active_slot);
                println!("[nilupd] Running image SHA-256: {}", s.running_image_sha256);
                println!("[nilupd] Pending update journal: {}", if s.pending { "yes" } else { "no" });
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
                ExitCode::SUCCESS
            }
            Err(e) => failure("Status", e),
        },
        "daemon" | "run" => {
            let root = get_install_root();
            if let Err(e) = recover_pending(&root) {
                eprintln!("[nilupd] Recovery failed: {e}");
                return ExitCode::FAILURE;
            }
            println!("[nilupd] A/B System Image Chunk-Delta Updater active.");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(60));
            }
        }
        cmd => {
            eprintln!("Unknown command '{cmd}'. Type 'nilupd help' for usage.");
            ExitCode::FAILURE
        }
    }
}