// services/crashd/src/main.rs — Crash Tombstone & Kernel Panic Collector
use std::thread;
use std::time::Duration;

fn main() {
    println!("[crashd] Crash Tombstone & Kernel Panic Collector active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
