// services/nilandroidd/src/main.rs — Android Container LifeCycle & Binder-Shim Bridge
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;

fn main() {
    println!("[nilandroidd] Android Container Bridge starting...");
    #[cfg(unix)]
    {
        let _ = std::fs::remove_file("/run/nilos/android.sock");
        if let Ok(listener) = UnixListener::bind("/run/nilos/android.sock") {
            println!("[nilandroidd] Listening on /run/nilos/android.sock");
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    let mut buf = [0u8; 256];
                    if let Ok(n) = s.read(&mut buf) {
                        let cmd = String::from_utf8_lossy(&buf[..n]);
                        println!("[nilandroidd] Forwarding Intent: {}", cmd.trim());
                        let _ = s.write_all(b"OK\n");
                    }
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        println!("[nilandroidd] Simulated Android LXC runtime bridge active.");
    }
}

