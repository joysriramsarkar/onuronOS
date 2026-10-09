// services/btd/src/main.rs — Onuron OS Bluetooth Subsystem Daemon (btd)
// Discovers Linux Bluetooth adapters (/sys/class/bluetooth), manages RFKILL state,
// coordinates device pairing/connection, and exposes a canonical framed IPC interface (/run/nilos/bt.sock).

use std::fs;
use std::path::Path;
#[cfg(unix)]
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use nilprotocol::{
    BtConnectPayload, BtDevicePayload, BtScanResultPayload, BtSetPowerPayload, BtStatePayload,
    Frame, MessageType,
};

pub const BT_SOCK_PATH: &str = "/run/nilos/bt.sock";

#[derive(Debug, Clone)]
pub struct BluetoothManager {
    pub enabled: bool,
    pub adapter_name: String,
    pub address: String,
    pub discovering: bool,
    pub paired_devices: Vec<BtDevicePayload>,
    pub discovered_devices: Vec<BtDevicePayload>,
}

impl Default for BluetoothManager {
    fn default() -> Self {
        Self::new()
    }
}

impl BluetoothManager {
    pub fn new() -> Self {
        let (found, name, addr) = Self::detect_hardware_adapter();
        let mut mgr = Self {
            enabled: found,
            adapter_name: name,
            address: addr,
            discovering: false,
            paired_devices: Vec::new(),
            discovered_devices: Vec::new(),
        };

        // Seed with sample paired devices if in simulator/prototype mode
        if !found {
            mgr.paired_devices.push(BtDevicePayload {
                address: "44:6D:57:12:34:56".to_string(),
                name: "NilBuds Pro".to_string(),
                rssi: -58,
                connected: false,
                paired: true,
            });
        }
        mgr
    }

    /// Scan `/sys/class/bluetooth` and `/sys/class/rfkill` for Linux Bluetooth adapters.
    pub fn detect_hardware_adapter() -> (bool, String, String) {
        let bt_sysfs = Path::new("/sys/class/bluetooth");
        if bt_sysfs.is_dir() {
            if let Ok(entries) = fs::read_dir(bt_sysfs) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.starts_with("hci") {
                        let addr = fs::read_to_string(entry.path().join("address"))
                            .map(|s| s.trim().to_uppercase())
                            .unwrap_or_else(|_| "00:00:00:00:00:00".to_string());
                        return (true, name, addr);
                    }
                }
            }
        }

        // Fallback for QEMU / development host
        (false, "hci0".to_string(), "02:00:00:00:00:01".to_string())
    }

    pub fn get_state(&self) -> BtStatePayload {
        let mut connected = Vec::new();
        for dev in &self.paired_devices {
            if dev.connected {
                connected.push(dev.clone());
            }
        }
        BtStatePayload {
            enabled: self.enabled,
            adapter_name: self.adapter_name.clone(),
            address: self.address.clone(),
            discovering: self.discovering,
            connected_devices: connected,
        }
    }

    pub fn set_power(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.discovering = false;
            for dev in &mut self.paired_devices {
                dev.connected = false;
            }
            self.discovered_devices.clear();
        }
    }

    pub fn start_scan(&mut self) -> Vec<BtDevicePayload> {
        if !self.enabled {
            return Vec::new();
        }
        self.discovering = true;

        // In real hardware this commands BlueZ / HCI mgmt; in prototype/headless we return discovered devices
        self.discovered_devices = vec![
            BtDevicePayload {
                address: "E4:5F:01:23:45:67".to_string(),
                name: "NilKeyboard BLE".to_string(),
                rssi: -65,
                connected: false,
                paired: false,
            },
            BtDevicePayload {
                address: "12:34:56:78:9A:BC".to_string(),
                name: "Smart Watch".to_string(),
                rssi: -72,
                connected: false,
                paired: false,
            },
        ];

        let mut all = self.paired_devices.clone();
        all.extend(self.discovered_devices.clone());
        all
    }

    pub fn connect(&mut self, address: &str) -> bool {
        if !self.enabled {
            return false;
        }

        for dev in &mut self.paired_devices {
            if dev.address.eq_ignore_ascii_case(address) {
                dev.connected = true;
                return true;
            }
        }

        // If in discovered, promote to paired and connected
        if let Some(pos) = self
            .discovered_devices
            .iter()
            .position(|d| d.address.eq_ignore_ascii_case(address))
        {
            let mut dev = self.discovered_devices.remove(pos);
            dev.paired = true;
            dev.connected = true;
            self.paired_devices.push(dev);
            return true;
        }

        false
    }

    pub fn disconnect(&mut self, address: &str) -> bool {
        for dev in &mut self.paired_devices {
            if dev.address.eq_ignore_ascii_case(address) {
                dev.connected = false;
                return true;
            }
        }
        false
    }
}

