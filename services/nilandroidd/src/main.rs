// services/nilandroidd/src/main.rs — Android Container LifeCycle & Binder-Shim Bridge
// Governed by OnuronOS Architecture: the guest container is isolated via
// namespaces and cgroups, and its lifecycle is driven by real child processes.
//
// IPC surface on /run/nilos/android.sock (canonical ONUR frames):
//   Ping                 → Pong
//   ServiceStatusRequest → ServiceStatusResponse (real container state)
//   Custom(0x0B01) StartContainer  → {"ok":true,"pid":N} | {"ok":false,"error":"…"}
//   Custom(0x0B02) StopContainer   → {"ok":true}
//   Custom(0x0B03) ContainerStatus → full status JSON
//   Custom(0x0B04) StartActivity   → forwarded to the in-container agent

pub mod container;
pub mod isolation;

use container::{ContainerLayout, ContainerManager, ContainerState};
use isolation::ContainerProfile;
use nilprotocol::{Frame, MessageType};
#[cfg(unix)]
use std::io::{Read, Write};
#[cfg(unix)]
use std::os::unix::net::UnixListener;
use std::sync::{Arc, Mutex};

/// OnuronOS extension message types for the Android bridge (outside the
/// canonical 0x0001–0x0AFF range so they cannot collide with core services).
pub const MSG_START_CONTAINER: u16 = 0x0B01;
pub const MSG_STOP_CONTAINER: u16 = 0x0B02;
pub const MSG_CONTAINER_STATUS: u16 = 0x0B03;
pub const MSG_START_ACTIVITY: u16 = 0x0B04;

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

    let manager = Arc::new(Mutex::new(ContainerManager::new(
        ContainerLayout::default(),
        profile,
    )));

    {
        let mgr = manager.lock().unwrap();
        match mgr.state() {
            ContainerState::Unavailable { reason } => println!(
                "\x1b[1;33m[nilandroidd] [ WARN ]\x1b[0m Android container unavailable: {}",
                reason
            ),
            other => println!(
                "\x1b[1;32m[nilandroidd] [  OK  ]\x1b[0m Android container state: {}",
                other.label()
            ),
        }
    }

    // Reap a container that died without anyone asking, so the reported state
    // never drifts from reality.
    {
        let mgr = Arc::clone(&manager);
        std::thread::spawn(move || loop {
            std::thread::sleep(std::time::Duration::from_secs(2));
            if let Ok(mut m) = mgr.lock() {
                m.refresh();
            }
        });
    }

    #[cfg(unix)]
    {
        let _ = std::fs::remove_file("/run/nilos/android.sock");
        if let Ok(listener) = UnixListener::bind("/run/nilos/android.sock") {
            println!(
                "\x1b[1;32m[nilandroidd] [  OK  ]\x1b[0m Listening on /run/nilos/android.sock"
            );
            let _ = nilsd::notify_ready("nilandroidd", Some("/run/nilos/android.sock"));
            let policy = nilsd::auth::load_default_policy();
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    // C1: the Android bridge is restricted by IPC policy.
                    if !nilsd::auth::authorize_stream(&policy, "nilandroidd", &s) {
                        continue;
                    }
                    let mgr = Arc::clone(&manager);
                    std::thread::spawn(move || {
                        let mut buf = [0u8; 4096];
                        while let Ok(n) = s.read(&mut buf) {
                            if n == 0 {
                                break;
                            }
                            let resp = if n >= 4 && buf[..4] == nilprotocol::PROTOCOL_MAGIC {
                                let mut cursor = std::io::Cursor::new(&buf[..n]);
                                match Frame::read_from(&mut cursor) {
                                    Ok(frame) => handle_ipc_request(&frame, &mgr),
                                    Err(e) => Frame::new(
                                        MessageType::ErrorResponse,
                                        0,
                                        format!("frame parse error: {e}").into_bytes(),
                                    ),
                                }
                            } else {
                                // Legacy line protocol retained for the
                                // nilandroid-client compatibility path.
                                let cmd = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                                handle_legacy_line(&cmd, &mgr)
                            };
                            if resp.write_to(&mut s).is_err() {
                                break;
                            }
                        }
                    });
                }
            }
        } else {
            eprintln!(
                "\x1b[1;31m[nilandroidd] [ FAIL ]\x1b[0m Cannot bind /run/nilos/android.sock"
            );
        }
    }
    #[cfg(not(unix))]
    {
        println!("[nilandroidd] Android container management requires Linux; bridge idle.");
    }
}

