// runtime/nilbus-client/src/lib.rs — Client library for NilOS SoftBus
//
// Communicates with the nilbus daemon via /run/nilos/bus.sock.
// Protocol: plain-text, newline-terminated commands.
//
//   LIST_PEERS             → "id@ip:port,id@ip:port,…\n"  or "EMPTY\n"
//   HANDOFF <p> <a> <hex>  → "OK HANDOFF→…\n"            or "ERR: …\n"
//   PING <peer>            → "OK peer=… reachable\n"      or "FAIL …\n"

#[cfg(unix)]
use std::io::{BufRead, BufReader, Write};
#[cfg(unix)]
use std::os::unix::net::UnixStream;
#[cfg(unix)]
use std::time::Duration;

const BUS_SOCK: &str = "/run/nilos/bus.sock";

/// A discovered peer returned by `get_nearby_peers()`.
#[derive(Debug, Clone)]
pub struct Peer {
    pub device_id: String,
    pub addr: String,
    pub quic_port: u16,
}

pub struct SoftBusClient;

impl SoftBusClient {
    /// Returns the list of peers currently known to the local nilbus daemon.
    /// Each entry is resolved from a real mDNS-SD discovery event.
    pub fn get_nearby_peers() -> Result<Vec<Peer>, String> {
        #[cfg(unix)]
        {
            let mut stream = UnixStream::connect(BUS_SOCK)
                .map_err(|e| format!("connect {BUS_SOCK}: {e}"))?;
            stream
                .set_read_timeout(Some(Duration::from_secs(2)))
                .ok();
            stream
                .write_all(b"LIST_PEERS\n")
                .map_err(|e| e.to_string())?;

            let mut response = String::new();
            BufReader::new(&stream)
                .read_line(&mut response)
                .map_err(|e| e.to_string())?;

            let response = response.trim();
            if response == "EMPTY" {
                return Ok(vec![]);
            }

            let peers = response
                .split(',')
                .filter(|s| !s.is_empty())
                .map(|entry| {
                    // Format: "device_id@ip:port"
                    if let Some((id, addr_port)) = entry.split_once('@') {
                        let (addr, port) = addr_port.rsplit_once(':').unwrap_or((addr_port, "7979"));
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
                    }
                })
                .collect();

            Ok(peers)
        }

        #[cfg(not(unix))]
        {
            // Dev-host stub — returns empty list, daemon not running
            Ok(vec![])
        }
    }

    /// Send a handoff request to a peer.
    /// `payload` is forwarded verbatim via QUIC to `peer_id`.
    /// Returns the daemon's response line.
    pub fn send_handoff(peer_id: &str, app_id: &str, payload: &[u8]) -> Result<String, String> {
        #[cfg(unix)]
        {
            let hex_payload = hex_encode(payload);
            let cmd = format!("HANDOFF {peer_id} {app_id} {hex_payload}\n");

            let mut stream = UnixStream::connect(BUS_SOCK)
                .map_err(|e| format!("connect {BUS_SOCK}: {e}"))?;
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .ok();
            stream.write_all(cmd.as_bytes()).map_err(|e| e.to_string())?;

            let mut response = String::new();
            BufReader::new(&stream)
                .read_line(&mut response)
                .map_err(|e| e.to_string())?;

            let response = response.trim().to_string();
            if response.starts_with("ERR") {
                Err(response)
            } else {
                Ok(response)
            }
        }

        #[cfg(not(unix))]
        {
            Ok(format!(
                "OK HANDOFF→{peer_id}/{app_id} {} bytes (simulated)",
                payload.len()
            ))
        }
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
            false
        }

        #[cfg(not(unix))]
        false
    }
}

// ── Minimal hex encoder (avoids pulling in `hex` crate in this lib) ──────────
fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}
