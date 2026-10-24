// runtime/nilnet/src/lib.rs — Client for the netd network service.
//
// The counterpart to `nilpower`: it replaces the shell's former direct reads of
// `/sys/class/net`, which violated AGENTS.md §6. Network state is asked for over
// the canonical framed IPC protocol, and the absence of netd is reported as
// such rather than papered over with a fabricated link.

use std::path::Path;

/// Socket paths netd may be listening on, in preference order.
pub const NET_SOCKET_PATHS: &[&str] = &["/run/nilos/net.sock", "/run/onuron/net.sock"];

/// Environment variable that overrides the socket path, for integration tests
/// that start a real `netd` on a private socket. Only the address changes.
pub const NET_SOCKET_ENV: &str = "ONURON_NET_SOCKET";

/// The socket paths to try, with an override taking precedence when set.
pub fn socket_candidates() -> Vec<String> {
    if let Ok(override_path) = std::env::var(NET_SOCKET_ENV) {
        if !override_path.is_empty() {
            return vec![override_path];
        }
    }
    NET_SOCKET_PATHS.iter().map(|s| s.to_string()).collect()
}

pub struct NetClient;

impl NetClient {
    /// True when a netd socket exists.
    pub fn is_daemon_active() -> bool {
        socket_candidates().iter().any(|p| Path::new(p).exists())
    }

    /// The socket this client would talk to, if one exists.
    pub fn socket_path() -> Option<String> {
        socket_candidates()
            .into_iter()
            .find(|p| Path::new(p).exists())
    }

    /// Current network state from netd.
    ///
    /// When netd is unreachable the result is `NetworkStatePayload::default()`,
    /// which reports "not connected" with `is_simulated = true`. A default state
    /// is deliberately *pessimistic*: claiming connectivity that does not exist
    /// would be worse than showing nothing.
    pub fn state() -> Result<nilprotocol::NetworkStatePayload, String> {
        #[cfg(unix)]
        if let Some(state) = Self::request_over_ipc() {
            return Ok(state);
        }
        Ok(nilprotocol::NetworkStatePayload::default())
    }

    /// Like `state`, but fails when netd is unreachable. Used by callers that
    /// must not mistake "no daemon" for "no connectivity".
    pub fn try_state() -> Result<nilprotocol::NetworkStatePayload, String> {
        #[cfg(unix)]
        if let Some(state) = Self::request_over_ipc() {
            return Ok(state);
        }
        Err("netd is not reachable on any known socket".to_string())
    }

    #[cfg(unix)]
    fn request_over_ipc() -> Option<nilprotocol::NetworkStatePayload> {
        use std::os::unix::net::UnixStream;

        let req = Frame::new(MessageType::NetGetState, 1, Vec::new());
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
            // `NetScanWifi` answers with the same message type but a different
            // payload shape, and we never send it here, so a NetStateInfo frame
            // is unambiguously ours.
            if resp.message_type != u16::from(MessageType::NetStateInfo) {
                continue;
            }
            if let Ok(state) = resp.parse_json::<nilprotocol::NetworkStatePayload>() {
                return Some(state);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn state_always_returns_a_payload() {
        let state = NetClient::state().expect("state is infallible by design");
        // A state must never claim connectivity without naming the interface
        // that provides it.
        if state.is_connected {
            assert!(
                state.active_interface.is_some(),
                "connected state with no active interface"
            );
        }
        if let Some(iface) = state.active() {
            assert!(!iface.name.is_empty());
        }
    }

    #[test]
    fn default_state_is_pessimistic_and_flagged() {
        let d = nilprotocol::NetworkStatePayload::default();
        assert!(!d.is_connected, "a default state must not claim a link");
        assert!(d.active_interface.is_none());
        assert_eq!(d.connection_type, nilprotocol::ConnectionType::None);
        assert!(d.dns_servers.is_empty());
        assert!(d.is_simulated, "a default state must be flagged simulated");
    }

    #[test]
    fn try_state_reports_absence_instead_of_faking() {
        match NetClient::try_state() {
            Ok(state) => assert!(
                !state.is_simulated,
                "try_state must never return a simulated state"
            ),
            Err(e) => assert!(e.contains("netd"), "unhelpful error: {e}"),
        }
    }

    #[test]
    fn socket_path_agrees_with_the_existence_check() {
        assert_eq!(
            NetClient::socket_path().is_some(),
            NetClient::is_daemon_active()
        );
    }

    #[test]
    fn socket_candidates_is_never_empty() {
        assert!(!socket_candidates().is_empty());
    }

    #[test]
    fn active_resolves_the_named_interface() {
        use nilprotocol::{ConnectionType, NetworkInterfacePayload, NetworkStatePayload};
        let state = NetworkStatePayload {
            is_connected: true,
            active_interface: Some("wlan0".into()),
            connection_type: ConnectionType::Wifi,
            interfaces: vec![
                NetworkInterfacePayload {
                    name: "lo".into(),
                    conn_type: ConnectionType::Loopback,
                    operstate: "unknown".into(),
                    carrier_connected: true,
                    mac_address: String::new(),
                },
                NetworkInterfacePayload {
                    name: "wlan0".into(),
                    conn_type: ConnectionType::Wifi,
                    operstate: "up".into(),
                    carrier_connected: true,
                    mac_address: "aa:bb:cc:dd:ee:ff".into(),
                },
            ],
            dns_servers: vec!["1.1.1.1".into()],
            is_simulated: false,
        };
        let active = state.active().expect("active interface must resolve");
        assert_eq!(active.name, "wlan0");
        assert_eq!(active.conn_type, ConnectionType::Wifi);

        // A named interface that is not in the list resolves to nothing rather
        // than panicking.
        let dangling = NetworkStatePayload {
            active_interface: Some("gone0".into()),
            ..state
        };
        assert!(dangling.active().is_none());
    }
}
