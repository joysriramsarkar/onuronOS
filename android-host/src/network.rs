// android-host/src/network.rs — Android ConnectivityManager & WifiManager Bridge
use nilhal::traits::{ConnectionType, HalError, HalNetworkState, NetworkHal, WifiApInfo};

pub struct AndroidHostNetwork {
    state: HalNetworkState,
}

impl AndroidHostNetwork {
    pub fn new() -> Self {
        Self {
            state: HalNetworkState {
                is_connected: true,
                active_interface: Some("wlan0".into()),
                connection_type: ConnectionType::Wifi,
                ip_address: Some("192.168.1.100".into()),
                dns_servers: vec!["8.8.8.8".into(), "1.1.1.1".into()],
                wifi_ssid: Some("Galaxy-Network".into()),
                cellular_carrier: Some("Jio 5G".into()),
            },
        }
    }

    pub fn update_from_host(&mut self, is_connected: bool, conn_type: ConnectionType, ssid: Option<String>) {
        self.state.is_connected = is_connected;
        self.state.connection_type = conn_type;
        self.state.wifi_ssid = ssid;
    }
}

impl NetworkHal for AndroidHostNetwork {
    fn get_state(&self) -> HalNetworkState {
        self.state.clone()
    }

    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        Ok(vec![
            WifiApInfo {
                ssid: "Host-Wi-Fi-6E".into(),
                bssid: "12:34:56:78:9A:BC".into(),
                signal_level: -48,
                security: "WPA3".into(),
            }
        ])
    }

    fn connect_wifi(&mut self, ssid: &str, _psk: &str) -> Result<(), HalError> {
        self.state.wifi_ssid = Some(ssid.to_string());
        self.state.is_connected = true;
        Ok(())
    }

    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        Ok(())
    }
}
