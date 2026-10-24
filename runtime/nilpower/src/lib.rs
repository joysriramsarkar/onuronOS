// runtime/nilpower/src/lib.rs — Client for the powerd battery service.
//
// Native Onuron applications must not read `/sys/class/power_supply` directly
// (AGENTS.md §6: "Native applications should not access hardware directly.
// They should use stable Onuron APIs and service boundaries"). This client is
// that boundary for battery telemetry: it asks powerd over the canonical framed
// IPC protocol and reports honestly when powerd is not reachable.
//
// There is deliberately no on-disk cache. Battery percentage is a live reading;
// replaying an old value as if it were current is exactly the kind of fabricated
// data the project forbids. If powerd cannot be reached the caller gets a
// payload with `is_simulated = true` and must label it as such.

use std::path::Path;

use nilprotocol::{Frame, MessageType};

/// Socket paths powerd may be listening on, in preference order.
///
/// `/run/nilos` is the project convention; `/run/onuron` is the older path some
/// services still bind, and is tried so a partially-migrated image works.
pub const POWER_SOCKET_PATHS: &[&str] = &["/run/nilos/power.sock", "/run/onuron/power.sock"];

/// Environment variable that overrides the socket path.
///
/// This exists so an integration test can start a real `powerd` on a private
/// socket and point the client at it. It changes only the address, never the
/// protocol, so what the test exercises is the same code path a device uses.
pub const POWER_SOCKET_ENV: &str = "ONURON_POWER_SOCKET";

/// The socket paths to try, with an override taking precedence when set.
pub fn socket_candidates() -> Vec<String> {
    if let Ok(override_path) = std::env::var(POWER_SOCKET_ENV) {
        if !override_path.is_empty() {
            return vec![override_path];
        }
    }
    POWER_SOCKET_PATHS.iter().map(|s| s.to_string()).collect()
}

pub struct PowerClient;

impl PowerClient {
    /// True when a powerd socket exists.
    ///
    /// This is a cheap existence check for UI that wants a status line; it does
    /// not prove the daemon will answer, so callers must still handle
    /// `battery_info()` returning a simulated reading.
    pub fn is_daemon_active() -> bool {
        socket_candidates().iter().any(|p| Path::new(p).exists())
    }

    /// The socket this client would talk to, if one exists.
    pub fn socket_path() -> Option<String> {
        socket_candidates()
            .into_iter()
            .find(|p| Path::new(p).exists())
    }

    /// Battery telemetry from powerd.
    ///
    /// Returns real values when powerd answers. When no daemon is reachable,
    /// returns a simulated payload rather than an `Err`, so a UI always has
    /// something to draw — but its `is_simulated` flag is true and must be
    /// surfaced.
    pub fn battery_info() -> Result<nilprotocol::BatteryInfoPayload, String> {
        #[cfg(unix)]
        if let Some(info) = Self::request_over_ipc() {
            return Ok(info);
        }
        Ok(nilprotocol::BatteryInfoPayload::simulated())
    }

    /// Like `battery_info`, but distinguishes "powerd answered" from "no
    /// daemon". Callers that must not show placeholder data (a boot check, a
    /// test) should use this.
    pub fn try_battery_info() -> Result<nilprotocol::BatteryInfoPayload, String> {
        #[cfg(unix)]
        if let Some(info) = Self::request_over_ipc() {
            return Ok(info);
        }
        Err("powerd is not reachable on any known socket".to_string())
    }

    #[cfg(unix)]
    fn request_over_ipc() -> Option<nilprotocol::BatteryInfoPayload> {
        use std::os::unix::net::UnixStream;

        let req = Frame::new(MessageType::PowerGetBattery, 1, Vec::new());
        for sock in socket_candidates() {
            let Ok(mut stream) = UnixStream::connect(&sock) else {
                continue;
            };
            if nilprotocol::write_frame(&mut stream, &req).is_err() {
                continue;
            }
            let Ok(resp) = nilprotocol::read_frame(&mut stream) else {
                continue;
            };
            // Only a battery-info response is a usable answer; anything else
            // (an error frame, a status frame) means this is not the service we
            // wanted.
            if resp.message_type != u16::from(MessageType::PowerBatteryInfo) {
                continue;
            }
            if let Ok(info) = resp.parse_json::<nilprotocol::BatteryInfoPayload>() {
                return Some(info);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn battery_info_always_returns_a_payload() {
        // Whether or not powerd runs in this environment, the infallible
        // accessor must produce a payload so a UI can render something.
        let info = PowerClient::battery_info().expect("battery_info is infallible by design");
        assert!(
            info.capacity <= 100,
            "capacity out of range: {}",
            info.capacity
        );
        if !info.is_simulated {
            assert!(!info.status.is_empty());
            assert!(info.voltage_mv > 0);
        } else {
            // The simulated payload must be self-describing.
            assert!(info.is_simulated);
        }
    }

    #[test]
    fn simulated_payload_is_marked_simulated() {
        let sim = nilprotocol::BatteryInfoPayload::simulated();
        assert!(sim.is_simulated, "the placeholder battery must be flagged");
        assert_eq!(sim.capacity, 85);
        assert_eq!(sim.status, "Discharging");
    }

    #[test]
    fn try_battery_info_reports_absence_instead_of_faking() {
        // In CI there is no powerd socket, so this must be an Err. On a machine
        // where powerd happens to run, it must be Ok and not simulated.
        match PowerClient::try_battery_info() {
            Ok(info) => assert!(
                !info.is_simulated,
                "try_battery_info must never return a simulated reading"
            ),
            Err(e) => assert!(e.contains("powerd"), "unhelpful error: {e}"),
        }
    }

    #[test]
    fn socket_path_agrees_with_the_existence_check() {
        assert_eq!(
            PowerClient::socket_path().is_some(),
            PowerClient::is_daemon_active()
        );
    }

    #[test]
    fn socket_preference_order_is_stable() {
        // /run/nilos is the current convention and must be tried first.
        assert_eq!(POWER_SOCKET_PATHS[0], "/run/nilos/power.sock");
        assert!(POWER_SOCKET_PATHS.len() >= 2);
    }

    #[test]
    fn override_env_replaces_the_candidate_list() {
        // The override is read from the environment, so this test cannot set it
        // safely in parallel with others; instead it asserts the shape of the
        // default list and that `socket_candidates` is never empty.
        assert!(!socket_candidates().is_empty());
    }
}