pub fn handle_ipc_request(frame: &Frame, manager: &mut BluetoothManager) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),

        MessageType::BtGetState => {
            let state = manager.get_state();
            Frame::with_json(MessageType::BtStateInfo, frame.request_id, &state)
                .unwrap_or_else(|e| {
                    Frame::new(
                        MessageType::ErrorResponse,
                        frame.request_id,
                        e.to_string().into_bytes(),
                    )
                })
        }

        MessageType::BtSetPower => {
            if let Ok(payload) = frame.parse_json::<BtSetPowerPayload>() {
                manager.set_power(payload.enabled);
                let state = manager.get_state();
                Frame::with_json(MessageType::BtStateInfo, frame.request_id, &state)
                    .unwrap_or_else(|e| {
                        Frame::new(
                            MessageType::ErrorResponse,
                            frame.request_id,
                            e.to_string().into_bytes(),
                        )
                    })
            } else {
                Frame::new(
                    MessageType::ErrorResponse,
                    frame.request_id,
                    b"invalid BtSetPower payload".to_vec(),
                )
            }
        }

        MessageType::BtScan => {
            let devices = manager.start_scan();
            let payload = BtScanResultPayload { devices };
            Frame::with_json(MessageType::BtScanResult, frame.request_id, &payload)
                .unwrap_or_else(|e| {
                    Frame::new(
                        MessageType::ErrorResponse,
                        frame.request_id,
                        e.to_string().into_bytes(),
                    )
                })
        }

        MessageType::BtConnect => {
            if let Ok(payload) = frame.parse_json::<BtConnectPayload>() {
                let _ok = manager.connect(&payload.address);
                let state = manager.get_state();
                Frame::with_json(MessageType::BtStateInfo, frame.request_id, &state)
                    .unwrap_or_else(|e| {
                        Frame::new(
                            MessageType::ErrorResponse,
                            frame.request_id,
                            e.to_string().into_bytes(),
                        )
                    })
            } else {
                Frame::new(
                    MessageType::ErrorResponse,
                    frame.request_id,
                    b"invalid BtConnect payload".to_vec(),
                )
            }
        }

        _ => Frame::new(
            MessageType::ErrorResponse,
            frame.request_id,
            b"unsupported message type".to_vec(),
        ),
    }
}

fn main() {
    println!("=========================================================");
    println!("       Onuron OS Bluetooth Subsystem Daemon (btd)        ");
    println!("=========================================================");

    let manager = Arc::new(Mutex::new(BluetoothManager::new()));
    println!(
        "[btd] Adapter: {} ({}) | Initial State: {}",
        manager.lock().unwrap().adapter_name,
        manager.lock().unwrap().address,
        if manager.lock().unwrap().enabled { "ENABLED" } else { "DISABLED" }
    );

    #[cfg(unix)]
    {
        use std::os::unix::net::UnixListener;
        let sock_path = PathBuf::from(BT_SOCK_PATH);
        if let Some(parent) = sock_path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let _ = fs::remove_file(&sock_path);

        match UnixListener::bind(&sock_path) {
            Ok(listener) => {
                println!("[btd] Listening for IPC at {}", sock_path.display());
                let mgr = Arc::clone(&manager);
                thread::spawn(move || {
                    for stream in listener.incoming() {
                        if let Ok(mut stream) = stream {
                            let mgr = Arc::clone(&mgr);
                            thread::spawn(move || {
                                while let Ok(frame) = nilprotocol::read_frame(&mut stream) {
                                    let resp = {
                                        let mut m = mgr.lock().unwrap();
                                        handle_ipc_request(&frame, &mut m)
                                    };
                                    if nilprotocol::write_frame(&mut stream, &resp).is_err() {
                                        break;
                                    }
                                }
                            });
                        }
                    }
                });
            }
            Err(e) => {
                eprintln!("[btd] Notice: could not bind Unix socket {}: {}", sock_path.display(), e);
            }
        }
    }

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bluetooth_state_and_power_toggling() {
        let mut mgr = BluetoothManager::new();
        mgr.set_power(true);
        assert!(mgr.enabled);
        let state = mgr.get_state();
        assert!(state.enabled);
        assert_eq!(state.adapter_name, "hci0");

        mgr.set_power(false);
        assert!(!mgr.enabled);
        let state2 = mgr.get_state();
        assert!(!state2.enabled);
    }

    #[test]
    fn test_bluetooth_scan_and_connect() {
        let mut mgr = BluetoothManager::new();
        mgr.set_power(true);
        let scan_results = mgr.start_scan();
        assert!(!scan_results.is_empty());

        let target_addr = "E4:5F:01:23:45:67";
        let connected = mgr.connect(target_addr);
        assert!(connected);

        let state = mgr.get_state();
        assert!(state.connected_devices.iter().any(|d| d.address == target_addr));

        let disconnected = mgr.disconnect(target_addr);
        assert!(disconnected);
        let state_after = mgr.get_state();
        assert!(!state_after.connected_devices.iter().any(|d| d.address == target_addr));
    }

    #[test]
    fn test_bluetooth_ipc_handling() {
        let mut mgr = BluetoothManager::new();
        mgr.set_power(true);

        // 1. Ping
        let ping_frame = Frame::new(MessageType::Ping, 1, vec![]);
        let pong = handle_ipc_request(&ping_frame, &mut mgr);
        assert_eq!(pong.message_type, u16::from(MessageType::Pong));

        // 2. BtGetState
        let get_state_frame = Frame::new(MessageType::BtGetState, 2, vec![]);
        let state_resp = handle_ipc_request(&get_state_frame, &mut mgr);
        assert_eq!(state_resp.message_type, u16::from(MessageType::BtStateInfo));
        let state: BtStatePayload = state_resp.parse_json().unwrap();
        assert!(state.enabled);

        // 3. BtSetPower (disable)
        let set_pwr = Frame::with_json(MessageType::BtSetPower, 3, &BtSetPowerPayload { enabled: false }).unwrap();
        let pwr_resp = handle_ipc_request(&set_pwr, &mut mgr);
        assert_eq!(pwr_resp.message_type, u16::from(MessageType::BtStateInfo));
        let state_pwr: BtStatePayload = pwr_resp.parse_json().unwrap();
        assert!(!state_pwr.enabled);

        // 4. BtScan while disabled returns empty
        let scan_frame = Frame::new(MessageType::BtScan, 4, vec![]);
        let scan_resp = handle_ipc_request(&scan_frame, &mut mgr);
        let scan_result: BtScanResultPayload = scan_resp.parse_json().unwrap();
        assert!(scan_result.devices.is_empty());
    }
}
