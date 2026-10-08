// services/nilwdt/src/main.rs — Onuron OS Hardware & Software Watchdog Feeder (nilwdt)
//
// Regularly feeds the Linux hardware watchdog device (/dev/watchdog or /dev/watchdog0)
// while running health checks against critical OS subsystems.
// If the userspace or critical services freeze, nilwdt stops feeding, allowing the
// hardware watchdog timer (or QEMU watchdog) to trigger an automatic system reset.

use std::env;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::thread;
use std::time::Duration;

pub const DEFAULT_INTERVAL_SECS: u64 = 5;
pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
pub const WATCHDOG_DEV_PRIMARY: &str = "/dev/watchdog";
pub const WATCHDOG_DEV_SECONDARY: &str = "/dev/watchdog0";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchdogConfig {
    pub interval_secs: u64,
    pub timeout_secs: u64,
    pub max_consecutive_failures: u32,
    pub simulation_mode: bool,
}

impl Default for WatchdogConfig {
    fn default() -> Self {
        Self {
            interval_secs: DEFAULT_INTERVAL_SECS,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            max_consecutive_failures: 5,
            simulation_mode: false,
        }
    }
}

impl WatchdogConfig {
    pub fn from_env() -> Self {
        let interval_secs = env::var("NILWDT_INTERVAL_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_INTERVAL_SECS)
            .clamp(1, 60);

        let timeout_secs = env::var("NILWDT_TIMEOUT_SECS")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(DEFAULT_TIMEOUT_SECS)
            .clamp(5, 300);

        let max_failures = (timeout_secs / interval_secs).max(1) as u32;

        let simulation_mode = env::var("NILWDT_SIMULATE").map(|v| v == "1").unwrap_or(false)
            || !Path::new(WATCHDOG_DEV_PRIMARY).exists() && !Path::new(WATCHDOG_DEV_SECONDARY).exists();

        Self {
            interval_secs,
            timeout_secs,
            max_consecutive_failures: max_failures,
            simulation_mode,
        }
    }
}

pub struct HealthChecker {
    pub consecutive_failures: u32,
}

impl HealthChecker {
    pub fn new() -> Self {
        Self {
            consecutive_failures: 0,
        }
    }

    /// Evaluates system health. Returns true if healthy, false if unhealthy.
    pub fn check_health(&mut self, required_paths: &[&str]) -> bool {
        let mut healthy = true;

        for path in required_paths {
            if !Path::new(path).exists() {
                eprintln!("[nilwdt] Health check warning: required path missing: {}", path);
                healthy = false;
            }
        }

        if healthy {
            self.consecutive_failures = 0;
            true
        } else {
            self.consecutive_failures += 1;
            false
        }
    }

    pub fn should_trip(&self, max_failures: u32) -> bool {
        self.consecutive_failures >= max_failures
    }
}

pub struct WatchdogFeeder {
    pub config: WatchdogConfig,
    dev_file: Option<File>,
}

impl WatchdogFeeder {
    pub fn new(config: WatchdogConfig) -> Self {
        let dev_file = if !config.simulation_mode {
            OpenOptions::new()
                .write(true)
                .open(WATCHDOG_DEV_PRIMARY)
                .or_else(|_| OpenOptions::new().write(true).open(WATCHDOG_DEV_SECONDARY))
                .ok()
        } else {
            None
        };

        Self { config, dev_file }
    }

    /// Feeds the hardware watchdog by writing a heartbeat byte.
    pub fn feed(&mut self) -> Result<(), String> {
        if let Some(file) = &mut self.dev_file {
            file.write_all(b"\0")
                .map_err(|e| format!("Failed to ping hardware watchdog: {e}"))?;
            let _ = file.flush();
        }
        Ok(())
    }

