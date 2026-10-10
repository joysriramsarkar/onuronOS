// runtime/nilsd/src/lib.rs — Systemd/NilInit compatible socket activation helper
pub mod auth;
pub use auth::{IpcPolicy, PeerCred};

#[cfg(unix)]
pub use auth::{authorize_fd, authorize_stream, peer_cred, peer_cred_fd};

#[cfg(unix)]
use std::env;
#[cfg(unix)]
use std::os::unix::io::FromRawFd;
#[cfg(unix)]
use std::os::unix::net::UnixListener;

pub const SD_LISTEN_FDS_START: i32 = 3;

#[cfg(unix)]
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

#[cfg(unix)]
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

#[cfg(not(unix))]
pub fn listen_fds() -> Vec<()> {
    Vec::new()
}

#[cfg(not(unix))]
pub fn first_listener_or_bind(_fallback_path: &str) -> std::io::Result<()> {
    Ok(())
}

/// Notify supervisor and runtime that this service has initialized and is ready.
///
/// On Unix, if NOTIFY_SOCKET or ONURON_NOTIFY_SOCKET is present, a datagram notification
/// is dispatched. Additionally, the readiness marker `/run/onuron/ready/<service_name>` is written.
#[cfg(unix)]
pub fn notify_ready(service_name: &str, sock_path: Option<&str>) -> std::io::Result<()> {
    if let Ok(notify_socket) = env::var("NOTIFY_SOCKET").or_else(|_| env::var("ONURON_NOTIFY_SOCKET")) {
        use std::os::unix::net::UnixDatagram;
        if let Ok(socket) = UnixDatagram::unbound() {
            let msg = format!("READY=1\nMAINPID={}\nSERVICE={}\n", std::process::id(), service_name);
            let _ = socket.send_to(msg.as_bytes(), &notify_socket);
        }
    }

    let ready_dir = if let Ok(dir) = env::var("ONURON_READY_DIR") {
        std::path::PathBuf::from(dir)
    } else {
        let default_run = std::path::Path::new("/run/onuron/ready");
        if std::fs::create_dir_all(default_run).is_ok() {
            default_run.to_path_buf()
        } else if let Ok(xdg) = env::var("XDG_RUNTIME_DIR") {
            let xdg_path = std::path::PathBuf::from(xdg).join("onuron/ready");
            let _ = std::fs::create_dir_all(&xdg_path);
            xdg_path
        } else {
            let tmp_path = std::path::PathBuf::from("/tmp/onuron/ready");
            let _ = std::fs::create_dir_all(&tmp_path);
            tmp_path
        }
    };

    let _ = std::fs::create_dir_all(&ready_dir);
    let ready_file = ready_dir.join(service_name);
    let pid = std::process::id();
    let sock = sock_path.unwrap_or("");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let content = format!("pid={}\nsocket={}\ntimestamp_secs={}\n", pid, sock, timestamp);
    let _ = std::fs::write(ready_file, content);
    Ok(())
}

#[cfg(not(unix))]
pub fn notify_ready(_service_name: &str, _sock_path: Option<&str>) -> std::io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_ready_does_not_panic() {
        assert!(notify_ready("test_daemon", Some("/run/onuron/test.sock")).is_ok());
    }
}

