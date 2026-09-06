// runtime/nilhal/src/backends/qemu.rs — QEMU / Simulated Hardware Backend
use crate::traits::*;

pub struct QemuDisplay {
    width: u32,
    height: u32,
    refresh_rate: u32,
    brightness: u8,
}

impl QemuDisplay {
    pub fn new() -> Self {
        Self {
            width: 1080,
            height: 2340,
            refresh_rate: 60,
            brightness: 80,
        }
    }
}

impl DisplayHal for QemuDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }
    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }
    fn present_frame(&mut self, _buffer: &[u32]) -> Result<(), HalError> {
        Ok(())
    }
    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError> {
        self.brightness = percent.min(100);
        Ok(())
    }
    fn get_brightness(&self) -> u8 {
        self.brightness
    }
}

pub struct QemuInput {
    queue: Vec<HalInputEvent>,
}

impl QemuInput {
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }
}

impl InputHal for QemuInput {
    fn poll_events(&mut self) -> Vec<HalInputEvent> {
        std::mem::take(&mut self.queue)
    }
    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError> {
        self.queue.push(event);
        Ok(())
    }
}

pub struct QemuNetwork;

impl NetworkHal for QemuNetwork {
    fn get_state(&self) -> HalNetworkState {
        HalNetworkState {
            is_connected: true,
            active_interface: Some("virtio-net0".into()),
            connection_type: ConnectionType::Ethernet,
            ip_address: Some("10.0.2.15".into()),
            dns_servers: vec!["10.0.2.3".into(), "1.1.1.1".into()],
            wifi_ssid: None,
            cellular_carrier: None,
        }
    }
    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        Ok(vec![
            WifiApInfo {
                ssid: "QEMU-Virtual-WiFi".into(),
                bssid: "52:54:00:12:34:56".into(),
                signal_level: -45,
                security: "WPA2-PSK".into(),
            }
        ])
    }
    fn connect_wifi(&mut self, _ssid: &str, _psk: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct QemuPower {
    battery: HalBatteryInfo,
    perf_mode: PerformanceMode,
}

impl QemuPower {
    pub fn new() -> Self {
        Self {
            battery: HalBatteryInfo {
                capacity: 92,
                is_charging: false,
                status: "Discharging".into(),
                voltage_mv: 4120,
                temp_c: 28.5,
                health: "Good".into(),
            },
            perf_mode: PerformanceMode::Balanced,
        }
    }
}

impl PowerHal for QemuPower {
    fn get_battery_info(&self) -> HalBatteryInfo {
        self.battery.clone()
    }
    fn set_screen_timeout(&mut self, _seconds: u32) -> Result<(), HalError> {
        Ok(())
    }
    fn set_performance_mode(&mut self, mode: PerformanceMode) -> Result<(), HalError> {
        self.perf_mode = mode;
        Ok(())
    }
    fn acquire_wakelock(&mut self, _tag: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn release_wakelock(&mut self, _tag: &str) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct QemuTelephony {
    call_state: CallState,
}

impl QemuTelephony {
    pub fn new() -> Self {
        Self {
            call_state: CallState::Idle,
        }
    }
}

impl TelephonyHal for QemuTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        let call_id = format!("call_qemu_{}", number);
        self.call_state = CallState::Active {
            number: number.to_string(),
            duration_secs: 0,
        };
        Ok(call_id)
    }
    fn hangup(&mut self, _call_id: &str) -> Result<(), HalError> {
        self.call_state = CallState::Idle;
        Ok(())
    }
    fn answer(&mut self, _call_id: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn send_sms(&mut self, _recipient: &str, _message: &str) -> Result<(), HalError> {
        Ok(())
    }
    fn get_call_state(&self) -> CallState {
        self.call_state.clone()
    }
    fn get_sim_status(&self) -> SimStatus {
        SimStatus {
            slot: 1,
            is_ready: true,
            carrier: "QEMU-Virtual-SIM".into(),
            phone_number: Some("+15550001".into()),
        }
    }
}

pub struct QemuCamera {
    torch_on: bool,
}

impl QemuCamera {
    pub fn new() -> Self {
        Self { torch_on: false }
    }
}

impl CameraHal for QemuCamera {
    fn open(&mut self, _camera_id: u32) -> Result<(), HalError> {
        Ok(())
    }
    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError> {
        // Return synthetic 64x64 checkerboard JPEG or test bytes
        Ok(vec![0xFF, 0xD8, 0xFF, 0xE0, 0x00, 0x10, 0x4A, 0x46, 0x49, 0x46])
    }
    fn start_preview(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn stop_preview(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn set_torch(&mut self, on: bool) -> Result<(), HalError> {
        self.torch_on = on;
        Ok(())
    }
}

pub struct QemuAudio {
    volume: u8,
}

impl QemuAudio {
    pub fn new() -> Self {
        Self { volume: 75 }
    }
}

impl AudioHal for QemuAudio {
    fn set_master_volume(&mut self, percent: u8) -> Result<(), HalError> {
        self.volume = percent.min(100);
        Ok(())
    }
    fn get_master_volume(&self) -> u8 {
        self.volume
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

pub struct QemuBluetooth;

impl BluetoothHal for QemuBluetooth {
    fn set_powered(&mut self, _powered: bool) -> Result<(), HalError> {
        Ok(())
    }
    fn start_discovery(&mut self) -> Result<(), HalError> {
        Ok(())
    }
    fn get_paired_devices(&self) -> Vec<BluetoothDeviceEntry> {
        vec![
            BluetoothDeviceEntry {
                name: "QEMU Virtual Buds".into(),
                address: "AA:BB:CC:DD:EE:01".into(),
                is_connected: true,
            }
        ]
    }
    fn pair(&mut self, _address: &str) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct QemuSensors;

impl SensorHal for QemuSensors {
    fn get_accelerometer(&self) -> (f32, f32, f32) {
        (0.0, 9.81, 0.0) // 1G standard gravity
    }
    fn get_gyroscope(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }
    fn get_ambient_light(&self) -> f32 {
        350.0 // Normal indoor lighting
    }
    fn get_proximity(&self) -> bool {
        false // Far
    }
    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)> {
        Some((22.5726, 88.3639, 11.0)) // Kolkata coordinates as default lab test location
    }
}
