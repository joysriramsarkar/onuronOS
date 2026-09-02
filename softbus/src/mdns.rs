// softbus/src/mdns.rs — Real mDNS-SD peer discovery (Linux/Unix only)
#![cfg(unix)]

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::{Arc, Mutex};

use mdns_sd::{ServiceDaemon, ServiceEvent, ServiceInfo};

#[derive(Debug, Clone)]
pub struct Peer {
    pub device_id: String,
    pub addr: IpAddr,
    pub quic_port: u16,
}

pub type PeerMap = Arc<Mutex<HashMap<String, Peer>>>;

const SERVICE_TYPE: &str = "_nilbus._udp.local.";

pub async fn start_discovery(
    device_id: &str,
    quic_port: u16,
    peers: PeerMap,
) -> Result<(), String> {
    let mdns = ServiceDaemon::new().map_err(|e| format!("mdns daemon: {e}"))?;

    let hostname = format!("{device_id}.local.");
    let service_name = format!("{device_id}.{SERVICE_TYPE}");
    let port_str = quic_port.to_string();
    let properties = [("device_id", device_id), ("quic_port", port_str.as_str())];

    let service = ServiceInfo::new(
        SERVICE_TYPE,
        device_id,
        &hostname,
        "",
        quic_port,
        &properties[..],
    )
    .map_err(|e| format!("ServiceInfo: {e}"))?
    .enable_addr_auto();

    mdns.register(service).map_err(|e| format!("mdns register: {e}"))?;
    println!("[nilbus:mdns] Registered '{service_name}' on port {quic_port}");

    let receiver = mdns.browse(SERVICE_TYPE).map_err(|e| format!("mdns browse: {e}"))?;
    let peers_clone = peers.clone();
    let own_id = device_id.to_string();

    tokio::spawn(async move {
        while let Ok(event) = receiver.recv_async().await {
            match event {
                ServiceEvent::ServiceResolved(info) => {
                    let remote_id = info
                        .get_property_val_str("device_id")
                        .unwrap_or_else(|| info.get_fullname())
                        .to_string();

                    if remote_id == own_id { continue; }

                    let port: u16 = info
                        .get_property_val_str("quic_port")
                        .and_then(|p| p.parse().ok())
                        .unwrap_or(7979);

                    if let Some(&addr) = info.get_addresses().iter().next() {
                        println!("[nilbus:mdns] Discovered peer: {remote_id} @ {addr}:{port}");
                        peers_clone.lock().unwrap().insert(
                            remote_id.clone(),
                            Peer { device_id: remote_id, addr, quic_port: port },
                        );
                    }
                }
                ServiceEvent::ServiceRemoved(_, fullname) => {
                    println!("[nilbus:mdns] Peer left: {fullname}");
                    let mut map = peers_clone.lock().unwrap();
                    map.retain(|id, _| !fullname.contains(id.as_str()));
                }
                _ => {}
            }
        }
    });

    Ok(())
}
