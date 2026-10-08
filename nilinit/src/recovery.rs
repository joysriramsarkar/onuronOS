// nilinit/src/recovery.rs — Recovery boot path selection and failed-boot counter.
//
// Two independent triggers drop the device into `nilrecovery` instead of the
// normal UI:
//
//   1. An explicit `onuron.recovery=1` kernel command line (or `recovery`).
//   2. `MAX_FAILED_BOOTS` consecutive boots that never reached the point where
//      nilinit reported boot completion. The counter lives on persistent
//      storage so it survives a boot loop, and is reset to zero only after a
//      successful boot.
//
// The decision logic is pure and unit-tested; the boot loop in `main.rs` only
// performs the file I/O and the actual `exec`.

/// Consecutive failed boots before recovery is entered automatically.
pub const MAX_FAILED_BOOTS: u32 = 3;

/// Default location of the failed-boot counter (on the persistent partition).
pub const BOOT_COUNT_PATH: &str = "/data/system/boot_count";

/// Why the device is entering (or not entering) recovery.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BootDecision {
    /// Boot normally. `failed_boots` is the counter value to persist.
    Normal { failed_boots: u32 },
    /// Enter recovery.
    Recovery { reason: RecoveryReason },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryReason {
    /// Explicitly requested on the kernel command line.
    Requested,
    /// Too many consecutive failed boots.
    TooManyFailedBoots(u32),
}

impl RecoveryReason {
    pub fn describe(&self) -> String {
        match self {
            RecoveryReason::Requested => {
                "recovery requested on the kernel command line".to_string()
            }
            RecoveryReason::TooManyFailedBoots(n) => {
                format!("{n} consecutive failed boots (limit {MAX_FAILED_BOOTS})")
            }
        }
    }
}

/// True when the kernel command line explicitly asks for recovery.
pub fn recovery_requested(cmdline: &str) -> bool {
    let lower = cmdline.to_ascii_lowercase();
    for token in lower.split_whitespace() {
        if token == "recovery" || token == "onuron.recovery" || token == "onuron.recovery=1" {
            return true;
        }
    }
    false
}

/// Decide how to boot given the command line and the previous failed-boot
/// count read from disk.
///
/// A normal boot increments the counter (because we cannot yet know the boot
/// will succeed); the caller must call [`reset_boot_count_decision`] — i.e.
/// write zero — only after boot completion.
pub fn plan_boot(cmdline: &str, previous_failed_boots: u32) -> BootDecision {
    if recovery_requested(cmdline) {
        return BootDecision::Recovery { reason: RecoveryReason::Requested };
    }
    let next = previous_failed_boots.saturating_add(1);
    if next >= MAX_FAILED_BOOTS {
        BootDecision::Recovery { reason: RecoveryReason::TooManyFailedBoots(next) }
    } else {
        BootDecision::Normal { failed_boots: next }
    }
}

/// The counter value to write after a successful boot.
pub fn successful_boot_counter() -> u32 {
    0
}

/// Parse a counter file's contents, tolerating garbage (treated as 0 with a
/// warning) so a corrupt file cannot permanently trap the device in recovery.
pub fn parse_boot_count(raw: &str) -> u32 {
    raw.trim().parse::<u32>().unwrap_or(0)
}

/// Read the boot counter from `path`, returning 0 when it is missing or
/// malformed.
pub fn read_boot_count(path: &str) -> u32 {
    match std::fs::read_to_string(path) {
        Ok(raw) => parse_boot_count(&raw),
        Err(_) => 0,
    }
}

/// Persist the boot counter, creating the parent directory if needed.
pub fn write_boot_count(path: &str, value: u32) -> std::io::Result<()> {
    if let Some(parent) = std::path::Path::new(path).parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    std::fs::write(path, value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_flag_requests_recovery() {
        assert!(recovery_requested("root=/dev/vda onuron.recovery=1"));
        assert!(recovery_requested("root=/dev/vda recovery"));
        assert!(recovery_requested("onuron.recovery=1"));
        assert!(!recovery_requested("quiet splash"));
        // Must not be fooled by a substring.
        assert!(!recovery_requested("onuron.recovery=0"));
        assert!(!recovery_requested("not-recovery"));
    }

    #[test]
    fn explicit_flag_wins_over_counter() {
        assert_eq!(
            plan_boot("onuron.recovery=1", 0),
            BootDecision::Recovery { reason: RecoveryReason::Requested }
        );
    }

    #[test]
    fn counter_increments_on_normal_boot() {
        assert_eq!(plan_boot("", 0), BootDecision::Normal { failed_boots: 1 });
        assert_eq!(plan_boot("quiet", 1), BootDecision::Normal { failed_boots: 2 });
    }

    #[test]
    fn third_failed_boot_enters_recovery() {
        assert_eq!(
            plan_boot("", MAX_FAILED_BOOTS - 1),
            BootDecision::Recovery { reason: RecoveryReason::TooManyFailedBoots(MAX_FAILED_BOOTS) }
        );
        // Saturating: a huge counter stays in recovery, never overflows.
        assert!(matches!(
            plan_boot("", u32::MAX),
            BootDecision::Recovery { .. }
        ));
    }

    #[test]
    fn corrupt_counter_is_treated_as_zero() {
        assert_eq!(parse_boot_count(""), 0);
        assert_eq!(parse_boot_count("  \n"), 0);
        assert_eq!(parse_boot_count("not-a-number"), 0);
        assert_eq!(parse_boot_count("-5"), 0);
        assert_eq!(parse_boot_count("7"), 7);
    }

    #[test]
    fn counter_round_trips_on_disk() {
        let path = std::env::temp_dir()
            .join(format!("onuron-bootcount-{:016x}", rand_like()))
            .join("boot_count");
        let path_str = path.to_string_lossy().to_string();
        assert_eq!(read_boot_count(&path_str), 0);
        write_boot_count(&path_str, 2).unwrap();
        assert_eq!(read_boot_count(&path_str), 2);
        write_boot_count(&path_str, successful_boot_counter()).unwrap();
        assert_eq!(read_boot_count(&path_str), 0);
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    // Tiny helper so this module needs no extra dependency. Not cryptographic.
    fn rand_like() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0)
    }
}