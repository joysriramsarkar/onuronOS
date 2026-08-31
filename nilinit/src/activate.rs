// nilinit/src/activate.rs — On-demand socket activation listener
use std::os::unix::net::UnixListener;
use std::os::unix::io::AsRawFd;
use std::collections::HashMap;

pub struct SocketActivationManager {
    listeners: HashMap<String, UnixListener>,
}

impl SocketActivationManager {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn register(&mut self, service_name: &str, socket_path: &str) -> std::io::Result<()> {
        let _ = std::fs::remove_file(socket_path);
        let listener = UnixListener::bind(socket_path)?;
        listener.set_nonblocking(true)?;
        self.listeners.insert(service_name.to_string(), listener);
        println!("[nilinit:activate] Registered socket activation for {} at {}", service_name, socket_path);
        Ok(())
    }

    pub fn check_pending(&self) -> Vec<String> {
        let mut pending = Vec::new();
        for (name, listener) in &self.listeners {
            match listener.accept() {
                Ok(_) => {
                    pending.push(name.clone());
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No pending connection
                }
                Err(e) => {
                    eprintln!("[nilinit:activate] Error checking socket for {}: {}", name, e);
                }
            }
        }
        pending
    }

    pub fn get_raw_fd(&self, service_name: &str) -> Option<i32> {
        self.listeners.get(service_name).map(|l| l.as_raw_fd())
    }
}
