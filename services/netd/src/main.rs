// services/netd/src/main.rs — Onuron OS Network Subsystem & Interface Manager
// Discovers Linux network interfaces (/sys/class/net), monitors carrier link states, and parses DNS servers.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use std::net::IpAddr;
use serde::{Deserialize, Serialize};
use nilprotocol::{Frame, MessageType};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum ConnectionType {
    None,
    Ethernet,
    Wifi,
    Cellular,
    Loopback,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NetworkInterface {
    pub name: String,
    pub conn_type: ConnectionType,
    pub operstate: String,          // "up", "down", "unknown"
    pub carrier_connected: bool,    // true if physical link carrier is detected
    pub mac_address: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct NetworkState {
    pub is_connected: bool,
    pub active_interface: Option<String>,
    pub connection_type: ConnectionType,
    pub interfaces: Vec<NetworkInterface>,
    pub dns_servers: Vec<String>,
    pub is_simulated: bool,
}

impl Default for NetworkState {
    fn default() -> Self {
        Self {
            // Never report a fabricated link/DNS configuration as live.
            is_connected: false,
            active_interface: None,
            connection_type: ConnectionType::None,
            interfaces: Vec::new(),
            dns_servers: Vec::new(),
            is_simulated: true,
        }
    }
}

pub fn classify_interface(name: &str) -> ConnectionType {
    let lower = name.to_lowercase();
    if lower == "lo" {
        ConnectionType::Loopback
    } else if lower.starts_with("wl") || lower.contains("wifi") || lower.contains("wlan") {
        ConnectionType::Wifi
    } else if lower.starts_with("rmnet") || lower.starts_with("wwan") || lower.starts_with("usb") {
        ConnectionType::Cellular
    } else if lower.starts_with("eth") || lower.starts_with("en") || lower.starts_with("virt") {
        ConnectionType::Ethernet
    } else {
        ConnectionType::None
    }
}

pub fn parse_dns_servers(resolv_conf_path: &Path) -> Vec<String> {
    let mut servers = Vec::new();
    if let Ok(content) = fs::read_to_string(resolv_conf_path) {
        for line in content.lines() {
            let mut parts = line.split('#').next().unwrap_or("").split_whitespace();
            if parts.next() != Some("nameserver") { continue; }
            if let Some(address) = parts.next() {
                if address.parse::<IpAddr>().is_ok() && !servers.iter().any(|s| s == address) {
                    servers.push(address.to_string());
                }
            }
        }
    }
    servers
}

pub fn scan_network_interfaces() -> NetworkState {
    let backend = nilhal::detect_backend();

    if backend == nilhal::BackendType::Android {
        let hal = nilhal::NilHal::auto();
        let st = hal.network.get_state();
        let conn_t = match st.connection_type {
            nilhal::traits::ConnectionType::Wifi => ConnectionType::Wifi,
            nilhal::traits::ConnectionType::Cellular => ConnectionType::Cellular,
            nilhal::traits::ConnectionType::Ethernet => ConnectionType::Ethernet,
            nilhal::traits::ConnectionType::Loopback => ConnectionType::Loopback,
            nilhal::traits::ConnectionType::None => ConnectionType::None,
        };

        let interfaces = st.active_interface.as_ref().map(|name| vec![NetworkInterface {
            name: name.clone(),
            conn_type: conn_t.clone(),
            operstate: if st.is_connected { "up" } else { "down" }.into(),
            carrier_connected: st.is_connected,
            // The old hard-coded MAC address was fabricated device data.
            mac_address: String::new(),
        }]).unwrap_or_default();
        return NetworkState {
            is_connected: st.is_connected,
            active_interface: st.active_interface,
            connection_type: conn_t,
            interfaces,
            dns_servers: st.dns_servers,
            is_simulated: false,
        };
    }

    let net_dir = Path::new("/sys/class/net");
    if net_dir.exists() {
        if let Ok(entries) = fs::read_dir(net_dir) {
            let mut ifaces = Vec::new();
            let mut active_iface = None;
            let mut active_type = ConnectionType::None;

            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                let dir = entry.path();

                let operstate = fs::read_to_string(dir.join("operstate"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "unknown".into());

                let carrier_connected = fs::read_to_string(dir.join("carrier"))
                    .ok()
                    .and_then(|s| s.trim().parse::<u8>().ok())
                    .map(|v| v == 1)
                    .unwrap_or(false);

                let mac_address = fs::read_to_string(dir.join("address"))
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|_| "00:00:00:00:00:00".into());

                let conn_type = classify_interface(&name);

                // Check if this interface is an active external connection
                if conn_type != ConnectionType::Loopback && (carrier_connected || operstate == "up") {
                    if active_iface.is_none() {
                        active_iface = Some(name.clone());
                        active_type = conn_type.clone();
                    }
                }

                ifaces.push(NetworkInterface {
                    name,
                    conn_type,
                    operstate,
                    carrier_connected,
                    mac_address,
                });
            }

            let dns_servers = parse_dns_servers(Path::new("/etc/resolv.conf"));
            let is_connected = active_iface.is_some();

            return NetworkState {
                is_connected,
                active_interface: active_iface,
                connection_type: active_type,
                interfaces: ifaces,
                dns_servers,
                is_simulated: false,
            };
        }
    }

    // Lack of Linux sysfs (or permission to read it) is not evidence of a link.
    NetworkState::default()
}

pub fn handle_ipc_request(frame: &Frame) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),
        MessageType::NetGetState => {
            let state = scan_network_interfaces();
            let json = serde_json::to_vec(&state).unwrap_or_default();
            Frame::new(MessageType::NetStateInfo, frame.request_id, json)
        }
        MessageType::NetScanWifi => {
            let mut hal = nilhal::NilHal::auto();
            let aps = hal.network.scan_wifi().unwrap_or_default();
            let json = serde_json::to_vec(&aps).unwrap_or_default();
            Frame::new(MessageType::NetStateInfo, frame.request_id, json)
        }
        _ => Frame::new(
            MessageType::ErrorResponse,
            frame.request_id,
            b"unsupported message type".to_vec(),
        ),
    }
}

