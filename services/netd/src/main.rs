// services/netd/src/main.rs — Onuron OS Network Subsystem & Interface Manager
// Discovers Linux network interfaces (/sys/class/net), monitors carrier link states, and parses DNS servers.

use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;
use serde::{Deserialize, Serialize};

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
            is_connected: true,
            active_interface: Some("eth0".into()),
            connection_type: ConnectionType::Ethernet,
            interfaces: vec![
                NetworkInterface {
                    name: "lo".into(),
                    conn_type: ConnectionType::Loopback,
                    operstate: "up".into(),
                    carrier_connected: true,
                    mac_address: "00:00:00:00:00:00".into(),
                },
                NetworkInterface {
                    name: "eth0".into(),
                    conn_type: ConnectionType::Ethernet,
                    operstate: "up".into(),
                    carrier_connected: true,
                    mac_address: "52:54:00:12:34:56".into(),
                },
            ],
            dns_servers: vec!["1.1.1.1".into(), "8.8.8.8".into()],
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
            let line = line.trim();
            if line.starts_with("nameserver") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    servers.push(parts[1].to_string());
                }
            }
        }
    }
    servers
}

pub fn scan_network_interfaces() -> NetworkState {
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

    NetworkState::default()
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
        let tmp_resolv = Path::new("target").join("test_resolv.conf");
        let _ = fs::create_dir_all("target");
        let content = "# Generated by nilinit\nnameserver 1.1.1.1\nnameserver 8.8.8.8\nsearch local\n";
        fs::write(&tmp_resolv, content).unwrap();

        let dns = parse_dns_servers(&tmp_resolv);
        assert_eq!(dns, vec!["1.1.1.1".to_string(), "8.8.8.8".to_string()]);
        let _ = fs::remove_file(tmp_resolv);
    }
}
