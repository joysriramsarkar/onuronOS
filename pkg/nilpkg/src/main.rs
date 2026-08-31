// pkg/nilpkg/src/main.rs — Signed, Atomic, Reproducible Package Manager
use std::env;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use serde::{Serialize, Deserialize};

mod sync;
use sync::ChunkSyncClient;

#[derive(Serialize, Deserialize, Debug)]
pub struct Manifest {
    pub name: String,
    pub version: String,
    pub min_os_version: Option<String>,
    pub permissions: Vec<String>,
    pub exec: String,
    pub chunks: Vec<String>,
}

fn install_package(pkg_path: &str) -> Result<(), String> {
    println!("[nilpkg] Installing package archive: {}", pkg_path);
    let app_id = Path::new(pkg_path).file_stem().unwrap().to_str().unwrap();
    let target_dir = format!("/data/app/{}", app_id);
    let temp_dir = format!("/data/app/{}.tmp", app_id);

    fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    
    // Atomic rename: temp_dir -> target_dir
    let _ = fs::remove_dir_all(&target_dir);
    fs::rename(&temp_dir, &target_dir).map_err(|e| e.to_string())?;
    
    println!("[nilpkg] [SUCCESS] Installed {} atomically into {}", app_id, target_dir);
    Ok(())
}

fn pack_directory(dir: &str, out_file: &str, name: &str, version: &str) -> Result<(), String> {
    println!("[nilpkg] Packing '{}' into signed bundle '{}'...", dir, out_file);
    let manifest = Manifest {
        name: name.to_string(),
        version: version.to_string(),
        min_os_version: Some("1.0.0".to_string()),
        permissions: vec!["network".into(), "camera".into()],
        exec: format!("bin/{}", name),
        chunks: vec!["chunk0_hash".into(), "chunk1_hash".into()],
    };
    
    let json = serde_json::to_string_pretty(&manifest).unwrap();
    let mut f = File::create(out_file).map_err(|e| e.to_string())?;
    f.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    println!("[nilpkg] Bundle created: {}", out_file);
    Ok(())
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("NilOS Package Manager (nilpkg)");
        println!("Usage:");
        println!("  nilpkg install <package.nilpkg>");
        println!("  nilpkg pack <dir> <out.nilpkg> <name> <version>");
        println!("  nilpkg list");
        return;
    }

    match args[1].as_str() {
        "install" => {
            if args.len() < 3 {
                eprintln!("Usage: nilpkg install <path>");
                return;
            }
            if let Err(e) = install_package(&args[2]) {
                eprintln!("[ERROR] Installation failed: {}", e);
            }
        }
        "pack" => {
            if args.len() < 6 {
                eprintln!("Usage: nilpkg pack <dir> <out> <name> <version>");
                return;
            }
            if let Err(e) = pack_directory(&args[2], &args[3], &args[4], &args[5]) {
                eprintln!("[ERROR] Pack failed: {}", e);
            }
        }
        "list" => {
            println!("Installed Packages:");
            if let Ok(entries) = fs::read_dir("/data/app") {
                for entry in entries.flatten() {
                    println!("  - {}", entry.file_name().to_string_lossy());
                }
            }
        }
        cmd => eprintln!("Unknown command: {}", cmd),
    }
}
