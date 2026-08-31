// android/agent/src/main.rs — Static Agent inside Android container
use std::os::unix::net::UnixListener;
use std::io::{Read, Write};

fn main() {
    println!("[nilagent] Initialized inside Android container. Listening for host IPC...");
    let _ = std::fs::remove_file("/dev/socket/nilagent");
    if let Ok(listener) = UnixListener::bind("/dev/socket/nilagent") {
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                let mut buf = [0u8; 256];
                if let Ok(n) = s.read(&mut buf) {
                    let cmd = String::from_utf8_lossy(&buf[..n]);
                    println!("[nilagent] Received Intent command: {}", cmd);
                    let _ = s.write_all(b"INTENT_SENT_OK\n");
                }
            }
        }
    }
}
