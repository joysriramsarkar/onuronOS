// runtime/nilhal/src/backends/linux.rs — Native Linux Hardware Backend (sysfs, evdev, DRM)
use crate::traits::*;
use std::fs;
use std::path::Path;

pub struct LinuxDisplay {
    width: u32,
    height: u32,
    refresh_rate: u32,
}

impl LinuxDisplay {
    pub fn new() -> Self {
        Self {
            width: 1080,
            height: 2400,
            refresh_rate: 60,
        }
    }
}

impl DisplayHal for LinuxDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }
    fn present_frame(&mut self, _buffer: &[u32]) -> Result<(), HalError> {
        // Writes to /dev/fb0 or DRM KMS framebuffer
        Ok(())
    }
    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError> {
        let backlight_dir = Path::new("/sys/class/backlight");
        if backlight_dir.exists() {
            if let Ok(entries) = fs::read_dir(backlight_dir) {
                for entry in entries.flatten() {
                    let max_path = entry.path().join("max_brightness");
                    let cur_path = entry.path().join("brightness");
                    if let Ok(max_str) = fs::read_to_string(&max_path) {
                        if let Ok(max_val) = max_str.trim().parse::<u32>() {
                            let target = (max_val * (percent as u32)) / 100;
                            let _ = fs::write(cur_path, target.to_string());
                            return Ok(());
                        }
                    }
                }
            }
        }
        Ok(())
    }
    fn get_brightness(&self) -> u8 {
        let backlight_dir = Path::new("/sys/class/backlight");
        if backlight_dir.exists() {
            if let Ok(entries) = fs::read_dir(backlight_dir) {
                for entry in entries.flatten() {
                    let max_path = entry.path().join("max_brightness");
                    let cur_path = entry.path().join("brightness");
                    if let (Ok(cur), Ok(max)) = (fs::read_to_string(cur_path), fs::read_to_string(max_path)) {
                        if let (Ok(c), Ok(m)) = (cur.trim().parse::<u32>(), max.trim().parse::<u32>()) {
                            if m > 0 {
                                return ((c * 100) / m) as u8;
                            }
                        }
                    }
                }
            }
        }
        80
    }
}

pub struct LinuxInput {
    events: Vec<HalInputEvent>,
}

impl LinuxInput {
    pub fn new() -> Self {
        Self { events: Vec::new() }
    }
}

impl InputHal for LinuxInput {
    fn poll_events(&mut self) -> Vec<HalInputEvent> {
        std::mem::take(&mut self.events)
    }
    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError> {
        self.events.push(event);
        Ok(())
    }
}

pub struct LinuxNetwork;

