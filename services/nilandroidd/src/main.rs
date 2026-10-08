// services/nilandroidd/src/main.rs — Android Container LifeCycle & Binder-Shim Bridge
// Governed by OnuronOS Architecture: Guest container must be isolated via namespaces and cgroups.

pub mod isolation;

use isolation::ContainerProfile;
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use nilprotocol::{Frame, MessageType};

fn main() {
    println!("\x1b[1;36m[nilandroidd]\x1b[0m Android Container Bridge & LifeCycle Manager Initializing...");

    let profile = ContainerProfile::default();
    match profile.validate_isolation() {
        Ok(()) => println!("\x1b[1;32m[nilandroidd] [  OK  ]\x1b[0m Isolation boundaries verified (SELinux: {}, namespaces: enabled)", profile.selinux_context),
        Err(e) => {
            eprintln!("\x1b[1;31m[nilandroidd] [CRITICAL]\x1b[0m Container isolation violation: {}", e);
            std::process::exit(1);
        }
    }

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file("/run/nilos/android.sock");
        if let Ok(listener) = UnixListener::bind("/run/nilos/android.sock") {
            println!("\x1b[1;32m[nilandroidd] [  OK  ]\x1b[0m Listening on /run/nilos/android.sock");
            let policy = nilsd::auth::load_default_policy();
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    // C1: the Android bridge is root-only by policy.
                    if !nilsd::auth::authorize_stream(&policy, "nilandroidd", &s) {
                        continue;
                    }
                    let mut buf = [0u8; 512];
                    if let Ok(n) = s.read(&mut buf) {
                        if n >= 4 && &buf[..4] == &nilprotocol::PROTOCOL_MAGIC {
                            let mut cursor = std::io::Cursor::new(&buf[..n]);
                            if let Ok(frame) = Frame::read_from(&mut cursor) {
                                let resp = handle_ipc_request(&frame);
                                let _ = resp.write_to(&mut s);
                                continue;
                            }
                        }
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

pub fn handle_ipc_request(frame: &Frame) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),
        _ => Frame::new(MessageType::Pong, frame.request_id, b"OK".to_vec()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ipc_ping() {
        let ping = Frame::new(MessageType::Ping, 42, vec![]);
        let pong = handle_ipc_request(&ping);
        assert_eq!(pong.message_type, u16::from(MessageType::Pong));
        assert_eq!(pong.payload, b"pong");
    }
}
