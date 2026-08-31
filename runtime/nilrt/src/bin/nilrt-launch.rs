// runtime/nilrt/src/bin/nilrt-launch.rs — App Launcher with Snapshot Restore
use std::env;
use std::process::Command;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: nilrt-launch <app_id> [snapshot_payload_path]");
        return;
    }

    let app_id = &args[1];
    let snapshot_path = args.get(2);

    println!("[nilrt-launch] Launching application: {}", app_id);
    if let Some(snap) = snapshot_path {
        println!("[nilrt-launch] Restoring from Handoff Snapshot: {}", snap);
    }

    let bin_path = format!("/data/app/{}/bin/{}", app_id, app_id);
    let mut cmd = if std::path::Path::new(&bin_path).exists() {
        Command::new(&bin_path)
    } else {
        Command::new(format!("/usr/bin/{}", app_id))
    };

    if let Some(snap) = snapshot_path {
        cmd.arg("--restore").arg(snap);
    }

    match cmd.spawn() {
        Ok(mut child) => {
            let _ = child.wait();
        }
        Err(e) => eprintln!("[nilrt-launch] Failed to start {}: {}", app_id, e),
    }
}
