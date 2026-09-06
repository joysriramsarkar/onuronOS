// runtime/nilhal/src/traits.rs — Unified Rust Hardware Abstraction Traits
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HalError {
    DeviceNotFound(String),
    PermissionDenied(String),
    HardwareBusy,
    UnsupportedOperation(String),
    BridgeError(String),
    IoError(String),
}

impl std::fmt::Display for HalError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HalError::DeviceNotFound(s) => write!(f, "Device not found: {}", s),
            HalError::PermissionDenied(s) => write!(f, "Permission denied: {}", s),
            HalError::HardwareBusy => write!(f, "Hardware busy"),
            HalError::UnsupportedOperation(s) => write!(f, "Unsupported operation: {}", s),
            HalError::BridgeError(s) => write!(f, "Host bridge error: {}", s),
            HalError::IoError(s) => write!(f, "I/O error: {}", s),
        }
    }
}

impl std::error::Error for HalError {}

// ─── Display HAL ─────────────────────────────────────────────────────────────
pub trait DisplayHal: Send + Sync {
    fn get_dimensions(&self) -> (u32, u32);
    fn get_refresh_rate(&self) -> u32; // e.g. 60, 120 (S25 AMOLED)
    fn present_frame(&mut self, buffer: &[u32]) -> Result<(), HalError>;
    fn set_brightness(&mut self, percent: u8) -> Result<(), HalError>;
    fn get_brightness(&self) -> u8;
}

// ─── Input HAL & Gestures ───────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SwipeDirection {
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TouchGesture {
    Tap { x: f32, y: f32 },
    DoubleTap { x: f32, y: f32 },
    LongPress { x: f32, y: f32 },
    Swipe { direction: SwipeDirection, velocity: f32 },
    Drag { x: f32, y: f32, dx: f32, dy: f32 },
    MultiTouchPinch { center_x: f32, center_y: f32, scale: f32 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum HalInputEvent {
    TouchDown { id: u32, x: f32, y: f32 },
    TouchMove { id: u32, x: f32, y: f32 },
    TouchUp { id: u32 },
    Gesture(TouchGesture),
    KeyDown { code: u32, name: String },
    KeyUp { code: u32, name: String },
}

pub trait InputHal: Send + Sync {
    fn poll_events(&mut self) -> Vec<HalInputEvent>;
    fn send_event(&mut self, event: HalInputEvent) -> Result<(), HalError>;
}

// ─── Network HAL ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ConnectionType {
    None,
    Ethernet,
    Wifi,
    Cellular,
    Loopback,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WifiApInfo {
    pub ssid: String,
    pub bssid: String,
    pub signal_level: i32, // dBm or 0-100
    pub security: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalNetworkState {
    pub is_connected: bool,
    pub active_interface: Option<String>,
    pub connection_type: ConnectionType,
    pub ip_address: Option<String>,
    pub dns_servers: Vec<String>,
    pub wifi_ssid: Option<String>,
    pub cellular_carrier: Option<String>,
}

pub trait NetworkHal: Send + Sync {
    fn get_state(&self) -> HalNetworkState;
    fn scan_wifi(&mut self) -> Result<Vec<WifiApInfo>, HalError>;
    fn connect_wifi(&mut self, ssid: &str, psk: &str) -> Result<(), HalError>;
    fn set_cellular_enabled(&mut self, enabled: bool) -> Result<(), HalError>;
}

// ─── Power HAL ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum PerformanceMode {
    PowerSaver,
    Balanced,
    HighPerformance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HalBatteryInfo {
    pub capacity: u8,            // 0 - 100%
    pub is_charging: bool,
    pub status: String,          // "Charging", "Discharging", "Full"
    pub voltage_mv: u32,
    pub temp_c: f32,
    pub health: String,
}

pub trait PowerHal: Send + Sync {
    fn get_battery_info(&self) -> HalBatteryInfo;
    fn set_screen_timeout(&mut self, seconds: u32) -> Result<(), HalError>;
    fn set_performance_mode(&mut self, mode: PerformanceMode) -> Result<(), HalError>;
    fn acquire_wakelock(&mut self, tag: &str) -> Result<(), HalError>;
    fn release_wakelock(&mut self, tag: &str) -> Result<(), HalError>;
}

// ─── Telephony HAL ──────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum CallState {
    Idle,
    Ringing { incoming_number: String },
    Active { number: String, duration_secs: u64 },
    Held,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SimStatus {
    pub slot: u8,
    pub is_ready: bool,
    pub carrier: String,
    pub phone_number: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SmsMessage {
    pub sender: String,
    pub body: String,
    pub timestamp: u64,
}

pub trait TelephonyHal: Send + Sync {
    fn dial(&mut self, number: &str) -> Result<String, HalError>;
    fn hangup(&mut self, call_id: &str) -> Result<(), HalError>;
    fn answer(&mut self, call_id: &str) -> Result<(), HalError>;
    fn send_sms(&mut self, recipient: &str, message: &str) -> Result<(), HalError>;
    fn get_call_state(&self) -> CallState;
    fn get_sim_status(&self) -> SimStatus;
}

// ─── Camera HAL ─────────────────────────────────────────────────────────────
pub trait CameraHal: Send + Sync {
    fn open(&mut self, camera_id: u32) -> Result<(), HalError>;
    fn capture_frame(&mut self) -> Result<Vec<u8>, HalError>;
    fn start_preview(&mut self) -> Result<(), HalError>;
    fn stop_preview(&mut self) -> Result<(), HalError>;
    fn set_torch(&mut self, on: bool) -> Result<(), HalError>;
}

// ─── Audio HAL ──────────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AudioRoute {
    Speaker,
    Earpiece,
    Headset,
    Bluetooth,
}

pub trait AudioHal: Send + Sync {
    fn set_master_volume(&mut self, percent: u8) -> Result<(), HalError>;
    fn get_master_volume(&self) -> u8;
    fn play_stream(&mut self, pcm_samples: &[i16]) -> Result<(), HalError>;
    fn record_stream(&mut self, buffer: &mut [i16]) -> Result<usize, HalError>;
    fn route_output(&mut self, route: AudioRoute) -> Result<(), HalError>;
}

// ─── Bluetooth HAL ──────────────────────────────────────────────────────────
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BluetoothDeviceEntry {
    pub name: String,
    pub address: String,
    pub is_connected: bool,
}

pub trait BluetoothHal: Send + Sync {
    fn set_powered(&mut self, powered: bool) -> Result<(), HalError>;
    fn start_discovery(&mut self) -> Result<(), HalError>;
    fn get_paired_devices(&self) -> Vec<BluetoothDeviceEntry>;
    fn pair(&mut self, address: &str) -> Result<(), HalError>;
}

// ─── Sensors HAL ────────────────────────────────────────────────────────────
pub trait SensorHal: Send + Sync {
    fn get_accelerometer(&self) -> (f32, f32, f32);
    fn get_gyroscope(&self) -> (f32, f32, f32);
    fn get_ambient_light(&self) -> f32; // Lux
    fn get_proximity(&self) -> bool;     // true = close
    fn get_gps_coordinates(&self) -> Option<(f64, f64, f32)>; // (lat, lon, alt)
}