fn main() {
    println!("\x1b[1;36m[netd]\x1b[0m Onuron OS Network Subsystem Initializing...");

    let _ = fs::create_dir_all("/run/onuron");

    let initial_state = scan_network_interfaces();
    println!(
        "\x1b[1;32m[netd] [  OK  ]\x1b[0m Network Status: connected={}, active_iface={:?}, type={:?} [simulated={}]",
        initial_state.is_connected, initial_state.active_interface, initial_state.connection_type, initial_state.is_simulated
    );
    for iface in &initial_state.interfaces {
        println!("  • {:<10} {:<10} (state: {}, carrier: {}) [{}]",
            iface.name, format!("{:?}", iface.conn_type), iface.operstate, iface.carrier_connected, iface.mac_address);
    }

    // Monitor network link status periodically
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(15));
            let state = scan_network_interfaces();
            if !state.is_connected {
                println!("[netd] [WARN] No active network link detected.");
            }
        }
    });

    #[cfg(unix)]
    {
        thread::spawn(move || {
            let listener = match nilsd::first_listener_or_bind("/run/onuron/net.sock") {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[netd] Failed to bind IPC socket /run/onuron/net.sock: {}", e);
                    return;
                }
            };
            println!("\x1b[1;32m[netd] [  OK  ]\x1b[0m IPC Server listening on /run/onuron/net.sock");

            for stream in listener.incoming() {
                if let Ok(mut sock) = stream {
                    thread::spawn(move || {
                        while let Ok(frame) = Frame::read_from(&mut sock) {
                            let resp = handle_ipc_request(&frame);
                            if let Err(e) = resp.write_to(&mut sock) {
                                eprintln!("[netd] IPC send error: {}", e);
                                break;
                            }
                        }
                    });
                }
            }
        });
    }

    println!("\x1b[1;32m[netd] [  OK  ]\x1b[0m Network manager active (/run/onuron/net.sock)");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interface_classification() {
        assert_eq!(classify_interface("lo"), ConnectionType::Loopback);
        assert_eq!(classify_interface("wlan0"), ConnectionType::Wifi);
        assert_eq!(classify_interface("wlp2s0"), ConnectionType::Wifi);
        assert_eq!(classify_interface("eth0"), ConnectionType::Ethernet);
        assert_eq!(classify_interface("enp0s3"), ConnectionType::Ethernet);
        assert_eq!(classify_interface("rmnet_data0"), ConnectionType::Cellular);
        assert_eq!(classify_interface("wwan0"), ConnectionType::Cellular);
    }

    #[test]
    fn test_parse_resolv_conf() {
        let tmp_resolv = std::env::temp_dir().join(format!("netd-resolv-{}-{}.conf", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let content = "# Generated by nilinit\nnameserver 1.1.1.1\nnameserver 8.8.8.8 # ipv4\nnameserver 2001:4860:4860::8888\nnameserverx 9.9.9.9\nnameserver not-an-ip\nnameserver 1.1.1.1\nsearch local\n";
        fs::write(&tmp_resolv, content).unwrap();

        let dns = parse_dns_servers(&tmp_resolv);
        assert_eq!(dns, vec!["1.1.1.1", "8.8.8.8", "2001:4860:4860::8888"]);
        let _ = fs::remove_file(tmp_resolv);
    }

    #[test]
    fn fallback_network_state_does_not_claim_connectivity() {
        let state = NetworkState::default();
        assert!(!state.is_connected);
        assert!(state.active_interface.is_none());
        assert!(state.dns_servers.is_empty());
        assert!(state.is_simulated);
    }

    #[test]
    fn test_net_ipc_ping_and_state() {
        // 1. Ping
        let ping_frame = Frame::new(MessageType::Ping, 1, vec![]);
        let pong_frame = handle_ipc_request(&ping_frame);
        assert_eq!(pong_frame.message_type, u16::from(MessageType::Pong));
        assert_eq!(pong_frame.payload, b"pong");

        // 2. NetGetState
        let state_frame = Frame::new(MessageType::NetGetState, 2, vec![]);
        let resp_frame = handle_ipc_request(&state_frame);
        assert_eq!(resp_frame.message_type, u16::from(MessageType::NetStateInfo));
        let state: NetworkState = serde_json::from_slice(&resp_frame.payload).unwrap();
        assert_eq!(state.is_connected, scan_network_interfaces().is_connected);

        // 3. NetScanWifi
        let scan_frame = Frame::new(MessageType::NetScanWifi, 3, vec![]);
        let scan_resp = handle_ipc_request(&scan_frame);
        assert_eq!(scan_resp.message_type, u16::from(MessageType::NetStateInfo));
    }
}