/// Handle a canonical framed request.
pub fn handle_ipc_request(frame: &Frame, manager: &Arc<Mutex<ContainerManager>>) -> Frame {
    match MessageType::from(frame.message_type) {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),
        MessageType::ServiceStatusRequest => {
            let mgr = match manager.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let payload = nilprotocol::ServiceStatusPayload {
                service_name: "nilandroidd".to_string(),
                is_ready: true,
                // The bridge itself is real; whether a container is running is
                // reported through MSG_CONTAINER_STATUS, not as "simulated".
                is_simulated: false,
                backend_name: format!("container:{}", mgr.state().label()),
                uptime_secs: 0,
                request_count: 1,
                last_error: None,
            };
            Frame::with_json(
                MessageType::ServiceStatusResponse,
                frame.request_id,
                &payload,
            )
            .unwrap_or_else(|_| {
                Frame::new(
                    MessageType::ErrorResponse,
                    frame.request_id,
                    b"encode error".to_vec(),
                )
            })
        }
        MessageType::Custom(code) if code == MSG_START_CONTAINER => {
            let mut mgr = match manager.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let body = match mgr.start() {
                Ok(pid) => serde_json::json!({ "ok": true, "pid": pid }).to_string(),
                Err(e) => serde_json::json!({ "ok": false, "error": e }).to_string(),
            };
            Frame::new(frame.message_type, frame.request_id, body.into_bytes())
        }
        MessageType::Custom(code) if code == MSG_STOP_CONTAINER => {
            let mut mgr = match manager.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let body = match mgr.stop() {
                Ok(()) => serde_json::json!({ "ok": true }).to_string(),
                Err(e) => serde_json::json!({ "ok": false, "error": e }).to_string(),
            };
            Frame::new(frame.message_type, frame.request_id, body.into_bytes())
        }
        MessageType::Custom(code) if code == MSG_CONTAINER_STATUS => {
            let mgr = match manager.lock() {
                Ok(g) => g,
                Err(p) => p.into_inner(),
            };
            let (state, pid) = match mgr.state() {
                ContainerState::Running { pid } => ("running", Some(*pid)),
                ContainerState::Stopped => ("stopped", None),
                ContainerState::Unavailable { .. } => ("unavailable", None),
                ContainerState::Failed { .. } => ("failed", None),
            };
            let body = serde_json::json!({
                "state": state,
                "pid": pid,
                "status_line": mgr.status_line(),
                "rootfs": mgr.layout.rootfs.display().to_string(),
            })
            .to_string();
            Frame::new(frame.message_type, frame.request_id, body.into_bytes())
        }
        MessageType::Custom(code) if code == MSG_START_ACTIVITY => {
            // The intent is forwarded to the in-container agent, which speaks to
            // Android's `am` through the binder shim.
            let request = String::from_utf8_lossy(&frame.payload).to_string();
            match forward_to_agent(&request) {
                Ok(reply) => Frame::new(frame.message_type, frame.request_id, reply.into_bytes()),
                Err(e) => Frame::new(
                    MessageType::ErrorResponse,
                    frame.request_id,
                    format!("agent unavailable: {e}").into_bytes(),
                ),
            }
        }
        _ => Frame::new(
            MessageType::ErrorResponse,
            frame.request_id,
            b"unsupported message type".to_vec(),
        ),
    }
}

