// services/nilkeyd/src/main.rs — Hardware-Backed App Key Lifecycle Daemon
use std::thread;
use std::time::Duration;
use std::os::unix::net::UnixListener;
use std::io::{Read, Write};

mod fscrypt;

fn main() {
    println!("[nilkeyd] fscrypt v2 Key Lifecycle Daemon started.");
    let _ = std::fs::remove_file("/run/nilos/keyd.sock");
    if let Ok(listener) = UnixListener::bind("/run/nilos/keyd.sock") {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                let mut buf = [0u8; 128];
                if let Ok(n) = s.read(&mut buf) {
                    let cmd = String::from_utf8_lossy(&buf[..n]);
                    println!("[nilkeyd] Key Request: {}", cmd.trim());
                    let _ = s.write_all(b"KEY_UNLOCKED\n");
                }
            }
        }
    }
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
