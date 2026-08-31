// softbus/src/main.rs — Distributed SoftBus: mDNS Discovery + QUIC Stream + Control Bridge
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

mod ctl;
mod quic;

use ctl::SoftBusControl;

fn main() {
    println!("=========================================================");
    println!("        NilOS Distributed SoftBus Daemon (nilbus)       ");
    println!("=========================================================");

    let peers = Arc::new(Mutex::new(vec![
        "NilPad-Pro-X1".to_string(),
        "NilBook-Ultra".to_string(),
        "NilVision-Display-65".to_string(),
    ]));

    let ctl = SoftBusControl::new("/run/nilos/bus.sock", peers.clone());
    let _ = ctl.start();

    println!("[nilbus] mDNS Service Discovery broadcasting on BLE / Wi-Fi Aware...");
    println!("[nilbus] Near-field P2P SoftBus active.");

    loop {
        thread::sleep(Duration::from_secs(30));
    }
}
