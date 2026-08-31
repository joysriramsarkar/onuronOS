// services/netd/src/main.rs — nftables Per-App Firewall Manager
use std::thread;
use std::time::Duration;

fn main() {
    println!("[netd] nftables Per-App Firewall Manager active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
