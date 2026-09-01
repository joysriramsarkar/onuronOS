// softbus/src/ctl.rs — Shell <-> SoftBus Local Control Bridge (/run/nilos/bus.sock)
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
#[cfg(unix)]
use std::thread;
use std::sync::{Arc, Mutex};

pub struct SoftBusControl {
    socket_path: String,
    peers: Arc<Mutex<Vec<String>>>,
}

impl SoftBusControl {
    pub fn new(socket_path: &str, peers: Arc<Mutex<Vec<String>>>) -> Self {
        #[cfg(unix)]
        let _ = std::fs::remove_file(socket_path);
        Self {
            socket_path: socket_path.to_string(),
            peers,
        }
    }

    pub fn start(&self) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            let listener = UnixListener::bind(&self.socket_path)?;
            println!("[nilbus:ctl] Control socket listening on {}", self.socket_path);
            let peers_clone = self.peers.clone();

            thread::spawn(move || {
                for stream in listener.incoming() {
                    if let Ok(mut s) = stream {
                        let mut buf = [0u8; 512];
                        if let Ok(n) = s.read(&mut buf) {
                            let cmd = String::from_utf8_lossy(&buf[..n]);
                            if cmd.starts_with("LIST_PEERS") {
                                let p = peers_clone.lock().unwrap();
                                let resp = p.join(",") + "\n";
                                let _ = s.write_all(resp.as_bytes());
                            } else if cmd.starts_with("HANDOFF") {
                                println!("[nilbus:ctl] Initiating softbus handoff: {}", cmd.trim());
                                let _ = s.write_all(b"OK\n");
                            }
                        }
                    }
                }
            });
        }
        #[cfg(not(unix))]
        {
            println!("[nilbus:ctl] Simulated SoftBus control bridge active on {}", self.socket_path);
        }
        Ok(())
    }
}

