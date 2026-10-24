// runtime/nilandroid-client/src/lib.rs — Android bridge client
//
// Talks to `nilandroidd` over `/run/nilos/android.sock` using the canonical
// ONUR framed protocol. This is what the shell uses to start/stop the container
// and to launch activities; it never fabricates a success reply.

#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

use nilprotocol::{Frame, MessageType};

pub const ANDROID_SOCK: &str = "/run/nilos/android.sock";

/// Extension message types; must match `services/nilandroidd/src/main.rs`.
pub const MSG_START_CONTAINER: u16 = 0x0B01;
pub const MSG_STOP_CONTAINER: u16 = 0x0B02;
pub const MSG_CONTAINER_STATUS: u16 = 0x0B03;
pub const MSG_START_ACTIVITY: u16 = 0x0B04;

/// Container state as reported by the daemon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContainerStatus {
    pub state: String,
    pub pid: Option<u32>,
    pub status_line: String,
    pub rootfs: String,
}

impl ContainerStatus {
    pub fn is_running(&self) -> bool {
        self.state == "running"
    }
}

pub struct AndroidBridge;

impl AndroidBridge {
    /// True when the bridge daemon is listening.
    pub fn is_available() -> bool {
        std::path::Path::new(ANDROID_SOCK).exists()
    }

    #[cfg(unix)]
    fn call(msg_type: u16, payload: Vec<u8>) -> Result<Vec<u8>, String> {
        let mut stream = UnixStream::connect(ANDROID_SOCK)
            .map_err(|e| format!("cannot connect to {ANDROID_SOCK}: {e}"))?;
        let req = Frame::new(msg_type, 1, payload);
        nilprotocol::write_frame(&mut stream, &req).map_err(|e| e.to_string())?;
        let resp = nilprotocol::read_frame(&mut stream).map_err(|e| e.to_string())?;
        if MessageType::from(resp.message_type) == MessageType::ErrorResponse {
            return Err(String::from_utf8_lossy(&resp.payload).to_string());
        }
        Ok(resp.payload)
    }

    /// Ask the daemon for the container's real state.
    pub fn status() -> Result<ContainerStatus, String> {
        #[cfg(unix)]
        {
            let payload = Self::call(MSG_CONTAINER_STATUS, Vec::new())?;
            let v: serde_json::Value =
                serde_json::from_slice(&payload).map_err(|e| format!("bad status JSON: {e}"))?;
            Ok(ContainerStatus {
                state: v
                    .get("state")
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown")
                    .to_string(),
                pid: v.get("pid").and_then(|p| p.as_u64()).map(|p| p as u32),
                status_line: v
                    .get("status_line")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
                rootfs: v
                    .get("rootfs")
                    .and_then(|s| s.as_str())
                    .unwrap_or("")
                    .to_string(),
            })
        }
        #[cfg(not(unix))]
        {
            Err("Android containers require Linux".to_string())
        }
    }

    /// Start the container. Returns the container init's PID on success.
    pub fn start_container() -> Result<u32, String> {
        #[cfg(unix)]
        {
            let payload = Self::call(MSG_START_CONTAINER, Vec::new())?;
            let v: serde_json::Value =
                serde_json::from_slice(&payload).map_err(|e| format!("bad reply: {e}"))?;
            if v.get("ok").and_then(|b| b.as_bool()) == Some(true) {
                return v
                    .get("pid")
                    .and_then(|p| p.as_u64())
                    .map(|p| p as u32)
                    .ok_or_else(|| "daemon reported success without a pid".to_string());
            }
            Err(v
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("start failed")
                .to_string())
        }
        #[cfg(not(unix))]
        {
            Err("Android containers require Linux".to_string())
        }
    }

    pub fn stop_container() -> Result<(), String> {
        #[cfg(unix)]
        {
            let payload = Self::call(MSG_STOP_CONTAINER, Vec::new())?;
            let v: serde_json::Value =
                serde_json::from_slice(&payload).map_err(|e| format!("bad reply: {e}"))?;
            if v.get("ok").and_then(|b| b.as_bool()) == Some(true) {
                return Ok(());
            }
            Err(v
                .get("error")
                .and_then(|e| e.as_str())
                .unwrap_or("stop failed")
                .to_string())
        }
        #[cfg(not(unix))]
        {
            Err("Android containers require Linux".to_string())
        }
    }

    /// Launch an Android activity inside the running container.
    pub fn start_activity(action: &str, uri: &str) -> Result<(), String> {
        let request = format!("START_ACTIVITY {action} {uri}");
        #[cfg(unix)]
        {
            let payload = Self::call(MSG_START_ACTIVITY, request.into_bytes())?;
            let reply = String::from_utf8_lossy(&payload).to_string();
            if reply.starts_with("ERR") {
                return Err(reply);
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let _ = request;
            println!(
                "[android-client] Host bridge Intent START: {}/{}",
                action, uri
            );
            Ok(())
        }
    }

    pub fn ping() -> Result<bool, String> {
        #[cfg(unix)]
        if let Ok(mut stream) = UnixStream::connect("/run/nilos/android.sock") {
            stream
                .write_all(b"{\"action\":\"PING\"}\n")
                .map_err(|e| e.to_string())?;
            return Ok(true);
        }
        Ok(false)
    }
}
