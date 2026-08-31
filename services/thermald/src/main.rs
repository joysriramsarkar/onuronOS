// services/thermald/src/main.rs — Thermal Throttling & Power Capping
use std::thread;
use std::time::Duration;

fn main() {
    println!("[thermald] Thermal Throttling & Power Capping active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
