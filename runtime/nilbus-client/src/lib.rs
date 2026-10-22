// runtime/nilbus-client/src/lib.rs — Client library for NilOS SoftBus
//
// Communicates with the nilbus daemon via /run/nilos/bus.sock and maintains
// the local paired-peer registry at /data/nilos/softbus_peers.json.
// Protocol: plain-text, newline-terminated commands.
//
//   LIST_PEERS             → "id@ip:port,id@ip:port,…\n"  or "EMPTY\n"
//   HANDOFF <p> <a> <hex>  → "OK HANDOFF→…\n"            or "ERR: …\n"
//   PING <peer>            → "OK peer=… reachable\n"      or "FAIL …\n"

use std::fs::{self, File};
use std::io::Write;
#[cfg(unix)]
use std::io::{BufRead, BufReader};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub const BUS_SOCK: &str = "/run/nilos/bus.sock";
pub const PEERS_FILE: &str = "/data/nilos/softbus_peers.json";

/// A discovered or paired peer returned by `get_nearby_peers()`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Peer {
    pub device_id: String,
    pub addr: String,
    pub quic_port: u16,
}

pub struct SoftBusClient;

impl SoftBusClient {
    pub fn is_daemon_active() -> bool {
        Path::new(BUS_SOCK).exists()
    }

    pub fn default_peers_path() -> PathBuf {
        let p = Path::new(PEERS_FILE);
        if p.parent().map(|d| d.exists()).unwrap_or(false) || Path::new("/data").exists() {
            p.to_path_buf()
        } else {
            std::env::temp_dir().join("onuron_softbus_peers.json")
        }
    }

    pub fn load_paired_peers_from(path: &Path) -> Vec<Peer> {
        if !path.exists() {
            return Vec::new();
        }
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str::<Vec<Peer>>(&text).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    pub fn save_paired_peers_to(path: &Path, peers: &[Peer]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(peers).map_err(|e| e.to_string())?;
        {
            let mut f = File::create(&tmp).map_err(|e| e.to_string())?;
            f.write_all(&bytes).map_err(|e| e.to_string())?;
            f.sync_all().map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    pub fn pair_peer_to(
        path: &Path,
        device_id: &str,
        addr: &str,
        quic_port: u16,
    ) -> Result<Peer, String> {
        let id = device_id.trim();
        let addr_clean = addr.trim();
        if id.is_empty() || addr_clean.is_empty() {
            return Err("device_id and address cannot be empty".into());
        }
        let mut peers = Self::load_paired_peers_from(path);
        let peer = Peer {
            device_id: id.to_string(),
            addr: addr_clean.to_string(),
            quic_port,
        };
        if let Some(existing) = peers.iter_mut().find(|p| p.device_id == id) {
            *existing = peer.clone();
        } else {
            peers.push(peer.clone());
        }
        Self::save_paired_peers_to(path, &peers)?;
        Ok(peer)
    }

    pub fn unpair_peer_from(path: &Path, device_id: &str) -> Result<bool, String> {
        let mut peers = Self::load_paired_peers_from(path);
        let prev = peers.len();
        peers.retain(|p| p.device_id != device_id.trim());
        if peers.len() < prev {
            Self::save_paired_peers_to(path, &peers)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// Returns the list of peers currently known to the local nilbus daemon,
    /// merged with locally paired peers on disk.
    pub fn get_nearby_peers() -> Result<Vec<Peer>, String> {
        #[cfg_attr(not(unix), allow(unused_mut))]
        let mut combined = Self::load_paired_peers_from(&Self::default_peers_path());

        #[cfg(unix)]
        {
            if let Ok(mut stream) = UnixStream::connect(BUS_SOCK) {
                stream.set_read_timeout(Some(Duration::from_secs(2))).ok();
                if stream.write_all(b"LIST_PEERS\n").is_ok() {
                    let mut response = String::new();
                    if BufReader::new(&stream).read_line(&mut response).is_ok() {
                        let response = response.trim();
                        if response != "EMPTY" && !response.is_empty() {
                            for entry in response.split(',').filter(|s| !s.is_empty()) {
                                let peer = if let Some((id, addr_port)) = entry.split_once('@') {
                                    let (addr, port) =
                                        addr_port.rsplit_once(':').unwrap_or((addr_port, "7979"));
                                    Peer {
                                        device_id: id.to_string(),
                                        addr: addr.to_string(),
                                        quic_port: port.parse().unwrap_or(7979),
                                    }
                                } else {
                                    Peer {
                                        device_id: entry.to_string(),
                                        addr: String::new(),
                                        quic_port: 7979,
                                    }
                                };
                                if !combined.iter().any(|p| p.device_id == peer.device_id) {
                                    combined.push(peer);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(combined)
    }

    /// Send a handoff request to a peer.
    /// `payload` is forwarded verbatim via QUIC to `peer_id`.
    /// Returns the daemon's response line.
    pub fn send_handoff(peer_id: &str, app_id: &str, payload: &[u8]) -> Result<String, String> {
        #[cfg(unix)]
        {
            if let Ok(mut stream) = UnixStream::connect(BUS_SOCK) {
                let hex_payload = hex_encode(payload);
                let cmd = format!("HANDOFF {peer_id} {app_id} {hex_payload}\n");
                stream.set_read_timeout(Some(Duration::from_secs(10))).ok();
                stream
                    .write_all(cmd.as_bytes())
                    .map_err(|e| e.to_string())?;

                let mut response = String::new();
                BufReader::new(&stream)
                    .read_line(&mut response)
                    .map_err(|e| e.to_string())?;

                let response = response.trim().to_string();
                if response.starts_with("ERR") {
                    return Err(response);
                } else {
                    return Ok(response);
                }
            }
        }

        Err(format!(
            "SoftBus daemon ({BUS_SOCK}) is offline; cannot handoff to {peer_id}/{app_id} ({} bytes)",
            payload.len()
        ))
    }

    /// Ping a peer to check if it is currently reachable.
    pub fn ping(peer_id: &str) -> bool {
        #[cfg(unix)]
        {
            let cmd = format!("PING {peer_id}\n");
            if let Ok(mut stream) = UnixStream::connect(BUS_SOCK) {
                let _ = stream.set_read_timeout(Some(Duration::from_secs(3)));
                if stream.write_all(cmd.as_bytes()).is_ok() {
                    let mut resp = String::new();
                    let _ = BufReader::new(&stream).read_line(&mut resp);
                    return resp.trim().starts_with("OK");
                }
            }
        }
        let _ = peer_id;
        false
    }
}

// ── Minimal hex encoder (avoids pulling in `hex` crate in this lib) ──────────
#[cfg_attr(not(unix), allow(dead_code))]
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pair_load_and_unpair_peers() {
        let dir = std::env::temp_dir().join("onuron_softbus_client_test");
        let _ = fs::remove_dir_all(&dir);
        let file = dir.join("peers.json");

        assert!(SoftBusClient::load_paired_peers_from(&file).is_empty());

        let p =
            SoftBusClient::pair_peer_to(&file, "onuron-tablet-1", "192.168.1.42", 7979).unwrap();
        assert_eq!(p.device_id, "onuron-tablet-1");
        assert_eq!(p.addr, "192.168.1.42");

        let list = SoftBusClient::load_paired_peers_from(&file);
        assert_eq!(list.len(), 1);

        assert!(SoftBusClient::unpair_peer_from(&file, "onuron-tablet-1").unwrap());
        assert!(SoftBusClient::load_paired_peers_from(&file).is_empty());

        let _ = fs::remove_dir_all(&dir);
    }
}