/// Legacy newline command path used by `runtime/nilandroid-client`.
fn handle_legacy_line(cmd: &str, manager: &Arc<Mutex<ContainerManager>>) -> Frame {
    let mut parts = cmd.split_whitespace();
    let verb = parts.next().unwrap_or("").to_uppercase();
    let mut mgr = match manager.lock() {
        Ok(g) => g,
        Err(p) => p.into_inner(),
    };

    let body = match verb.as_str() {
        "PING" => "OK pong".to_string(),
        "START" => match mgr.start() {
            Ok(pid) => format!("OK started pid={pid}"),
            Err(e) => format!("ERR {e}"),
        },
        "STOP" => match mgr.stop() {
            Ok(()) => "OK stopped".to_string(),
            Err(e) => format!("ERR {e}"),
        },
        "STATUS" => format!("OK {}", mgr.status_line()),
        "START_ACTIVITY" => {
            let rest: Vec<&str> = parts.collect();
            match forward_to_agent(&rest.join(" ")) {
                Ok(r) => format!("OK {r}"),
                Err(e) => format!("ERR {e}"),
            }
        }
        other => format!("ERR unknown command: {other}"),
    };
    // The legacy client reads raw bytes, so wrap them in a frame it ignores.
    Frame::new(MessageType::Pong, 0, body.into_bytes())
}

/// Send one command to the in-container agent over its Unix socket.
///
/// The agent runs *inside* the container at `/dev/socket/nilagent`; on the host
/// side that socket is visible under the container's runtime directory. When the
/// agent is not running this returns an error rather than pretending success.
fn forward_to_agent(request: &str) -> Result<String, String> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let candidates = [
            "/run/onuron/android/dev/socket/nilagent",
            "/data/android/rootfs/dev/socket/nilagent",
        ];
        for path in candidates {
            if let Ok(mut stream) = UnixStream::connect(path) {
                let line = format!("{request}\n");
                stream
                    .write_all(line.as_bytes())
                    .map_err(|e| e.to_string())?;
                let mut resp = String::new();
                stream
                    .read_to_string(&mut resp)
                    .map_err(|e| e.to_string())?;
                return Ok(resp.trim().to_string());
            }
        }
        Err("no nilagent socket (container not running?)".to_string())
    }
    #[cfg(not(unix))]
    {
        let _ = request;
        Err("Unix sockets unavailable on this host".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_manager() -> Arc<Mutex<ContainerManager>> {
        Arc::new(Mutex::new(ContainerManager::new(
            ContainerLayout {
                rootfs: std::path::PathBuf::from("/definitely/not/a/rootfs/onuron-test"),
                ..Default::default()
            },
            ContainerProfile::default(),
        )))
    }

    #[test]
    fn test_ipc_ping() {
        let ping = Frame::new(MessageType::Ping, 42, vec![]);
        let pong = handle_ipc_request(&ping, &test_manager());
        assert_eq!(pong.message_type, u16::from(MessageType::Pong));
        assert_eq!(pong.payload, b"pong");
    }

    #[test]
    fn status_reports_the_real_absence_of_a_container() {
        let frame = Frame::new(MessageType::Custom(MSG_CONTAINER_STATUS), 1, vec![]);
        let resp = handle_ipc_request(&frame, &test_manager());
        let body = String::from_utf8_lossy(&resp.payload).to_string();
        assert!(body.contains("unavailable"), "got: {body}");
        assert!(body.contains("\"pid\":null"), "got: {body}");
    }

    #[test]
    fn start_fails_loudly_when_no_rootfs_exists() {
        let frame = Frame::new(MessageType::Custom(MSG_START_CONTAINER), 2, vec![]);
        let resp = handle_ipc_request(&frame, &test_manager());
        let body = String::from_utf8_lossy(&resp.payload).to_string();
        assert!(body.contains("\"ok\":false"), "got: {body}");
        assert!(body.contains("rootfs"), "got: {body}");
    }

    #[test]
    fn service_status_is_not_marked_simulated() {
        let frame = Frame::new(MessageType::ServiceStatusRequest, 3, vec![]);
        let resp = handle_ipc_request(&frame, &test_manager());
        let status: nilprotocol::ServiceStatusPayload = resp.parse_json().unwrap();
        assert_eq!(status.service_name, "nilandroidd");
        assert!(!status.is_simulated);
        assert!(status.backend_name.starts_with("container:"));
    }

    #[test]
    fn legacy_status_line_is_answered() {
        let resp = handle_legacy_line("STATUS", &test_manager());
        let body = String::from_utf8_lossy(&resp.payload).to_string();
        assert!(body.starts_with("OK UNAVAILABLE"), "got: {body}");
    }
}
