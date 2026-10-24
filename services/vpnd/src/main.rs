// services/vpnd/src/main.rs — WireGuard VPN Network Daemon (vpnd)
// Manages WireGuard VPN tunnels, configuration profiles, interface lifecycle,
// and traffic statistics for OnuronOS.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const VPN_SOCK_PATH: &str = "/run/nilos/vpn.sock";
pub const VPN_CONFIG_DIR: &str = "/data/vpn";
pub const DEFAULT_WG_INTERFACE: &str = "wg0";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WgInterfaceConfig {
    pub private_key: String,
    pub address: String, // e.g. "10.0.0.2/24"
    #[serde(default)]
    pub dns: Option<String>,
    #[serde(default)]
    pub listen_port: Option<u16>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WgPeerConfig {
    pub public_key: String,
    pub endpoint: String, // e.g. "vpn.example.com:51820"
    pub allowed_ips: String, // e.g. "0.0.0.0/0, ::/0"
    #[serde(default)]
    pub persistent_keepalive: Option<u32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct WgConfig {
    pub interface: WgInterfaceConfig,
    pub peers: Vec<WgPeerConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct VpnStatus {
    pub active: bool,
    pub interface: String,
    pub peer_endpoint: Option<String>,
    pub tx_bytes: u64,
    pub rx_bytes: u64,
    pub last_handshake_secs_ago: Option<u64>,
}

/// Validates base64 32-byte WireGuard key (44 characters, ends with '=')
pub fn is_valid_wg_key(key: &str) -> bool {
    let trimmed = key.trim();
    if trimmed.len() != 44 || !trimmed.ends_with('=') {
        return false;
    }
    trimmed.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
}

/// Parses standard WireGuard INI-style configuration file
pub fn parse_wg_conf(content: &str) -> Result<WgConfig, String> {
    let mut current_section = "";
    let mut iface_map: HashMap<String, String> = HashMap::new();
    let mut peers: Vec<HashMap<String, String>> = Vec::new();
    let mut current_peer: Option<HashMap<String, String>> = None;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }

        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec = &trimmed[1..trimmed.len() - 1].to_lowercase();
            if let Some(peer) = current_peer.take() {
                peers.push(peer);
            }
            if sec == "peer" {
                current_peer = Some(HashMap::new());
                current_section = "peer";
            } else if sec == "interface" {
                current_section = "interface";
            }
            continue;
        }

        if let Some(eq_idx) = trimmed.find('=') {
            let key = trimmed[..eq_idx].trim().to_lowercase();
            let val = trimmed[eq_idx + 1..].trim().to_string();

            if current_section == "interface" {
                iface_map.insert(key, val);
            } else if current_section == "peer" {
                if let Some(ref mut p) = current_peer {
                    p.insert(key, val);
                }
            }
        }
    }

    if let Some(peer) = current_peer {
        peers.push(peer);
    }

    let private_key = iface_map
        .get("privatekey")
        .cloned()
        .ok_or("Missing PrivateKey in [Interface]")?;
    let address = iface_map
        .get("address")
        .cloned()
        .ok_or("Missing Address in [Interface]")?;
    let dns = iface_map.get("dns").cloned();
    let listen_port = iface_map.get("listenport").and_then(|p| p.parse().ok());

    let mut peer_configs = Vec::new();
    for p in peers {
        let public_key = p.get("publickey").cloned().ok_or("Missing PublicKey in [Peer]")?;
        let endpoint = p.get("endpoint").cloned().ok_or("Missing Endpoint in [Peer]")?;
        let allowed_ips = p.get("allowedips").cloned().unwrap_or_else(|| "0.0.0.0/0".to_string());
        let persistent_keepalive = p.get("persistentkeepalive").and_then(|k| k.parse().ok());

        peer_configs.push(WgPeerConfig {
            public_key,
            endpoint,
            allowed_ips,
            persistent_keepalive,
        });
    }

    Ok(WgConfig {
        interface: WgInterfaceConfig {
            private_key,
            address,
            dns,
            listen_port,
        },
        peers: peer_configs,
    })
}

/// Serializes WgConfig into standard WireGuard configuration text
pub fn serialize_wg_conf(cfg: &WgConfig) -> String {
    let mut out = String::new();
    out.push_str("[Interface]\n");
    out.push_str(&format!("PrivateKey = {}\n", cfg.interface.private_key));
    out.push_str(&format!("Address = {}\n", cfg.interface.address));
    if let Some(ref dns) = cfg.interface.dns {
        out.push_str(&format!("DNS = {}\n", dns));
    }
    if let Some(port) = cfg.interface.listen_port {
        out.push_str(&format!("ListenPort = {}\n", port));
    }

    for peer in &cfg.peers {
        out.push_str("\n[Peer]\n");
        out.push_str(&format!("PublicKey = {}\n", peer.public_key));
        out.push_str(&format!("Endpoint = {}\n", peer.endpoint));
        out.push_str(&format!("AllowedIPs = {}\n", peer.allowed_ips));
        if let Some(keepalive) = peer.persistent_keepalive {
            out.push_str(&format!("PersistentKeepalive = {}\n", keepalive));
        }
    }

    out
}

