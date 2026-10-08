// services/thermald/src/main.rs — Thermal Throttling & Power Capping
// Reads Linux thermal zones (/sys/class/thermal) and reduces performance when
// a zone runs too hot. Policy is kept in pure functions so it can be tested
// with synthetic sysfs content (no real hardware required).
use std::fs;
use std::path::Path;
use std::thread;
use std::time::Duration;

/// Temperature (degrees Celsius) above which the daemon throttles the CPU.
/// Conservative default for a mobile target.
pub const THERMAL_THROTTLE_C: i32 = 45;

/// Temperature (degrees Celsius) above which the daemon escalates to a
/// critical cooling state (aggressive CPU capping).
pub const THERMAL_CRITICAL_C: i32 = 65;

/// The action a temperature reading calls for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalAction {
    /// Within safe limits: do nothing.
    None,
    /// Above the throttle threshold: reduce performance.
    Throttle,
    /// Above the critical threshold: aggressively cap performance.
    Critical,
}

/// Parse a Linux thermal-zone `temp` value. The kernel reports millidegrees
/// Celsius (e.g. `"42000"` => 42 °C). Malformed or empty input yields `None`.
pub fn parse_temp_millicelsius(raw: &str) -> Option<i32> {
    raw.trim().parse::<i64>().ok().map(|milli| (milli / 1000) as i32)
}

/// Pure thermal policy: strictly *above* a threshold triggers its action, so a
/// reading exactly at the threshold is still safe.
pub fn thermal_action(temp_c: i32) -> ThermalAction {
    if temp_c > THERMAL_CRITICAL_C {
        ThermalAction::Critical
    } else if temp_c > THERMAL_THROTTLE_C {
        ThermalAction::Throttle
    } else {
        ThermalAction::None
    }
}

/// Build the log line for a reading, or `None` when no action is required.
pub fn thermal_event(name: &str, temp_c: i32) -> Option<String> {
    match thermal_action(temp_c) {
        ThermalAction::None => None,
        ThermalAction::Throttle => Some(format!(
            "{} at {}°C above {}°C — reducing performance (CPU governor → powersave)",
            name, temp_c, THERMAL_THROTTLE_C
        )),
        ThermalAction::Critical => Some(format!(
            "{} at {}°C above {}°C — CRITICAL thermal state, capping CPU",
            name, temp_c, THERMAL_CRITICAL_C
        )),
    }
}

/// Read a single thermal zone `temp` file. Returns `None` on a missing file, a
/// malformed value, or any I/O error — this never panics.
pub fn read_zone_temp(path: &Path) -> Option<i32> {
    let raw = fs::read_to_string(path).ok()?;
    parse_temp_millicelsius(&raw)
}

/// Scan a directory laid out like `/sys/class/thermal` and return the hottest
/// `thermal_zone*` reading as `(zone name, temp_c)`. Missing, non-zone, or
/// malformed sensors are skipped; no sensor -> `None`.
pub fn read_hottest_zone(base: &Path) -> Option<(String, i32)> {
    let entries = fs::read_dir(base).ok()?;
    let mut hottest: Option<(String, i32)> = None;
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        if !name.starts_with("thermal_zone") {
            continue;
        }
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(temp_c) = read_zone_temp(&path.join("temp")) else {
            continue;
        };
        if hottest.as_ref().map_or(true, |(_, prev)| temp_c > *prev) {
            hottest = Some((name, temp_c));
        }
    }
    hottest
}

/// Lower the CPU frequency policy. Failures are expected on development hosts
/// and are ignored; this is best-effort.
fn apply_throttle() {
    if let Ok(entries) = fs::read_dir("/sys/devices/system/cpu") {
        for entry in entries.flatten() {
            let governor = entry.path().join("cpufreq/scaling_governor");
            if governor.exists() {
                let _ = fs::write(governor, "powersave");
            }
        }
    }
}

