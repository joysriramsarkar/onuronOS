// runtime/nilhal/src/backends/fake.rs — In-Memory Mock Hardware Backend for Headless CI and Testing
use crate::traits::*;
use std::sync::{Arc, Mutex};

/// In-memory mock display that records presented frames for test assertions.
#[derive(Debug, Clone)]
pub struct FakeDisplay {
    pub width: u32,
    pub height: u32,
    pub refresh_rate: u32,
    pub brightness: u8,
    pub frame_count: Arc<Mutex<usize>>,
    pub last_frame: Arc<Mutex<Option<Vec<u32>>>>,
}

impl FakeDisplay {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            refresh_rate: 60,
            brightness: 100,
            frame_count: Arc::new(Mutex::new(0)),
            last_frame: Arc::new(Mutex::new(None)),
        }
    }
}

impl Default for FakeDisplay {
    fn default() -> Self {
        Self::new(1080, 2400)
    }
}

impl DisplayHal for FakeDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }

    fn present_frame(&mut self, buffer: &[u32]) -> Result<(), HalError> {
        let mut count = self.frame_count.lock().unwrap();
        *count += 1;
        let mut last = self.last_frame.lock().unwrap();
        *last = Some(buffer.to_vec());
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

/// In-memory mock input device allowing test code to inject and poll events.
#[derive(Debug, Clone, Default)]
pub struct FakeInput {
    pub queue: Arc<Mutex<Vec<HalInputEvent>>>,
}

impl FakeInput {
    pub fn new() -> Self {
        Self {
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn inject(&self, event: HalInputEvent) {
        self.queue.lock().unwrap().push(event);
    }
}

impl InputHal for FakeInput {
    fn poll_events(&mut self) -> Vec<HalInputEvent> {
        let mut q = self.queue.lock().unwrap();
        std::mem::take(&mut *q)
    }

    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError> {
        self.queue.lock().unwrap().push(event);
        Ok(())
    }
}

/// In-memory mock network controller.
#[derive(Debug, Clone)]
pub struct FakeNetwork {
    pub state: Arc<Mutex<HalNetworkState>>,
    pub wifi_aps: Arc<Mutex<Vec<WifiApInfo>>>,
}

impl FakeNetwork {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(HalNetworkState {
                is_connected: true,
                active_interface: Some("fake0".to_string()),
                connection_type: ConnectionType::Wifi,
                ip_address: Some("192.168.1.100".to_string()),
                dns_servers: vec!["1.1.1.1".to_string()],
                wifi_ssid: Some("MockWiFi".to_string()),
                cellular_carrier: None,
            })),
            wifi_aps: Arc::new(Mutex::new(vec![WifiApInfo {
                ssid: "MockWiFi".to_string(),
                bssid: "00:11:22:33:44:55".to_string(),
                signal_level: -50,
                security: "WPA2".to_string(),
            }])),
        }
    }
}

impl Default for FakeNetwork {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkHal for FakeNetwork {
    fn get_state(&self) -> HalNetworkState {
        self.state.lock().unwrap().clone()
    }

    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        Ok(self.wifi_aps.lock().unwrap().clone())
    }

    fn connect_wifi(&mut self, ssid: &str, _psk: &str) -> Result<(), HalError> {
        let mut st = self.state.lock().unwrap();
        st.is_connected = true;
        st.wifi_ssid = Some(ssid.to_string());
        st.connection_type = ConnectionType::Wifi;
        Ok(())
    }

    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        Ok(())
    }
}

/// In-memory mock power manager.
#[derive(Debug, Clone)]
pub struct FakePower {
    pub battery: Arc<Mutex<HalBatteryInfo>>,
    pub wakelocks: Arc<Mutex<Vec<String>>>,
    pub perf_mode: Arc<Mutex<PerformanceMode>>,
}

impl FakePower {
    pub fn new() -> Self {
        Self {
            battery: Arc::new(Mutex::new(HalBatteryInfo {
                capacity: 85,
                is_charging: false,
                status: "Discharging".to_string(),
                voltage_mv: 4100,
                temp_c: 29.5,
                health: "Good".to_string(),
            })),
            wakelocks: Arc::new(Mutex::new(Vec::new())),
            perf_mode: Arc::new(Mutex::new(PerformanceMode::Balanced)),
        }
    }
}

impl Default for FakePower {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerHal for FakePower {
    fn get_battery_info(&self) -> HalBatteryInfo {
        self.battery.lock().unwrap().clone()
    }

    fn set_screen_timeout(&mut self, _seconds: u32) -> Result<(), HalError> {
        Ok(())
    }

    fn set_performance_mode(&mut self, mode: PerformanceMode) -> Result<(), HalError> {
        *self.perf_mode.lock().unwrap() = mode;
        Ok(())
    }

    fn acquire_wakelock(&mut self, tag: &str) -> Result<(), HalError> {
        self.wakelocks.lock().unwrap().push(tag.to_string());
        Ok(())
    }

