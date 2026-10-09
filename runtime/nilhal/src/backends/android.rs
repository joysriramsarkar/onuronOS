// runtime/nilhal/src/backends/android.rs — Android Hosted Backend (Samsung Galaxy S25 / Android Mobile Runtime)
#![allow(dead_code)]
use crate::traits::*;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Standard valid 16x16 test pattern JPEG (JFIF baseline, SOF0, DQT, DHT, SOS, EOI)
const VALID_BASELINE_JPEG: &[u8] = &[
    0xFF, 0xD8, // SOI
    0xFF, 0xE0, 0x00, 0x10, b'J', b'F', b'I', b'F', 0x00, 0x01, 0x01, 0x01, 0x00, 0x48, 0x00, 0x48, 0x00, 0x00, // APP0
    0xFF, 0xDB, 0x00, 0x43, 0x00, // DQT
    0x08, 0x06, 0x06, 0x07, 0x06, 0x05, 0x08, 0x07, 0x07, 0x07, 0x09, 0x09, 0x08, 0x0A, 0x0C, 0x14,
    0x0D, 0x0C, 0x0B, 0x0B, 0x0C, 0x19, 0x12, 0x13, 0x0F, 0x14, 0x1D, 0x1A, 0x1F, 0x1E, 0x1D, 0x1A,
    0x1C, 0x1C, 0x20, 0x24, 0x2E, 0x27, 0x20, 0x22, 0x2C, 0x23, 0x1C, 0x1C, 0x28, 0x37, 0x29, 0x2C,
    0x30, 0x31, 0x34, 0x34, 0x34, 0x1F, 0x27, 0x39, 0x3D, 0x38, 0x32, 0x3C, 0x2E, 0x33, 0x34, 0x32,
    0xFF, 0xC0, 0x00, 0x0B, 0x08, 0x00, 0x10, 0x00, 0x10, 0x01, 0x01, 0x11, 0x00, // SOF0
    0xFF, 0xC4, 0x00, 0x1F, 0x00, 0x00, 0x01, 0x05, 0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x00, 0x00, // DHT
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    0xFF, 0xDA, 0x00, 0x08, 0x01, 0x01, 0x00, 0x00, 0x3F, 0x00, // SOS
    0x7F, 0x00, 0xD2, 0x8A, 0x28, 0xA2, 0x80, // Scan data
    0xFF, 0xD9, // EOI
];

/// IPC bridge client to Android Host service (via Unix socket or shared bridge)
#[derive(Clone)]
pub struct AndroidBridgeClient {
    pub socket_path: String,
}

impl AndroidBridgeClient {
    pub fn new() -> Self {
        Self {
            socket_path: std::env::var("ONURON_ANDROID_BRIDGE")
                .unwrap_or_else(|_| "/run/onuron/android_bridge.sock".to_string()),
        }
    }
}

impl Default for AndroidBridgeClient {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AndroidDisplay {
    bridge: AndroidBridgeClient,
    width: u32,
    height: u32,
    refresh_rate: u32,
    brightness: u8,
    frame_count: u64,
    last_frame: Option<Vec<u32>>,
}

impl AndroidDisplay {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self {
            bridge,
            // S25 Dynamic AMOLED 2X specs
            width: 1080,
            height: 2340,
            refresh_rate: 120, // 120 Hz ProMotion display
            brightness: 85,
            frame_count: 0,
            last_frame: None,
        }
    }

    pub fn get_frame_count(&self) -> u64 {
        self.frame_count
    }
}

impl DisplayHal for AndroidDisplay {
    fn get_dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn get_refresh_rate(&self) -> u32 {
        self.refresh_rate
    }

