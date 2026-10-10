// services/powerd/src/main.rs — Onuron OS Power Governor & Suspend/Wakelock Manager
// Reads Linux sysfs (/sys/class/power_supply), manages wakelocks, and controls screen brightness/suspend.

use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct BatteryInfo {
    pub capacity: u8,               // 0 - 100%
    pub status: String,             // "Charging", "Discharging", "Full", "Not charging"
    pub is_charging: bool,
    pub voltage_mv: u32,            // Millivolts
    pub temp_c: f32,                // Celsius
    pub health: String,             // "Good", "Overheat", "Dead"
    pub is_simulated: bool,
}

impl Default for BatteryInfo {
    fn default() -> Self {
        Self {
            capacity: 85,
            status: "Discharging".into(),
            is_charging: false,
            voltage_mv: 3820,
            temp_c: 29.5,
            health: "Good".into(),
            is_simulated: true,
        }
    }
}

pub const MAX_WAKELOCK_DURATION: Duration = Duration::from_secs(30 * 60); // 30 minutes hard limit
pub const DEFAULT_WAKELOCK_TIMEOUT: Duration = Duration::from_secs(10 * 60); // 10 minutes default auto-expire

#[derive(Clone, Debug, PartialEq)]
pub struct WakelockEntry {
    pub tag: String,
    pub acquired_at: Instant,
    pub expires_at: Instant,
}

pub struct PowerGovernor {
    wakelocks: std::collections::HashMap<String, WakelockEntry>,
    screen_timeout_secs: u64,
    last_user_activity: Instant,
    screen_on: bool,
    perf_mode: nilhal::traits::PerformanceMode,
}

impl PowerGovernor {
    pub fn new() -> Self {
        Self {
            wakelocks: std::collections::HashMap::new(),
            screen_timeout_secs: 60,
            last_user_activity: Instant::now(),
            screen_on: true,
            perf_mode: nilhal::traits::PerformanceMode::Balanced,
        }
    }

    pub fn acquire_wakelock(&mut self, tag: &str) {
        self.acquire_wakelock_with_timeout(tag, DEFAULT_WAKELOCK_TIMEOUT);
    }

    pub fn acquire_wakelock_with_timeout(&mut self, tag: &str, requested_timeout: Duration) {
        let duration = requested_timeout.min(MAX_WAKELOCK_DURATION);
        let now = Instant::now();
        let entry = WakelockEntry {
            tag: tag.to_string(),
            acquired_at: now,
            expires_at: now + duration,
        };
        self.wakelocks.insert(tag.to_string(), entry);
    }

    pub fn release_wakelock(&mut self, tag: &str) -> bool {
        self.wakelocks.remove(tag).is_some()
    }

    pub fn evict_expired_wakelocks(&mut self) -> usize {
        let now = Instant::now();
        let before = self.wakelocks.len();
        self.wakelocks.retain(|tag, entry| {
            if entry.expires_at <= now {
                eprintln!(
                    "[powerd:governor] Evicting leaked/expired wakelock '{}' after {:?} (exceeded deadline)",
                    tag,
                    now.duration_since(entry.acquired_at)
                );
                false
            } else {
                true
            }
        });
        before - self.wakelocks.len()
    }

    pub fn has_wakelocks(&mut self) -> bool {
        self.evict_expired_wakelocks();
        !self.wakelocks.is_empty()
    }

    pub fn active_wakelocks(&mut self) -> Vec<String> {
        self.evict_expired_wakelocks();
        self.wakelocks.keys().cloned().collect()
    }

    pub fn set_performance_mode(&mut self, mode: nilhal::traits::PerformanceMode) {
        self.perf_mode = mode;
    }

    pub fn performance_mode(&self) -> &nilhal::traits::PerformanceMode {
        &self.perf_mode
    }

    pub fn set_screen_timeout(&mut self, secs: u64) {
        self.screen_timeout_secs = secs;
    }

    pub fn screen_timeout(&self) -> u64 {
        self.screen_timeout_secs
    }

    pub fn notify_activity(&mut self) {
        self.last_user_activity = Instant::now();
        self.screen_on = true;
    }

