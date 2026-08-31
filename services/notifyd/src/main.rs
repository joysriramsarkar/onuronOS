// services/notifyd/src/main.rs — Notification Broker
use std::os::unix::net::UnixListener;
use std::io::{Read, Write};

fn main() {
    println!("[notifyd] Notification Broker active on /run/nilos/notify.sock");
    let listener = nilsd::first_listener_or_bind("/run/nilos/notify.sock").unwrap();
    for stream in listener.incoming() {
        if let Ok(mut s) = stream {
            let mut buf = [0u8; 512];
            if let Ok(n) = s.read(&mut buf) {
                let msg = String::from_utf8_lossy(&buf[..n]);
                println!("[NOTIFY BANNER] {}", msg.trim());
                let _ = s.write_all(b"OK\n");
            }
        }
    }
}
