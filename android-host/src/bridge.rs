// android-host/src/bridge.rs — Onuron Android Host IPC Bridge Protocol
// Facilitates bidirectional communication between the Samsung Galaxy S25 Android Host
// APK and the Onuron userspace runtime via Unix domain sockets or TCP localhost streams.

use serde::{Deserialize, Serialize};

/// Messages sent from Samsung Android Host (APK/Service) to Onuron Userspace
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum HostToGuestEvent {
    // Input
    TouchEvent {
        action: String, // "DOWN", "MOVE", "UP", "CANCEL"
        pointer_id: u32,
        x: f32,
        y: f32,
        pressure: f32,
    },
    KeyEvent {
        action: String, // "DOWN", "UP"
        keycode: u32,
        character: Option<char>,
    },
    // Telephony
    IncomingCall {
        call_id: String,
        caller_number: String,
    },
    CallStateChanged {
        call_id: String,
        state: String, // "DIALING", "ACTIVE", "DISCONNECTED"
    },
    SmsReceived {
        sender: String,
        body: String,
        timestamp: u64,
    },
    // Power
    BatteryUpdate {
        level: u8,
        is_charging: bool,
        temperature_c: f32,
        voltage_mv: u32,
    },
    // Network
    NetworkUpdate {
        is_connected: bool,
        conn_type: String, // "WIFI", "CELLULAR_5G", "NONE"
        ip_address: Option<String>,
        ssid: Option<String>,
    },
    // Sensors
    SensorData {
        sensor_type: String, // "ACCELEROMETER", "GYROSCOPE", "LIGHT", "PROXIMITY"
        values: Vec<f32>,
    },
    GpsUpdate {
        latitude: f64,
        longitude: f64,
        altitude: f32,
    },
    // Lifecycle
    HostPause,
    HostResume,
}

/// Commands and requests sent from Onuron Userspace to Samsung Android Host (APK/Service)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action", content = "params")]
pub enum GuestToHostCommand {
    // Display
    SetBrightness { percent: u8 },
    SetKeepScreenOn { keep_on: bool },

    // Telephony
    DialNumber { number: String },
    HangupCall { call_id: String },
    AnswerCall { call_id: String },
    SendSms { recipient: String, message: String },

    // Network
    ScanWifi,
    ConnectWifi { ssid: String, password: String },

    // Camera
    StartCameraPreview { camera_id: u32 },
    StopCameraPreview,
    CapturePhoto { camera_id: u32 },
    SetTorch { enabled: bool },

    // Audio & Feedback
    Vibrate { duration_ms: u32 },
    SetVolume { stream_type: String, percent: u8 },

    // Notifications
    PostSystemNotification {
        id: u32,
        title: String,
        content: String,
    },

    // Ping / Handshake
    Handshake { runtime_version: String },
}

/// Response returned from Android Host for synchronous or correlated requests
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BridgeResponse {
    pub success: bool,
    pub error: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl BridgeResponse {
    pub fn ok(data: Option<serde_json::Value>) -> Self {
        Self {
            success: true,
            error: None,
            data,
        }
    }

    pub fn err(message: impl Into<String>) -> Self {
        Self {
            success: false,
            error: Some(message.into()),
            data: None,
        }
    }
}
