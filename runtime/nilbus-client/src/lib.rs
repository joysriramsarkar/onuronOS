// runtime/nilbus-client/src/lib.rs — Client library for native NilOS apps
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub struct SoftBusClient;

impl SoftBusClient {
    pub fn get_nearby_peers() -> Result<Vec<String>, String> {
        let mut stream = UnixStream::connect("/run/nilos/bus.sock").map_err(|e| e.to_string())?;
        stream.write_all(b"LIST_PEERS\n").map_err(|e| e.to_string())?;
        let mut buf = [0u8; 1024];
        let n = stream.read(&mut buf).map_err(|e| e.to_string())?;
        let resp = String::from_utf8_lossy(&buf[..n]);
        Ok(resp.trim().split(',').map(|s| s.to_string()).filter(|s| !s.is_empty()).collect())
    }

    pub fn send_handoff(peer: &str, app_id: &str, payload: &[u8]) -> Result<(), String> {
        let mut stream = UnixStream::connect("/run/nilos/bus.sock").map_err(|e| e.to_string())?;
        stream.write_all(format!("HANDOFF {} {} {}\n", peer, app_id, payload.len()).as_bytes()).map_err(|e| e.to_string())?;
        Ok(())
    }
}