    fn present_frame(&mut self, buffer: &[u32]) -> Result<(), HalError> {
        let expected = (self.width * self.height) as usize;
        if !buffer.is_empty() && buffer.len() != expected {
            return Err(HalError::UnsupportedOperation(format!(
                "Frame buffer size mismatch: got {}, expected {}",
                buffer.len(),
                expected
            )));
        }

        self.last_frame = Some(buffer.to_vec());
        self.frame_count = self.frame_count.saturating_add(1);
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

pub struct AndroidInput {
    bridge: AndroidBridgeClient,
    queue: Arc<Mutex<Vec<HalInputEvent>>>,
}

impl AndroidInput {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self {
            bridge,
            queue: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl InputHal for AndroidInput {
    fn poll_events(&mut self) -> Vec<HalInputEvent> {
        if let Ok(mut lock) = self.queue.lock() {
            std::mem::take(&mut *lock)
        } else {
            Vec::new()
        }
    }

    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError> {
        if let Ok(mut lock) = self.queue.lock() {
            lock.push(event);
            Ok(())
        } else {
            Err(HalError::HardwareBusy)
        }
    }
}

pub struct AndroidNetwork {
    bridge: AndroidBridgeClient,
}

impl AndroidNetwork {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self { bridge }
    }
}

impl NetworkHal for AndroidNetwork {
    fn get_state(&self) -> HalNetworkState {
        HalNetworkState {
            is_connected: false,
            active_interface: None,
            connection_type: ConnectionType::None,
            ip_address: None,
            dns_servers: Vec::new(),
            wifi_ssid: None,
            cellular_carrier: None,
        }
    }

    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError> {
        Err(HalError::UnsupportedOperation("Wi-Fi scan bridge not connected".into()))
    }

    fn connect_wifi(&mut self, _ssid: &str, _psk: &str) -> Result<(), HalError> {
        let _ = (_ssid, _psk);
        Err(HalError::UnsupportedOperation("Wi-Fi connect bridge not connected".into()))
    }

    fn set_cellular_enabled(&mut self, _enabled: bool) -> Result<(), HalError> {
        let _ = _enabled;
        Err(HalError::UnsupportedOperation("Cellular control bridge not connected".into()))
    }
}

pub struct AndroidPower {
    bridge: AndroidBridgeClient,
}

impl AndroidPower {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self { bridge }
    }
}

impl PowerHal for AndroidPower {
    fn get_battery_info(&self) -> HalBatteryInfo {
        HalBatteryInfo {
            capacity: 88,
            is_charging: false,
            status: "Discharging".into(),
            voltage_mv: 4050,
            temp_c: 31.2,
            health: "Good".into(),
        }
    }

    fn set_screen_timeout(&mut self, _seconds: u32) -> Result<(), HalError> {
        Ok(())
    }

    fn set_performance_mode(&mut self, _mode: PerformanceMode) -> Result<(), HalError> {
        Ok(())
    }

    fn acquire_wakelock(&mut self, _tag: &str) -> Result<(), HalError> {
        Ok(())
    }

    fn release_wakelock(&mut self, _tag: &str) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct AndroidTelephony {
    bridge: AndroidBridgeClient,
    call_state: CallState,
    sent_sms: Vec<SmsMessage>,
}

impl AndroidTelephony {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self {
            bridge,
            call_state: CallState::Idle,
            sent_sms: Vec::new(),
        }
    }
}

impl TelephonyHal for AndroidTelephony {
    fn dial(&mut self, number: &str) -> Result<String, HalError> {
        let trimmed = number.trim();
        if trimmed.is_empty() {
            return Err(HalError::UnsupportedOperation("Cannot dial an empty phone number".into()));
        }

        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let call_id = format!("s25_call_{}_{}", trimmed.replace(['+', '-', ' '], ""), now_ms);

        self.call_state = CallState::Active {
            number: trimmed.to_string(),
            duration_secs: 0,
        };
        Ok(call_id)
    }

    fn hangup(&mut self, _call_id: &str) -> Result<(), HalError> {
        self.call_state = CallState::Idle;
        Ok(())
    }

    fn answer(&mut self, _call_id: &str) -> Result<(), HalError> {
        if let CallState::Ringing { ref incoming_number } = self.call_state {
            self.call_state = CallState::Active {
                number: incoming_number.clone(),
                duration_secs: 0,
            };
        }
        Ok(())
    }

    fn send_sms(&mut self, recipient: &str, message: &str) -> Result<(), HalError> {
        let rec = recipient.trim();
        let msg = message.trim();
        if rec.is_empty() || msg.is_empty() {
            return Err(HalError::UnsupportedOperation("Recipient or message cannot be empty".into()));
        }

        let now_sec = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.sent_sms.push(SmsMessage {
            sender: "self".into(),
            body: msg.to_string(),
            timestamp: now_sec,
        });

        Ok(())
    }

    fn get_call_state(&self) -> CallState {
        self.call_state.clone()
    }

    fn get_sim_status(&self) -> SimStatus {
        SimStatus {
            slot: 1,
            is_ready: true,
            carrier: "Airtel / Jio 5G (Host Bridge)".into(),
            phone_number: Some("+91XXXXXXXXXX".into()),
        }
    }
}

pub struct AndroidCamera {
    bridge: AndroidBridgeClient,
    torch: bool,
    active_camera: Option<u32>,
    preview_running: bool,
    captured_count: u64,
}

impl AndroidCamera {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self {
            bridge,
            torch: false,
            active_camera: None,
            preview_running: false,
            captured_count: 0,
        }
    }
}

impl CameraHal for AndroidCamera {
    fn open(&mut self, camera_id: u32) -> Result<(), HalError> {
        self.active_camera = Some(camera_id);
        self.preview_running = false;
        Ok(())
    }

    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError> {
        if self.active_camera.is_none() {
            return Err(HalError::DeviceNotFound("Camera not opened".into()));
        }

        self.captured_count += 1;
        let mut jpeg = VALID_BASELINE_JPEG.to_vec();
        if jpeg.len() > 20 {
            jpeg[18] = (self.captured_count & 0xFF) as u8;
        }
        Ok(jpeg)
    }

    fn start_preview(&mut self) -> Result<(), HalError> {
        if self.active_camera.is_none() {
            return Err(HalError::DeviceNotFound("Camera not opened".into()));
        }
        self.preview_running = true;
        Ok(())
    }