    pub fn check_idle_timeout(&mut self) -> bool {
        if self.has_wakelocks() {
            return false;
        }
        self.last_user_activity.elapsed() >= Duration::from_secs(self.screen_timeout_secs)
    }
}

/// Read battery telemetry using NilHAL (sysfs, Android Host Bridge, or QEMU)
pub fn read_battery_info() -> BatteryInfo {
    let hal = nilhal::NilHal::auto();
    let hal_bat = hal.power.get_battery_info();

    BatteryInfo {
        capacity: hal_bat.capacity,
        status: hal_bat.status,
        is_charging: hal_bat.is_charging,
        voltage_mv: hal_bat.voltage_mv,
        temp_c: hal_bat.temp_c,
        health: hal_bat.health,
        is_simulated: hal.backend_type == nilhal::BackendType::Qemu,
    }
}

pub fn read_sysfs_battery() -> BatteryInfo {
    read_battery_info()
}

// ─── Battery policy (pure, testable) ────────────────────────────────────────

/// Warn when the battery drops *below* this capacity percentage.
///
/// Conservative default: 5%. The comparison is strict, so exactly 5% does not
/// warn; 4% does.
pub const LOW_BATTERY_WARN_PERCENT: u8 = 5;

/// Request a graceful shutdown when the battery drops *below* this capacity.
///
/// Conservative default: 3%. Deliberately lower than the warning threshold so
/// a device that cannot charge still has headroom to warn the user and persist
/// state before powering off.
pub const CRITICAL_BATTERY_SHUTDOWN_PERCENT: u8 = 3;

/// What the power governor should do for a battery reading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BatteryAction {
    /// Battery is healthy (or charging): do nothing.
    None,
    /// Battery is low: emit a warning event.
    WarnLowBattery,
    /// Battery is critically low: request an orderly poweroff.
    RequestShutdown,
}

/// Parse a sysfs `capacity` value, e.g. `"42\n"`. Values above 100 are clamped.
/// Malformed or empty input yields `None` rather than a fabricated reading.
pub fn parse_capacity(raw: &str) -> Option<u8> {
    raw.trim().parse::<u8>().ok().map(|v| v.min(100))
}

/// Parse a sysfs `status` value into `(normalized status, power connected)`.
///
/// A device that is `Charging` or `Full` is treated as power-connected, which
/// suppresses the shutdown path.
pub fn parse_status(raw: &str) -> (String, bool) {
    let status = raw.trim().to_string();
    let connected = status.eq_ignore_ascii_case("charging")
        || status.eq_ignore_ascii_case("full");
    (status, connected)
}

/// Pure battery policy.
///
/// `power_connected` (charging or full) always suppresses the shutdown request:
/// a device that is plugged in must never power itself off. The low-battery
/// *warning* is still emitted while charging, since it is informational.
pub fn battery_action(capacity: u8, power_connected: bool) -> BatteryAction {
    if !power_connected && capacity < CRITICAL_BATTERY_SHUTDOWN_PERCENT {
        BatteryAction::RequestShutdown
    } else if capacity < LOW_BATTERY_WARN_PERCENT {
        BatteryAction::WarnLowBattery
    } else {
        BatteryAction::None
    }
}

/// Ask the system to power off gracefully. Prefers the init-provided poweroff
/// binary; failure is logged and never panics. This only *requests* shutdown —
/// nilinit remains the component that actually stops the system.
pub fn request_graceful_shutdown() {
    for cmd in ["/sbin/poweroff", "/bin/poweroff", "/usr/bin/poweroff"] {
        if Path::new(cmd).exists() {
            match std::process::Command::new(cmd).status() {
                Ok(_) => return,
                Err(e) => eprintln!("[powerd] poweroff via {} failed: {}", cmd, e),
            }
        }
    }
    eprintln!("[powerd] no poweroff binary found; graceful shutdown request logged only.");
}

pub fn set_backlight(level: u32) -> Result<(), String> {
    let backlight_dir = Path::new("/sys/class/backlight");
    if backlight_dir.exists() {
        if let Ok(entries) = fs::read_dir(backlight_dir) {
            for entry in entries.flatten() {
                let brightness_file = entry.path().join("brightness");
                if brightness_file.exists() {
                    return fs::write(brightness_file, level.to_string())
                        .map_err(|e| format!("Failed to set brightness: {}", e));
                }
            }
        }
    }
    Ok(())
}

