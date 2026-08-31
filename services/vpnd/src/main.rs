// services/vpnd/src/main.rs — WireGuard VPN Network Daemon
use std::thread;
use std::time::Duration;

fn main() {
    println!("[vpnd] WireGuard VPN Network Daemon active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
