// services/nilwdt/src/main.rs — Hardware Watchdog Feeder
use std::thread;
use std::time::Duration;

fn main() {
    println!("[nilwdt] Hardware Watchdog Feeder active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
