// services/nilsr/src/main.rs — Accessibility Screen Reader
use std::thread;
use std::time::Duration;

fn main() {
    println!("[nilsr] Accessibility Screen Reader active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