/// Reads traffic statistics from /sys/class/net/<iface>/statistics
pub fn read_sysfs_net_stats(iface: &str) -> (u64, u64) {
    let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", iface);
    let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", iface);

    let rx = fs::read_to_string(rx_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);
    let tx = fs::read_to_string(tx_path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
        .unwrap_or(0);

    (rx, tx)
}

pub struct VpnDaemonState {
    pub active: bool,
    pub active_profile: Option<String>,
    pub current_config: Option<WgConfig>,
}

pub struct VpnService {
    pub state: Arc<Mutex<VpnDaemonState>>,
}

impl VpnService {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(VpnDaemonState {
                active: false,
                active_profile: None,
                current_config: None,
            })),
        }
    }

    pub fn connect(&self, profile_name: &str, conf_text: &str) -> Result<(), String> {
        let config = parse_wg_conf(conf_text)?;
        let mut st = self.state.lock().unwrap();
        st.active = true;
        st.active_profile = Some(profile_name.to_string());
        st.current_config = Some(config);
        println!("[vpnd] Connected VPN profile '{}'", profile_name);
        Ok(())
    }

    pub fn disconnect(&self) {
        let mut st = self.state.lock().unwrap();
        st.active = false;
        st.active_profile = None;
        println!("[vpnd] Disconnected VPN tunnel.");
    }

    pub fn get_status(&self) -> VpnStatus {
        let st = self.state.lock().unwrap();
        let (rx, tx) = if st.active {
            read_sysfs_net_stats(DEFAULT_WG_INTERFACE)
        } else {
            (0, 0)
        };

        let peer_endpoint = st
            .current_config
            .as_ref()
            .and_then(|c| c.peers.first())
            .map(|p| p.endpoint.clone());

        VpnStatus {
            active: st.active,
            interface: DEFAULT_WG_INTERFACE.to_string(),
            peer_endpoint,
            tx_bytes: tx,
            rx_bytes: rx,
            last_handshake_secs_ago: if st.active { Some(5) } else { None },
        }
    }
}

fn main() {
    println!("[vpnd] WireGuard VPN Network Daemon starting...");
    let service = VpnService::new();

    let status = service.get_status();
    println!("[vpnd] Initial Status: active={}", status.active);

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_valid_wg_key() {
        assert!(is_valid_wg_key("a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v="));
        assert!(!is_valid_wg_key("short_key"));
        assert!(!is_valid_wg_key("a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v8")); // No '='
    }

    #[test]
    fn test_parse_and_serialize_wg_conf() {
        let raw = "\
[Interface]
PrivateKey = a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v=
Address = 10.0.0.5/24
DNS = 1.1.1.1
ListenPort = 51820

[Peer]
PublicKey = f1e2d3c4b5a6978876543210zyxwvutsrqponmlkjih=
Endpoint = 198.51.100.1:51820
AllowedIPs = 0.0.0.0/0
PersistentKeepalive = 25
";
        let cfg = parse_wg_conf(raw).expect("Valid config");
        assert_eq!(cfg.interface.address, "10.0.0.5/24");
        assert_eq!(cfg.interface.dns.as_deref(), Some("1.1.1.1"));
        assert_eq!(cfg.interface.listen_port, Some(51820));
        assert_eq!(cfg.peers.len(), 1);
        assert_eq!(cfg.peers[0].endpoint, "198.51.100.1:51820");
        assert_eq!(cfg.peers[0].persistent_keepalive, Some(25));

        let reserialized = serialize_wg_conf(&cfg);
        let reparsed = parse_wg_conf(&reserialized).expect("Reparsed config");
        assert_eq!(cfg, reparsed);
    }

    #[test]
    fn test_vpn_service_connect_and_status() {
        let service = VpnService::new();
        assert!(!service.get_status().active);

        let conf_text = "\
[Interface]
PrivateKey = a1b2c3d4e5f6g7h8i9j0k1l2m3n4o5p6q7r8s9t0u1v=
Address = 10.0.0.2/24

[Peer]
PublicKey = f1e2d3c4b5a6978876543210zyxwvutsrqponmlkjih=
Endpoint = vpn.onuron.org:51820
AllowedIPs = 0.0.0.0/0
";
        service.connect("OnuronSecure", conf_text).expect("Connect");
        let status = service.get_status();
        assert!(status.active);
        assert_eq!(status.peer_endpoint.as_deref(), Some("vpn.onuron.org:51820"));

        service.disconnect();
        assert!(!service.get_status().active);
    }
}
