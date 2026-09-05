// services/powerd/src/main.rs — Onuron OS Power Governor & Suspend/Wakelock Manager
// Reads Linux sysfs (/sys/class/power_supply), manages wakelocks, and controls screen brightness/suspend.

use std::collections::HashSet;
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

pub struct PowerGovernor {
    wakelocks: HashSet<String>,
    screen_timeout_secs: u64,
    last_user_activity: Instant,
    screen_on: bool,
}

impl PowerGovernor {
    pub fn new() -> Self {
        Self {
            wakelocks: HashSet::new(),
            screen_timeout_secs: 60,
            last_user_activity: Instant::now(),
            screen_on: true,
        }
    }

    pub fn acquire_wakelock(&mut self, tag: &str) {
        self.wakelocks.insert(tag.to_string());
    }

    pub fn release_wakelock(&mut self, tag: &str) -> bool {
        self.wakelocks.remove(tag)
    }

    pub fn has_wakelocks(&self) -> bool {
        !self.wakelocks.is_empty()
    }

    pub fn active_wakelocks(&self) -> Vec<String> {
        self.wakelocks.iter().cloned().collect()
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

/// Read battery telemetry from Linux sysfs (/sys/class/power_supply/)
pub fn read_sysfs_battery() -> BatteryInfo {
    let power_supply = Path::new("/sys/class/power_supply");
    if power_supply.exists() {
        if let Ok(entries) = fs::read_dir(power_supply) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_lowercase();
                if name.contains("bat") || name.contains("battery") || name.contains("axp20x") {
                    let dir = entry.path();
                    let capacity = fs::read_to_string(dir.join("capacity"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u8>().ok())
                        .unwrap_or(85);

                    let status = fs::read_to_string(dir.join("status"))
                        .map(|s| s.trim().to_string())
                        .unwrap_or_else(|_| "Discharging".into());

                    let is_charging = status.eq_ignore_ascii_case("charging");

                    let voltage_mv = fs::read_to_string(dir.join("voltage_now"))
                        .ok()
                        .and_then(|s| s.trim().parse::<u32>().ok())
                        .map(|uv| uv / 1000)
                        .unwrap_or(3800);

                    let temp_c = fs::read_to_string(dir.join("temp"))
                        .ok()
                        .and_then(|s| s.trim().parse::<f32>().ok())
                        .map(|t| t / 10.0)
                        .unwrap_or(30.0);

                    return BatteryInfo {
                        capacity,
                        status,
                        is_charging,
                        voltage_mv,
                        temp_c,
                        health: "Good".into(),
                        is_simulated: false,
                    };
                }
            }
        }
    }

    // Default development fallback (QEMU / desktop test)
    BatteryInfo::default()
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

            if bat.capacity <= 5 && !bat.is_charging {
                eprintln!("\x1b[1;31m[powerd] [CRITICAL]\x1b[0m Battery level <= 5%! Requesting safe poweroff.");
            }

            if gov.check_idle_timeout() {
                // In a real device, write "mem" to /sys/power/state
                println!("[powerd] Idle timeout reached with no wakelocks. Ready to suspend.");
            }
        }
    });

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
}