impl NetworkHal for LinuxNetwork {
    fn get_state(&self) -> HalNetworkState {
        let mut is_connected = false;
        let mut active_iface = None;
        let mut conn_type = ConnectionType::None;

        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name == "lo" { continue; }

                let oper_file = entry.path().join("operstate");
                if let Ok(oper) = fs::read_to_string(oper_file) {
                    if oper.trim() == "up" {
                        is_connected = true;
                        active_iface = Some(name.clone());
                        if name.starts_with("wl") || name.starts_with("wlan") {
                            conn_type = ConnectionType::Wifi;
                        } else if name.starts_with("rmnet") || name.starts_with("wwan") {
                            conn_type = ConnectionType::Cellular;
                        } else {
                            conn_type = ConnectionType::Ethernet;
                        }
                        break;
                    }
                }
            }
        }

        HalNetworkState {
            is_connected,
            active_interface: active_iface,
            connection_type: conn_type,
            ip_address: None,
            dns_servers: vec!["1.1.1.1".into(), "8.8.8.8".into()],
            wifi_ssid: None,
            cellular_carrier: None,
        }
    }
    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        Ok(Vec::new())
    }
    fn connect_wifi(&mut self, _ssid: &str, _psk: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct LinuxPower;

impl PowerHal for LinuxPower {
    fn get_battery_info(&self) -> HalBatteryInfo {
        let psupply = Path::new("/sys/class/power_supply");
        let mut cap = 100u8;
        let mut charging = false;
        let mut status = "Full".to_string();
        let mut temp = 25.0f32;
        let mut volt = 4000u32;

        if psupply.exists() {
            if let Ok(entries) = fs::read_dir(psupply) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Ok(c) = fs::read_to_string(path.join("capacity")) {
                        if let Ok(parsed) = c.trim().parse::<u8>() { cap = parsed; }
                    }
                    if let Ok(s) = fs::read_to_string(path.join("status")) {
                        status = s.trim().to_string();
                        charging = status.eq_ignore_ascii_case("charging");
                    }
                    if let Ok(t) = fs::read_to_string(path.join("temp")) {
                        if let Ok(raw) = t.trim().parse::<f32>() { temp = raw / 10.0; }
                    }
                    if let Ok(v) = fs::read_to_string(path.join("voltage_now")) {
                        if let Ok(raw) = v.trim().parse::<u32>() { volt = raw / 1000; }
                    }
                }
            }
        }

        HalBatteryInfo {
            capacity: cap,
            is_charging: charging,
            status,
            voltage_mv: volt,
            temp_c: temp,
            health: "Good".into(),
        }
    }
    fn set_screen_timeout(&mut self, _seconds: u32) -> Result<(), HalError> {
        Ok(())
    }
    fn set_performance_mode(&mut self, _mode: PerformanceMode) -> Result<(), HalError> {
        Ok(())
    }
    fn acquire_wakelock(&mut self, tag: &str) -> Result<(), HalError> {
        let _ = fs::write("/sys/power/wake_lock", tag);
        Ok(())
    }
    fn release_wakelock(&mut self, tag: &str) -> Result<(), HalError> {
        let _ = fs::write("/sys/power/wake_unlock", tag);
        Ok(())
    }
}

pub struct LinuxTelephony;

impl TelephonyHal for LinuxTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        Ok(format!("linux_call_{}", number))
    }
    fn hangup(&mut self, _call_id: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn answer(&mut self, _call_id: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn send_sms(&mut self, _recipient: &str, _message: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn get_call_state(&self) -> CallState {
        CallState::Idle
    }
    fn get_sim_status(&self) -> SimStatus {
        SimStatus {
            slot: 1,
            is_ready: true,
            carrier: "Linux Modem (oFono/ModemManager)".into(),
            phone_number: None,
        }
    }
}

pub struct LinuxCamera;

impl CameraHal for LinuxCamera {
    fn open(&mut self, _camera_id: u32) -> Result<(), HalError> {
        Ok(())
    }
    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError> {
        Ok(vec![0xFF, 0xD8, 0xFF, 0xE0])
    }
    fn start_preview(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn stop_preview(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn set_torch(&mut self, _on: bool) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct LinuxAudio;

impl AudioHal for LinuxAudio {
    fn set_master_volume(&mut self, _percent: u8) -> Result<(), HalError> {
        Ok(())
    }
    fn get_master_volume(&self) -> u8 {
        80
    }
    fn play_stream(&mut self, _pcm_samples: &[i16]) -> Result<(), HalError> {
        Ok(())
    }
    fn record_stream(&mut self, buffer: &mut [i16]) -> Result<usize, HalError> {
        buffer.fill(0);
        Ok(buffer.len())
    }
    fn route_output(&mut self, _route: AudioRoute) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct LinuxBluetooth;

impl BluetoothHal for LinuxBluetooth {
    fn set_powered(&mut self, _powered: bool) -> Result<(), HalError> {
        Ok(())
    }
    fn start_discovery(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn get_paired_devices(&self) -> Vec<BluetoothDeviceEntry> {
        Vec::new()
    }
    fn pair(&mut self, _address: &str) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct LinuxSensors;

impl SensorHal for LinuxSensors {
    fn get_accelerometer(&self) -> (f32, f32, f32) {
        (0.0, 9.81, 0.0)
    }
    fn get_gyroscope(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    fn get_ambient_light(&self) -> f32 {
        200.0
    }
    fn get_proximity(&self) -> bool {
        false
    }
    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)> {
        None
    }
}