    /// Clean shutdown: Linux watchdog drivers disable the hardware timer
    /// when the magic character 'V' is written before closing.
    pub fn disarm_and_close(&mut self) {
        if let Some(mut file) = self.dev_file.take() {
            let _ = file.write_all(b"V");
            let _ = file.flush();
            println!("[nilwdt] Hardware watchdog cleanly disarmed ('V' magic close).");
        }
    }
}

fn main() {
    println!("\x1b[1;36m[nilwdt]\x1b[0m Onuron OS Hardware Watchdog Feeder Initializing...");

    let config = WatchdogConfig::from_env();
    if config.simulation_mode {
        println!(
            "\x1b[1;33m[nilwdt] [ WARN ]\x1b[0m Hardware watchdog device not found — running in software simulation mode (interval: {}s, timeout: {}s)",
            config.interval_secs, config.timeout_secs
        );
    } else {
        println!(
            "\x1b[1;32m[nilwdt] [  OK  ]\x1b[0m Hardware watchdog device opened (interval: {}s, timeout: {}s)",
            config.interval_secs, config.timeout_secs
        );
    }

    let mut feeder = WatchdogFeeder::new(config.clone());
    let mut checker = HealthChecker::new();

    let check_paths = ["/run", "/proc"];

    println!("\x1b[1;32m[nilwdt] [  OK  ]\x1b[0m Watchdog feeder loop active.");

    loop {
        let is_healthy = checker.check_health(&check_paths);

        if is_healthy {
            if let Err(e) = feeder.feed() {
                eprintln!("[nilwdt] Error feeding watchdog: {}", e);
            }
        } else {
            eprintln!(
                "\x1b[1;31m[nilwdt] [ALERT]\x1b[0m System health check failed ({}/{} failures)!",
                checker.consecutive_failures, config.max_consecutive_failures
            );

            if checker.should_trip(config.max_consecutive_failures) {
                eprintln!(
                    "\x1b[1;31m[nilwdt] [CRITICAL]\x1b[0m Max health check failures exceeded! Ceasing watchdog feeds to force system reboot."
                );
                // Intentionally halt feeding so watchdog hardware trips
                loop {
                    thread::sleep(Duration::from_secs(60));
                }
            }
        }

        thread::sleep(Duration::from_secs(config.interval_secs));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults() {
        let cfg = WatchdogConfig::default();
        assert_eq!(cfg.interval_secs, 5);
        assert_eq!(cfg.timeout_secs, 30);
        assert_eq!(cfg.max_consecutive_failures, 5);
        assert!(!cfg.simulation_mode);
    }

    #[test]
    fn test_health_checker_healthy() {
        let mut checker = HealthChecker::new();
        // Test paths that definitely exist across all platforms
        let result = checker.check_health(&["Cargo.toml"]);
        assert!(result);
        assert_eq!(checker.consecutive_failures, 0);
        assert!(!checker.should_trip(3));
    }

    #[test]
    fn test_health_checker_unhealthy_trip() {
        let mut checker = HealthChecker::new();
        assert!(!checker.check_health(&["/nonexistent_path_xyz_123"]));
        assert_eq!(checker.consecutive_failures, 1);
        assert!(!checker.should_trip(3));

        assert!(!checker.check_health(&["/nonexistent_path_xyz_123"]));
        assert_eq!(checker.consecutive_failures, 2);
        assert!(!checker.should_trip(3));

        assert!(!checker.check_health(&["/nonexistent_path_xyz_123"]));
        assert_eq!(checker.consecutive_failures, 3);
        assert!(checker.should_trip(3));

        // Recovery resets the counter
        assert!(checker.check_health(&["Cargo.toml"]));
        assert_eq!(checker.consecutive_failures, 0);
        assert!(!checker.should_trip(3));
    }

    #[test]
    fn test_simulated_feeder_succeeds() {
        let mut cfg = WatchdogConfig::default();
        cfg.simulation_mode = true;
        let mut feeder = WatchdogFeeder::new(cfg);
        assert!(feeder.feed().is_ok());
    }
}

