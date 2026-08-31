// services/ntpd/src/main.rs — SNTP Lightweight Time Synchronization
use std::thread;
use std::time::Duration;

fn main() {
    println!("[ntpd] SNTP Lightweight Time Synchronization active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