    fn release_wakelock(&mut self, tag: &str) -> Result<(), HalError> {
        let mut locks = self.wakelocks.lock().unwrap();
        locks.retain(|l| l != tag);
        Ok(())
    }
}

/// In-memory mock telephony.
#[derive(Debug, Clone)]
pub struct FakeTelephony {
    pub call_state: Arc<Mutex<CallState>>,
    pub sim_status: Arc<Mutex<SimStatus>>,
    pub sent_sms: Arc<Mutex<Vec<(String, String)>>>,
}

impl FakeTelephony {
    pub fn new() -> Self {
        Self {
            call_state: Arc::new(Mutex::new(CallState::Idle)),
            sim_status: Arc::new(Mutex::new(SimStatus {
                slot: 1,
                is_ready: true,
                carrier: "Mock Carrier".to_string(),
                phone_number: Some("+1234567890".to_string()),
            })),
            sent_sms: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl Default for FakeTelephony {
    fn default() -> Self {
        Self::new()
    }
}

impl TelephonyHal for FakeTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        let call_id = "fake-call-1".to_string();
        *self.call_state.lock().unwrap() = CallState::Active {
            number: number.to_string(),
            duration_secs: 0,
        };
        Ok(call_id)
    }

    fn hangup(&mut self, _call_id: &str) -> Result<(), HalError> {
        *self.call_state.lock().unwrap() = CallState::Idle;
        Ok(())
    }

    fn answer(&mut self, _call_id: &str) -> Result<(), HalError> {
        Ok(())
    }

    fn send_sms(&mut self, recipient: &str, message: &str) -> Result<(), HalError> {
        self.sent_sms.lock().unwrap().push((recipient.to_string(), message.to_string()));
        Ok(())
    }

    fn get_call_state(&self) -> CallState {
        self.call_state.lock().unwrap().clone()
    }

    fn get_sim_status(&self) -> SimStatus {
        self.sim_status.lock().unwrap().clone()
    }
}

/// In-memory mock camera.
#[derive(Debug, Clone, Default)]
pub struct FakeCamera {
    pub is_open: Arc<Mutex<bool>>,
    pub is_previewing: Arc<Mutex<bool>>,
    pub torch_on: Arc<Mutex<bool>>,
}

impl CameraHal for FakeCamera {
    fn open(&mut self, _camera_id: u32) -> Result<(), HalError> {
        *self.is_open.lock().unwrap() = true;
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError> {
        Ok(vec![0u8; 1024])
    }

    fn start_preview(&mut self) -> Result<(), HalError> {
        *self.is_previewing.lock().unwrap() = true;
        Ok(())
    }

    fn stop_preview(&mut self) -> Result<(), HalError> {
        *self.is_previewing.lock().unwrap() = false;
        Ok(())
    }

    fn set_torch(&mut self, on: bool) -> Result<(), HalError> {
        *self.torch_on.lock().unwrap() = on;
        Ok(())
    }
}

/// In-memory mock audio.
#[derive(Debug, Clone)]
pub struct FakeAudio {
    pub volume: Arc<Mutex<u8>>,
    pub played_samples_count: Arc<Mutex<usize>>,
    pub route: Arc<Mutex<AudioRoute>>,
}

impl FakeAudio {
    pub fn new() -> Self {
        Self {
            volume: Arc::new(Mutex::new(80)),
            played_samples_count: Arc::new(Mutex::new(0)),
            route: Arc::new(Mutex::new(AudioRoute::Speaker)),
        }
    }
}

impl Default for FakeAudio {
    fn default() -> Self {
        Self::new()
    }
}

impl AudioHal for FakeAudio {
    fn set_master_volume(&mut self, percent: u8) -> Result<(), HalError> {
        *self.volume.lock().unwrap() = percent.min(100);
        Ok(())
    }

    fn get_master_volume(&self) -> u8 {
        *self.volume.lock().unwrap()
    }

    fn play_stream(&mut self, pcm_samples: &[i16]) -> Result<(), HalError> {
        let mut count = self.played_samples_count.lock().unwrap();
        *count += pcm_samples.len();
        Ok(())
    }

    fn record_stream(&mut self, buffer: &mut [i16]) -> Result<usize, HalError> {
        buffer.fill(0);
        Ok(buffer.len())
    }

    fn route_output(&mut self, route: AudioRoute) -> Result<(), HalError> {
        *self.route.lock().unwrap() = route;
        Ok(())
    }
}

/// In-memory mock bluetooth.
#[derive(Debug, Clone, Default)]
pub struct FakeBluetooth {
    pub powered: Arc<Mutex<bool>>,
    pub paired: Arc<Mutex<Vec<BluetoothDeviceEntry>>>,
}

impl BluetoothHal for FakeBluetooth {
    fn set_powered(&mut self, powered: bool) -> Result<(), HalError> {
        *self.powered.lock().unwrap() = powered;
        Ok(())
    }

    fn start_discovery(&mut self) -> Result<(), HalError> {
        Ok(())
    }

    fn get_paired_devices(&self) -> Vec<BluetoothDeviceEntry> {
        self.paired.lock().unwrap().clone()
    }

    fn pair(&mut self, address: &str) -> Result<(), HalError> {
        self.paired.lock().unwrap().push(BluetoothDeviceEntry {
            name: "Mock Device".to_string(),
            address: address.to_string(),
            is_connected: true,
        });
        Ok(())
    }
}

/// In-memory mock sensors.
#[derive(Debug, Clone, Default)]
pub struct FakeSensors;

impl SensorHal for FakeSensors {
    fn get_accelerometer(&self) -> (f32, f32, f32) {
        (0.0, 9.81, 0.0)
    }

    fn get_gyroscope(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }

    fn get_ambient_light(&self) -> f32 {
        350.0
    }

    fn get_proximity(&self) -> bool {
        false
    }

    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)> {
        Some((23.8103, 90.4125, 10.0))
    }
}