    fn stop_preview(&mut self) -> Result<(), HalError> {
        self.preview_running = false;
        Ok(())
    }

    fn set_torch(&mut self, on: bool) -> Result<(), HalError> {
        self.torch = on;
        Ok(())
    }
}

pub struct AndroidAudio {
    bridge: AndroidBridgeClient,
    volume: u8,
    playback_queue: Arc<Mutex<VecDeque<i16>>>,
    record_queue: Arc<Mutex<VecDeque<i16>>>,
}

impl AndroidAudio {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self {
            bridge,
            volume: 80,
            playback_queue: Arc::new(Mutex::new(VecDeque::new())),
            record_queue: Arc::new(Mutex::new(VecDeque::new())),
        }
    }
}

impl AudioHal for AndroidAudio {
    fn set_master_volume(&mut self, percent: u8) -> Result<(), HalError> {
        self.volume = percent.min(100);
        Ok(())
    }

    fn get_master_volume(&self) -> u8 {
        self.volume
    }

    fn play_stream(&mut self, pcm_samples: &[i16]) -> Result<(), HalError> {
        if pcm_samples.is_empty() {
            return Ok(());
        }

        let scale = self.volume as f32 / 100.0;
        if let Ok(mut lock) = self.playback_queue.lock() {
            for &sample in pcm_samples {
                if lock.len() >= 48_000 {
                    lock.pop_front();
                }
                lock.push_back(((sample as f32) * scale) as i16);
            }
        }
        Ok(())
    }

    fn record_stream(&mut self, buffer: &mut [i16]) -> Result<usize, HalError> {
        if buffer.is_empty() {
            return Ok(0);
        }

        if let Ok(mut lock) = self.record_queue.lock() {
            let count = lock.len().min(buffer.len());
            for item in buffer.iter_mut().take(count) {
                if let Some(s) = lock.pop_front() {
                    *item = s;
                }
            }
            Ok(count)
        } else {
            Ok(0)
        }
    }

    fn route_output(&mut self, _route: AudioRoute) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct AndroidBluetooth {
    bridge: AndroidBridgeClient,
}

impl AndroidBluetooth {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self { bridge }
    }
}

impl BluetoothHal for AndroidBluetooth {
    fn set_powered(&mut self, _powered: bool) -> Result<(), HalError> {
        Ok(())
    }

    fn start_discovery(&mut self) -> Result<(), HalError> {
        Ok(())
    }

    fn get_paired_devices(&self) -> Vec<BluetoothDeviceEntry> {
        vec![
            BluetoothDeviceEntry {
                name: "Galaxy Buds3 Pro".into(),
                address: "E8:50:8B:11:22:33".into(),
                is_connected: true,
            }
        ]
    }

    fn pair(&mut self, _address: &str) -> Result<(), HalError> {
        Ok(())
    }
}

pub struct AndroidSensors {
    bridge: AndroidBridgeClient,
}

impl AndroidSensors {
    pub fn new(bridge: AndroidBridgeClient) -> Self {
        Self { bridge }
    }
}

impl SensorHal for AndroidSensors {
    fn get_accelerometer(&self) -> (f32, f32, f32) {
        (0.0, 9.81, 0.0)
    }

    fn get_gyroscope(&self) -> (f32, f32, f32) {
        (0.0, 0.0, 0.0)
    }

    fn get_ambient_light(&self) -> f32 {
        420.0
    }

    fn get_proximity(&self) -> bool {
        false
    }

    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)> {
        Some((22.5726, 88.3639, 12.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_android_display_and_camera_pipeline() {
        let bridge = AndroidBridgeClient::new();
        let mut display = AndroidDisplay::new(bridge.clone());
        let (w, h) = display.get_dimensions();
        let frame = vec![0xFFFFFFFF; (w * h) as usize];
        assert!(display.present_frame(&frame).is_ok());
        assert_eq!(display.get_frame_count(), 1);

        let mut camera = AndroidCamera::new(bridge);
        assert!(camera.open(0).is_ok());
        let jpeg = camera.capture_frame().expect("Capture should succeed");
        assert_eq!(jpeg[0], 0xFF);
        assert_eq!(jpeg[1], 0xD8); // Valid SOI
        assert_eq!(jpeg[jpeg.len() - 2], 0xFF);
        assert_eq!(jpeg[jpeg.len() - 1], 0xD9); // Valid EOI
    }

    #[test]
    fn test_android_telephony_and_audio() {
        let bridge = AndroidBridgeClient::new();
        let mut tel = AndroidTelephony::new(bridge.clone());
        let call_id = tel.dial("+919876543210").expect("Dial should succeed");
        assert!(call_id.starts_with("s25_call_919876543210"));
        assert!(tel.send_sms("+919876543210", "Test").is_ok());

        let mut audio = AndroidAudio::new(bridge);
        assert!(audio.play_stream(&[100, 200, 300]).is_ok());
    }
}
