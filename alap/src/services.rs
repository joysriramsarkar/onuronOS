// alap/src/services.rs — System Service Contracts for Alap Framework
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceStatus {
    Available,
    Unavailable { reason: String },
    PermissionDenied { permission: String },
}

pub type ServiceResult<T> = Result<T, ServiceError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceError {
    Unavailable(String),
    PermissionDenied(String),
    Timeout,
    Internal(String),
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(r) => write!(f, "Service unavailable: {r}"),
            Self::PermissionDenied(p) => write!(f, "Permission denied for: {p}"),
            Self::Timeout => write!(f, "Service request timed out"),
            Self::Internal(m) => write!(f, "Internal error: {m}"),
        }
    }
}

impl std::error::Error for ServiceError {}

// ─── Telephony Contract ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimInfo {
    pub is_ready: bool,
    pub carrier: String,
    pub signal_strength_bars: u8,
}

pub trait TelephonyService {
    fn get_sim_info(&self) -> ServiceResult<SimInfo>;
    fn dial(&self, number: &str) -> ServiceResult<String>;
    fn hangup(&self, call_id: &str) -> ServiceResult<()>;
}

// ─── Network Contract ────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkStatus {
    pub is_connected: bool,
    pub connection_type: String, // "wifi", "cellular", "ethernet", "none"
    pub ssid: Option<String>,
}

pub trait NetworkService {
    fn get_status(&self) -> ServiceResult<NetworkStatus>;
}

// ─── Power Contract ──────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryInfo {
    pub capacity_percent: u8,
    pub is_charging: bool,
}

pub trait PowerService {
    fn get_battery_info(&self) -> ServiceResult<BatteryInfo>;
}
