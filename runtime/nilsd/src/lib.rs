// runtime/nilsd/src/lib.rs — Systemd/NilInit compatible socket activation helper
use std::env;
use std::os::unix::io::FromRawFd;
use std::os::unix::net::UnixListener;

pub const SD_LISTEN_FDS_START: i32 = 3;

pub fn listen_fds() -> Vec<UnixListener> {
    let mut listeners = Vec::new();
    if let Ok(fds_str) = env::var("LISTEN_FDS") {
        if let Ok(num_fds) = fds_str.parse::<i32>() {
            for i in 0..num_fds {
                let fd = SD_LISTEN_FDS_START + i;
                unsafe {
                    listeners.push(UnixListener::from_raw_fd(fd));
                }
            }
        }
    }
    listeners
}

pub fn first_listener_or_bind(fallback_path: &str) -> std::io::Result<UnixListener> {
    let mut fds = listen_fds();
    if !fds.is_empty() {
        println!("[nilsd] Using socket-activated file descriptor (FD {})", SD_LISTEN_FDS_START);
        Ok(fds.remove(0))
    } else {
        let _ = std::fs::remove_file(fallback_path);
        if let Some(parent) = std::path::Path::new(fallback_path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        println!("[nilsd] Binding fallback unix socket: {}", fallback_path);
        UnixListener::bind(fallback_path)
    }
}