use nilprotocol::{Frame, MessageType};

pub fn handle_ipc_request(frame: &Frame, governor: &Arc<Mutex<PowerGovernor>>) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => {
            Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec())
        }
        MessageType::ServiceStatusRequest => {
            let bat = read_sysfs_battery();
            let payload = nilprotocol::ServiceStatusPayload {
                service_name: "powerd".to_string(),
                is_ready: true,
                is_simulated: bat.is_simulated,
                backend_name: if bat.is_simulated { "simulated".to_string() } else { "linux-power_supply".to_string() },
                uptime_secs: 0,
                request_count: 1,
                last_error: None,
            };
            Frame::with_json(MessageType::ServiceStatusResponse, frame.request_id, &payload)
                .unwrap_or_else(|_| Frame::new(MessageType::ErrorResponse, frame.request_id, b"encode error".to_vec()))
        }
        MessageType::PowerGetBattery => {
            let bat = read_sysfs_battery();
            let json = serde_json::to_vec(&bat).unwrap_or_default();
            Frame::new(MessageType::PowerBatteryInfo, frame.request_id, json)
        }
        MessageType::PowerAcquireWakelock => {
            let tag = String::from_utf8_lossy(&frame.payload).trim().to_string();
            let mut g = governor.lock().unwrap();
            g.acquire_wakelock(&tag);
            Frame::new(MessageType::Pong, frame.request_id, b"acquired".to_vec())
        }
        MessageType::PowerReleaseWakelock => {
            let tag = String::from_utf8_lossy(&frame.payload).trim().to_string();
            let mut g = governor.lock().unwrap();
            let ok = g.release_wakelock(&tag);
            Frame::new(
                MessageType::Pong,
                frame.request_id,
                if ok { b"released".to_vec() } else { b"not_found".to_vec() },
            )
        }
        _ => {
            Frame::new(MessageType::ErrorResponse, frame.request_id, b"unsupported message type".to_vec())
        }
    }
}

