// android-host/src/network.rs — Android ConnectivityManager & WifiManager Bridge
// Adheres to Roadmap Section 28: Live telemetry source-of-truth from host OS,
// avoids misleading hard-coded IP/SSID/carrier data in production mode.

use crate::bridge::GuestToHostCommand;
use crate::jni_bridge;
use nilhal::traits::{ConnectionType, HalError, HalNetworkState, NetworkHal, WifiApInfo};

pub struct AndroidHostNetwork {
    state: HalNetworkState,
    is_simulated: bool,
}

impl AndroidHostNetwork {
    /// Default live network state: starts disconnected/unknown until live
    /// ConnectivityManager events arrive via JNI from the host Android OS.
    pub fn new() -> Self {
        Self {
            state: HalNetworkState {
                is_connected: false,
                active_interface: None,
                connection_type: ConnectionType::None,
                ip_address: None,
                dns_servers: Vec::new(),
                wifi_ssid: None,
                cellular_carrier: None,
            },
            is_simulated: false,
        }
    }

    /// Explicit simulated backend constructor for unit testing and offline development
    pub fn new_simulated() -> Self {
        Self {
            state: HalNetworkState {
                is_connected: true,
                active_interface: Some("wlan0".into()),
                connection_type: ConnectionType::Wifi,
                ip_address: Some("192.168.1.100".into()),
                dns_servers: vec!["8.8.8.8".into(), "1.1.1.1".into()],
                wifi_ssid: Some("Simulated-Galaxy-Network".into()),
                cellular_carrier: Some("Simulated 5G".into()),
            },
            is_simulated: true,
        }
    }

    pub fn is_simulated(&self) -> bool {
        self.is_simulated
    }

    /// Process live network telemetry dispatched by OnuronBridgeService ConnectivityManager callback
    pub fn update_from_host(
        &mut self,
        is_connected: bool,
        conn_type: ConnectionType,
        ip_address: Option<String>,
        ssid: Option<String>,
    ) {
        self.state.is_connected = is_connected;
        self.state.connection_type = conn_type;
        self.state.ip_address = ip_address;
        self.state.wifi_ssid = ssid;
        self.is_simulated = false;
    }
}

impl Default for AndroidHostNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkHal for AndroidHostNetwork {
    fn get_state(&self) -> HalNetworkState {
        self.state.clone()
    }

    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        if self.is_simulated {
            Ok(vec![WifiApInfo {
                ssid: "Simulated-Wi-Fi-6E".into(),
                bssid: "12:34:56:78:9A:BC".into(),
                signal_level: -48,
                security: "WPA3".into(),
            }])
        } else {
            // Live scan triggers host scan command and awaits ConnectivityManager results
            jni_bridge::enqueue_guest_command(GuestToHostCommand::ScanWifi);
            Ok(Vec::new())
        }
    }

    fn connect_wifi(&mut self, ssid: &str, psk: &str) -> Result<(), HalError> {
        let ssid_clean = ssid.trim();
        if ssid_clean.is_empty() {
            return Err(HalError::UnsupportedOperation("SSID cannot be empty".into()));
        }

        // Delegate to Android host WifiManager / Settings provisioning
        jni_bridge::enqueue_guest_command(GuestToHostCommand::ConnectWifi {
            ssid: ssid_clean.to_string(),
            password: psk.to_string(),
        });

        // Do NOT prematurely claim connected until host OS confirmation callback arrives
        Ok(())
    }

    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_network_live_vs_simulated() {
        let _guard = crate::jni_bridge::test_lock();
        let net_live = AndroidHostNetwork::new();
        assert!(!net_live.is_simulated());
        assert!(!net_live.get_state().is_connected);
        assert_eq!(net_live.get_state().connection_type, ConnectionType::None);

        let net_sim = AndroidHostNetwork::new_simulated();
        assert!(net_sim.is_simulated());
        assert!(net_sim.get_state().is_connected);
        assert_eq!(net_sim.get_state().connection_type, ConnectionType::Wifi);
    }

    #[test]
    fn test_network_update_from_host() {
        let _guard = crate::jni_bridge::test_lock();
        let mut net = AndroidHostNetwork::new();
        assert!(!net.get_state().is_connected);

        net.update_from_host(
            true,
            ConnectionType::Wifi,
            Some("10.0.0.42".into()),
            Some("MyHomeNetwork".into()),
        );

        let state = net.get_state();
        assert!(state.is_connected);
        assert_eq!(state.connection_type, ConnectionType::Wifi);
        assert_eq!(state.ip_address.as_deref(), Some("10.0.0.42"));
        assert_eq!(state.wifi_ssid.as_deref(), Some("MyHomeNetwork"));
    }

    #[test]
    fn test_connect_wifi_enqueues_command_without_premature_connected() {
        let _guard = crate::jni_bridge::test_lock();
        while jni_bridge::poll_guest_command().is_some() {}

        let mut net = AndroidHostNetwork::new();
        assert!(net.connect_wifi("Corp-Guest", "secret123").is_ok());

        // Still not connected until host event
        assert!(!net.get_state().is_connected);

        let cmd = jni_bridge::poll_guest_command().expect("Expected command");
        match cmd {
            GuestToHostCommand::ConnectWifi { ssid, password } => {
                assert_eq!(ssid, "Corp-Guest");
                assert_eq!(password, "secret123");
            }
            other => panic!("Expected ConnectWifi, got {:?}", other),
        }
    }
}
