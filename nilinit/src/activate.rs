// nilinit/src/activate.rs — On-demand socket activation listener
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::collections::HashMap;

pub struct SocketActivationManager {
    #[cfg(unix)]
    listeners: HashMap<String, UnixListener>,
    #[cfg(not(unix))]
    listeners: HashMap<String, ()>,
}

impl SocketActivationManager {
    pub fn new() -> Self {
        Self {
            listeners: HashMap::new(),
        }
    }

    pub fn register(&mut self, service_name: &str, socket_path: &str) -> std::io::Result<()> {
        #[cfg(unix)]
        {
            let _ = std::fs::remove_file(socket_path);
            let listener = UnixListener::bind(socket_path)?;
            listener.set_nonblocking(true)?;
            self.listeners.insert(service_name.to_string(), listener);
        }
        #[cfg(not(unix))]
        {
            self.listeners.insert(service_name.to_string(), ());
        }
        println!("[nilinit:activate] Registered socket activation for {} at {}", service_name, socket_path);
        Ok(())
    }

    pub fn check_pending(&self) -> Vec<String> {
        #[allow(unused_mut)]
        let mut pending = Vec::new();
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            for (name, listener) in &self.listeners {
                let mut pfd = libc::pollfd {
                    fd: listener.as_raw_fd(),
                    events: libc::POLLIN,
                    revents: 0,
                };
                let ret = unsafe { libc::poll(&mut pfd, 1, 0) };
                if ret > 0 && (pfd.revents & libc::POLLIN) != 0 {
                    pending.push(name.clone());
                }
            }
        }
        #[cfg(not(unix))]
        {
            let _ = &pending;
        }
        pending
    }

    #[allow(dead_code)]
    pub fn is_registered(&self, service_name: &str) -> bool {
        self.listeners.contains_key(service_name)
    }

    pub fn get_raw_fd(&self, service_name: &str) -> Option<i32> {
        #[cfg(unix)]
        {
            use std::os::unix::io::AsRawFd;
            self.listeners.get(service_name).map(|l| l.as_raw_fd())
        }
        #[cfg(not(unix))]
        {
            let _ = service_name;
            None
        }
    }
}

