// services/btd/src/main.rs — Bluetooth OBEX File Share Daemon
use std::thread;
use std::time::Duration;

fn main() {
    println!("[btd] Bluetooth OBEX File Share Daemon active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