fn main() {
    println!("\x1b[1;36m[powerd]\x1b[0m Onuron OS Power Governor & Suspend Manager Initializing...");

    let governor = Arc::new(Mutex::new(PowerGovernor::new()));
    let _ = fs::create_dir_all("/run/onuron");

    let initial_bat = read_sysfs_battery();
    println!(
        "\x1b[1;32m[powerd] [  OK  ]\x1b[0m Battery Telemetry: {}% ({}, {:.1}°C, {} mV) [simulated={}]",
        initial_bat.capacity, initial_bat.status, initial_bat.temp_c, initial_bat.voltage_mv, initial_bat.is_simulated
    );

    // Periodic Power Monitor & Idle Governor Thread
    let gov_clone = Arc::clone(&governor);
    thread::spawn(move || {
        loop {
            thread::sleep(Duration::from_secs(10));
            let mut gov = gov_clone.lock().unwrap();
            let bat = read_sysfs_battery();

            match battery_action(bat.capacity, bat.is_charging) {
                BatteryAction::RequestShutdown => {
                    eprintln!(
                        "\x1b[1;31m[powerd] [CRITICAL]\x1b[0m Battery {}% is below {}%! Requesting graceful shutdown.",
                        bat.capacity, CRITICAL_BATTERY_SHUTDOWN_PERCENT
                    );
                    request_graceful_shutdown();
                }
                BatteryAction::WarnLowBattery => {
                    eprintln!(
                        "\x1b[1;33m[powerd] [WARN]\x1b[0m Battery low: {}% is below {}%.",
                        bat.capacity, LOW_BATTERY_WARN_PERCENT
                    );
                }
                BatteryAction::None => {}
            }

            if gov.check_idle_timeout() {
                // In a real device, write "mem" to /sys/power/state
                println!("[powerd] Idle timeout reached with no wakelocks. Ready to suspend.");
            }
        }
    });

    #[cfg(unix)]
    {
        let gov_ipc = Arc::clone(&governor);
        thread::spawn(move || {
            let listener = match nilsd::first_listener_or_bind("/run/onuron/power.sock") {
                Ok(l) => l,
                Err(e) => {
                    eprintln!("[powerd] Failed to bind IPC socket /run/onuron/power.sock: {}", e);
                    return;
                }
            };
            println!("\x1b[1;32m[powerd] [  OK  ]\x1b[0m IPC Server listening on /run/onuron/power.sock");

            for stream in listener.incoming() {
                if let Ok(mut sock) = stream {
                    let gov = Arc::clone(&gov_ipc);
                    thread::spawn(move || {
                        while let Ok(frame) = Frame::read_from(&mut sock) {
                            let resp = handle_ipc_request(&frame, &gov);
                            if let Err(e) = resp.write_to(&mut sock) {
                                eprintln!("[powerd] IPC send error: {}", e);
                                break;
                            }
                        }
                    });
                }
            }
        });
    }

    let _ = nilsd::notify_ready("powerd", Some("/run/onuron/power.sock"));
    println!("\x1b[1;32m[powerd] [  OK  ]\x1b[0m Power manager daemon active (/run/onuron/power.sock)");

    // Keep service alive
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wakelock_management() {
        let mut gov = PowerGovernor::new();
        assert!(!gov.has_wakelocks());

        gov.acquire_wakelock("audio_stream");
        assert!(gov.has_wakelocks());
        assert_eq!(gov.active_wakelocks(), vec!["audio_stream".to_string()]);

        // Idle timeout should never fire while wakelock is held
        assert!(!gov.check_idle_timeout());

        let removed = gov.release_wakelock("audio_stream");
        assert!(removed);
        assert!(!gov.has_wakelocks());
    }

    #[test]
    fn test_battery_default_fallback() {
        let bat = BatteryInfo::default();
        assert_eq!(bat.capacity, 85);
        assert_eq!(bat.status, "Discharging");
        assert_eq!(bat.is_charging, false);
        assert!(bat.is_simulated);
    }

    // ── Synthetic sysfs parsing ─────────────────────────────────────────────

    #[test]
    fn capacity_parses_sysfs_formatting_and_rejects_garbage() {
        assert_eq!(parse_capacity("57\n"), Some(57));
        assert_eq!(parse_capacity("  4 "), Some(4));
        assert_eq!(parse_capacity("0"), Some(0));
        assert_eq!(parse_capacity("100"), Some(100));
        // Values above 100 are clamped rather than trusted.
        assert_eq!(parse_capacity("255"), Some(100));
        // Malformed / empty readings are rejected, never defaulted silently.
        assert_eq!(parse_capacity(""), None);
        assert_eq!(parse_capacity("not-a-number"), None);
        assert_eq!(parse_capacity("1.5"), None);
        assert_eq!(parse_capacity("-1"), None);
    }

    #[test]
    fn status_parser_flags_power_connected_states() {
        assert!(parse_status("Charging\n").1);
        assert!(parse_status("Full").1);
        assert!(!parse_status("Discharging\n").1);
        assert!(!parse_status("Not charging").1);
        let (status, connected) = parse_status("Discharging\n");
        assert_eq!(status, "Discharging");
        assert!(!connected);
        // Missing/empty status is treated as not connected.
        assert!(!parse_status("").1);
    }

    // ── Policy boundaries ───────────────────────────────────────────────────

    #[test]
    fn battery_policy_boundary_values() {
        // 6% and exactly-at-threshold 5% are fine.
        assert_eq!(battery_action(6, false), BatteryAction::None);
        assert_eq!(battery_action(5, false), BatteryAction::None);

        // Below the warning threshold but at/above critical: warn only.
        assert_eq!(battery_action(4, false), BatteryAction::WarnLowBattery);
        assert_eq!(battery_action(3, false), BatteryAction::WarnLowBattery);

        // Below the critical threshold: graceful shutdown request.
        assert_eq!(battery_action(2, false), BatteryAction::RequestShutdown);
        assert_eq!(battery_action(1, false), BatteryAction::RequestShutdown);
        assert_eq!(battery_action(0, false), BatteryAction::RequestShutdown);
    }

    #[test]
    fn charging_suppresses_shutdown_but_still_warns() {
        // A charging device must never request shutdown, even at 0%.
        assert_eq!(battery_action(0, true), BatteryAction::WarnLowBattery);
        assert_eq!(battery_action(2, true), BatteryAction::WarnLowBattery);
        // Below the warning threshold it warns; at/above it does nothing.
        assert_eq!(battery_action(4, true), BatteryAction::WarnLowBattery);
        assert_eq!(battery_action(5, true), BatteryAction::None);
        assert_eq!(battery_action(80, true), BatteryAction::None);
    }

    #[test]
    fn full_status_is_power_connected_and_suppresses_shutdown() {
        let (status, connected) = parse_status("Full\n");
        assert_eq!(status, "Full");
        assert!(connected);
        // 1% would shut down when discharging, but not while full/plugged in.
        assert_ne!(battery_action(1, connected), BatteryAction::RequestShutdown);
        assert_eq!(battery_action(1, connected), BatteryAction::WarnLowBattery);
    }

    #[test]
    fn test_power_ipc_handling() {
        let gov = Arc::new(Mutex::new(PowerGovernor::new()));

        // Ping -> Pong
        let ping_frame = Frame::new(MessageType::Ping, 10, b"ping".to_vec());
        let resp = handle_ipc_request(&ping_frame, &gov);
        assert_eq!(MessageType::from(resp.message_type), MessageType::Pong);
        assert_eq!(resp.request_id, 10);
        assert_eq!(resp.payload, b"pong");

        // PowerGetBattery -> PowerBatteryInfo
        let bat_frame = Frame::new(MessageType::PowerGetBattery, 11, vec![]);
        let resp = handle_ipc_request(&bat_frame, &gov);
        assert_eq!(MessageType::from(resp.message_type), MessageType::PowerBatteryInfo);
        assert_eq!(resp.request_id, 11);
        let bat: BatteryInfo = serde_json::from_slice(&resp.payload).unwrap();
        assert!(bat.capacity <= 100);

        // Wakelock acquire and release
        let lock_frame = Frame::new(MessageType::PowerAcquireWakelock, 12, b"music_player".to_vec());
        let resp = handle_ipc_request(&lock_frame, &gov);
        assert_eq!(MessageType::from(resp.message_type), MessageType::Pong);
        assert_eq!(resp.payload, b"acquired");
        assert!(gov.lock().unwrap().has_wakelocks());

        let unlock_frame = Frame::new(MessageType::PowerReleaseWakelock, 13, b"music_player".to_vec());
        let resp = handle_ipc_request(&unlock_frame, &gov);
        assert_eq!(MessageType::from(resp.message_type), MessageType::Pong);
        assert_eq!(resp.payload, b"released");
        assert!(!gov.lock().unwrap().has_wakelocks());

        // ServiceStatusRequest -> ServiceStatusResponse
        let status_frame = Frame::new(MessageType::ServiceStatusRequest, 14, vec![]);
        let resp = handle_ipc_request(&status_frame, &gov);
        assert_eq!(MessageType::from(resp.message_type), MessageType::ServiceStatusResponse);
        let status: nilprotocol::ServiceStatusPayload = resp.parse_json().unwrap();
        assert_eq!(status.service_name, "powerd");
        assert!(status.is_ready);
    }

    #[test]
    fn test_wakelock_expiration_and_leak_eviction() {
        let mut gov = PowerGovernor::new();
        // Acquire short-lived wakelock
        gov.acquire_wakelock_with_timeout("temp_task", Duration::from_millis(20));
        assert!(gov.has_wakelocks());
        assert_eq!(gov.active_wakelocks(), vec!["temp_task"]);

        // Wait for it to expire
        std::thread::sleep(Duration::from_millis(35));

        // Expired wakelock is evicted automatically, unblocking idle timeout
        assert!(!gov.has_wakelocks());
        assert!(gov.active_wakelocks().is_empty());
    }

    #[test]
    fn test_wakelock_max_duration_clamp() {
        let mut gov = PowerGovernor::new();
        // Request excessive 100-hour wakelock
        gov.acquire_wakelock_with_timeout("leaky_app", Duration::from_secs(100 * 3600));

        let entry = gov.wakelocks.get("leaky_app").unwrap();
        let requested_deadline = entry.expires_at.duration_since(entry.acquired_at);
        assert_eq!(requested_deadline, MAX_WAKELOCK_DURATION);
    }
}

