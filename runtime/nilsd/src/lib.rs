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
/// is dispatched. Additionally, the readiness marker `<ready_dir>/<service_name>` is written.
pub fn notify_ready(service_name: &str, sock_path: Option<&str>) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::env;
        if let Ok(notify_socket) = env::var("NOTIFY_SOCKET").or_else(|_| env::var("ONURON_NOTIFY_SOCKET")) {
            use std::os::unix::net::UnixDatagram;
            if let Ok(socket) = UnixDatagram::unbound() {
                let msg = format!("READY=1\nMAINPID={}\nSERVICE={}\n", std::process::id(), service_name);
                let _ = socket.send_to(msg.as_bytes(), &notify_socket);
            }
        }
    }

    let ready_dir = if let Ok(dir) = std::env::var("ONURON_READY_DIR") {
        std::path::PathBuf::from(dir)
    } else {
        #[cfg(unix)]
        {
            let default_run = std::path::Path::new("/run/onuron/ready");
            if std::fs::create_dir_all(default_run).is_ok() {
                default_run.to_path_buf()
            } else if let Ok(xdg) = std::env::var("XDG_RUNTIME_DIR") {
                let xdg_path = std::path::PathBuf::from(xdg).join("onuron/ready");
                let _ = std::fs::create_dir_all(&xdg_path);
                xdg_path
            } else {
                let tmp_path = std::path::PathBuf::from("/tmp/onuron/ready");
                let _ = std::fs::create_dir_all(&tmp_path);
                tmp_path
            }
        }
        #[cfg(not(unix))]
        {
            std::env::temp_dir().join("onuron").join("ready")
        }
    };

    std::fs::create_dir_all(&ready_dir)?;
    let ready_file = ready_dir.join(service_name);
    let pid = std::process::id();
    let sock = sock_path.unwrap_or("");
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let content = format!("pid={}\nsocket={}\ntimestamp_secs={}\n", pid, sock, timestamp);
    std::fs::write(&ready_file, content)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_notify_ready_creates_valid_marker_file() {
        let temp_dir = std::env::temp_dir().join(format!("onuron_test_ready_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::env::set_var("ONURON_READY_DIR", &temp_dir);

        let res = notify_ready("test_daemon", Some("/run/onuron/test.sock"));
        assert!(res.is_ok(), "notify_ready should succeed with valid directory");

        let marker_path = temp_dir.join("test_daemon");
        assert!(marker_path.exists(), "Marker file must actually be created on disk");

        let content = std::fs::read_to_string(&marker_path).expect("Marker file should be readable");
        assert!(content.contains(&format!("pid={}", std::process::id())), "Marker must contain correct PID");
        assert!(content.contains("socket=/run/onuron/test.sock"), "Marker must contain socket path");
        assert!(content.contains("timestamp_secs="), "Marker must contain timestamp");

        let _ = std::fs::remove_dir_all(&temp_dir);
        std::env::remove_var("ONURON_READY_DIR");
    }

    #[test]
    fn test_notify_ready_propagates_write_error() {
        // Create a regular file and use a subpath of it as ONURON_READY_DIR so create_dir_all fails
        let blocker_file = std::env::temp_dir().join(format!("onuron_blocker_{}", std::process::id()));
        std::fs::write(&blocker_file, "blocking").expect("Must create blocker file");

        let impossible_dir = blocker_file.join("subfolder_impossible");
        std::env::set_var("ONURON_READY_DIR", &impossible_dir);

        let res = notify_ready("fail_daemon", None);
        assert!(res.is_err(), "notify_ready must propagate error when directory cannot be created");

        let _ = std::fs::remove_file(&blocker_file);
        std::env::remove_var("ONURON_READY_DIR");
    }
}

