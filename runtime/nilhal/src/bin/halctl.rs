// runtime/nilhal/src/bin/halctl.rs — HAL Inspection & Diagnostic CLI
use std::env;
use nilhal::HalDevice;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        println!("Usage: halctl <light|camera|fingerprint|audio|sensors>");
        return;
    }

    let hal_name = &args[1];
    println!("Probing HAL module: {}...", hal_name);
    match HalDevice::load(hal_name) {
        Ok(dev) => println!("[SUCCESS] Loaded HAL: {}", dev.get_name()),
        Err(e) => eprintln!("[ERROR] Failed to load HAL '{}': {}", hal_name, e),
    }
}