fn main() {
    println!("\x1b[1;36m[thermald]\x1b[0m Thermal Throttling & Power Capping active.");
    loop {
        thread::sleep(Duration::from_secs(30));
        let Some((zone, temp_c)) = read_hottest_zone(Path::new("/sys/class/thermal")) else {
            // No readable thermal sensor (common under QEMU). Do not panic.
            continue;
        };
        if let Some(event) = thermal_event(&zone, temp_c) {
            eprintln!("\x1b[1;31m[thermald] [THROTTLE]\x1b[0m {}", event);
            apply_throttle();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── Synthetic sysfs parsing ─────────────────────────────────────────────

    #[test]
    fn parses_millidegree_readings() {
        assert_eq!(parse_temp_millicelsius("42000\n"), Some(42));
        assert_eq!(parse_temp_millicelsius("0"), Some(0));
        assert_eq!(parse_temp_millicelsius("-5000"), Some(-5));
        assert_eq!(parse_temp_millicelsius("45000"), Some(45));
    }

    #[test]
    fn malformed_or_empty_temperature_is_rejected_without_panicking() {
        assert_eq!(parse_temp_millicelsius(""), None);
        assert_eq!(parse_temp_millicelsius("\n"), None);
        assert_eq!(parse_temp_millicelsius("not-a-number"), None);
        assert_eq!(parse_temp_millicelsius("42.5"), None);
        assert_eq!(parse_temp_millicelsius("--1"), None);
    }

    // ── Policy boundaries ───────────────────────────────────────────────────

    #[test]
    fn thermal_policy_boundary_values() {
        assert_eq!(thermal_action(44), ThermalAction::None);
        // Exactly at the throttle threshold is still safe.
        assert_eq!(thermal_action(45), ThermalAction::None);
        assert_eq!(thermal_action(46), ThermalAction::Throttle);

        // Exactly at the critical threshold is a throttle, not critical.
        assert_eq!(thermal_action(65), ThermalAction::Throttle);
        assert_eq!(thermal_action(66), ThermalAction::Critical);
        assert_eq!(thermal_action(120), ThermalAction::Critical);

        // Cold readings never trigger anything.
        assert_eq!(thermal_action(-10), ThermalAction::None);
    }

    #[test]
    fn events_only_fire_above_threshold() {
        assert!(thermal_event("thermal_zone0", 44).is_none());
        assert!(thermal_event("thermal_zone0", 45).is_none());

        let throttle = thermal_event("thermal_zone0", 50).expect("throttle event");
        assert!(throttle.contains("thermal_zone0"));
        assert!(throttle.contains("powersave"));

        let critical = thermal_event("thermal_zone0", 80).expect("critical event");
        assert!(critical.contains("CRITICAL"));
    }

    // ── Sensor read failures ────────────────────────────────────────────────

    #[test]
    fn missing_thermal_directory_is_not_fatal() {
        assert!(read_hottest_zone(Path::new("/definitely/not/a/real/thermal/path")).is_none());
    }

    #[test]
    fn malformed_sensor_files_are_skipped() {
        let dir = std::env::temp_dir().join(format!(
            "thermald-malformed-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("thermal_zone0")).unwrap();
        fs::create_dir_all(dir.join("thermal_zone1")).unwrap();
        // Empty and garbage temp files must be skipped, not panicked on.
        fs::write(dir.join("thermal_zone0/temp"), "").unwrap();
        fs::write(dir.join("thermal_zone1/temp"), "garbage\n").unwrap();
        assert!(read_hottest_zone(&dir).is_none());
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn picks_hottest_valid_zone() {
        let dir = std::env::temp_dir().join(format!(
            "thermald-hottest-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(dir.join("thermal_zone0")).unwrap();
        fs::create_dir_all(dir.join("thermal_zone1")).unwrap();
        fs::write(dir.join("thermal_zone0/temp"), "41000\n").unwrap();
        fs::write(dir.join("thermal_zone1/temp"), "53000\n").unwrap();

        let (zone, temp_c) = read_hottest_zone(&dir).expect("a valid zone");
        assert_eq!(zone, "thermal_zone1");
        assert_eq!(temp_c, 53);
        assert_eq!(thermal_action(temp_c), ThermalAction::Throttle);
        let _ = fs::remove_dir_all(dir);
    }
}