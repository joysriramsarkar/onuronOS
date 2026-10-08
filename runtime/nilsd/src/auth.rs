// runtime/nilsd/src/auth.rs — Unix-socket peer authorization (improvement C1).
//
// Every service that opens a Unix socket should verify *who* connected before
// acting on the request. On Linux this is available without trusting anything
// the client sends: `SO_PEERCRED` returns the connecting process's pid/uid/gid
// as recorded by the kernel at `connect(2)` time.
//
// Policy is loaded from `/etc/nilos/ipc-policy.toml` (see the embedded default
// below). A missing service entry is denied by default — new sockets must be
// explicitly granted, which is the property the audit is trying to establish.

use std::collections::HashMap;
use std::path::Path;

/// The kernel-attested identity of a connected Unix-socket peer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PeerCred {
    pub pid: i32,
    pub uid: u32,
    pub gid: u32,
}

/// Default location of the IPC authorization policy.
pub const DEFAULT_POLICY_PATH: &str = "/etc/nilos/ipc-policy.toml";

/// Shipped default: privileged daemons accept root plus the Onuron UI user.
/// `keyd` and the Android bridge accept root only.
pub const DEFAULT_POLICY_TOML: &str = include_str!("../../../etc/nilos/ipc-policy.toml");

/// Per-service allow rules. A peer is accepted when any rule matches.
#[derive(Debug, Clone, Default, serde::Deserialize, PartialEq, Eq)]
pub struct ServicePolicy {
    /// Accept any peer (only for genuinely public sockets).
    #[serde(default)]
    pub allow_all: bool,
    /// Accepted uid values.
    #[serde(default)]
    pub allowed_uids: Vec<u32>,
    /// Accepted gid values.
    #[serde(default)]
    pub allowed_gids: Vec<u32>,
}

impl ServicePolicy {
    pub fn permits(&self, cred: PeerCred) -> bool {
        if self.allow_all {
            return true;
        }
        self.allowed_uids.contains(&cred.uid) || self.allowed_gids.contains(&cred.gid)
    }
}

#[derive(Debug, Clone, Default, serde::Deserialize)]
struct PolicyFile {
    #[serde(default)]
    services: HashMap<String, ServicePolicy>,
}

/// A parsed IPC authorization policy.
#[derive(Debug, Clone, Default)]
pub struct IpcPolicy {
    services: HashMap<String, ServicePolicy>,
}

impl IpcPolicy {
    /// Parse a policy from TOML text.
    pub fn from_toml(text: &str) -> Result<Self, String> {
        let file: PolicyFile =
            toml::from_str(text).map_err(|e| format!("invalid IPC policy: {e}"))?;
        Ok(IpcPolicy { services: file.services })
    }

    /// The built-in policy shipped with the OS.
    pub fn builtin() -> Self {
        Self::from_toml(DEFAULT_POLICY_TOML).expect("embedded IPC policy must parse")
    }

    /// Load from `path`, falling back to the built-in policy when the file is
    /// absent or malformed (logged, not fatal — an unreadable policy must not
    /// stop daemons from booting, but it must also not silently allow).
    pub fn load_or_builtin(path: &Path) -> Self {
        match std::fs::read_to_string(path) {
            Ok(text) => match Self::from_toml(&text) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!(
                        "[nilsd:auth] {e}; falling back to built-in IPC policy"
                    );
                    Self::builtin()
                }
            },
            Err(_) => Self::builtin(),
        }
    }

    /// The policy for a service, if defined.
    pub fn service(&self, service: &str) -> Option<&ServicePolicy> {
        self.services.get(service)
    }

    /// Decide whether `cred` may talk to `service`. Unknown services are
    /// denied so every new socket has to be added to the policy deliberately.
    pub fn authorize(&self, service: &str, cred: PeerCred) -> Result<(), String> {
        match self.services.get(service) {
            Some(policy) if policy.permits(cred) => Ok(()),
            Some(_) => Err(format!(
                "denied: uid {} gid {} pid {} is not permitted to use '{}'",
                cred.uid, cred.gid, cred.pid, service
            )),
            None => Err(format!(
                "denied: no IPC policy entry for service '{}' (add it to {DEFAULT_POLICY_PATH})",
                service
            )),
        }
    }
}

/// Look up an already-open policy, or the built-in one if loading failed.
pub fn load_default_policy() -> IpcPolicy {
    IpcPolicy::load_or_builtin(Path::new(DEFAULT_POLICY_PATH))
}

/// Authorize a `std` Unix socket connection, logging and returning `false` on
/// denial or when peer credentials cannot be read (fail closed).
#[cfg(unix)]
pub fn authorize_stream(policy: &IpcPolicy, service: &str, stream: &std::os::unix::net::UnixStream) -> bool {
    match peer_cred(stream) {
        Ok(cred) => match policy.authorize(service, cred) {
            Ok(()) => true,
            Err(reason) => {
                eprintln!("[nilsd:auth] {reason}");
                false
            }
        },
        Err(e) => {
            eprintln!("[nilsd:auth] cannot read peer credentials for '{service}': {e}");
            false
        }
    }
}

/// Authorize a connection by raw fd (for tokio `UnixStream`). Fail-closed.
#[cfg(unix)]
pub fn authorize_fd(policy: &IpcPolicy, service: &str, fd: std::os::unix::io::RawFd) -> bool {
    match peer_cred_fd(fd) {
        Ok(cred) => match policy.authorize(service, cred) {
            Ok(()) => true,
            Err(reason) => {
                eprintln!("[nilsd:auth] {reason}");
                false
            }
        },
        Err(e) => {
            eprintln!("[nilsd:auth] cannot read peer credentials for '{service}': {e}");
            false
        }
    }
}

/// Read peer credentials directly from a raw file descriptor. Works for both
/// `std` and tokio Unix sockets (call `AsRawFd::as_raw_fd` first).
#[cfg(unix)]
pub fn peer_cred_fd(fd: std::os::unix::io::RawFd) -> std::io::Result<PeerCred> {
    let mut cred: libc::ucred = unsafe { std::mem::zeroed() };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    let rc = unsafe {
        libc::getsockopt(
            fd,
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            &mut cred as *mut libc::ucred as *mut libc::c_void,
            &mut len,
        )
    };
    if rc != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(PeerCred {
        pid: cred.pid,
        uid: cred.uid,
        gid: cred.gid,
    })
}

/// Non-Unix fallback: there is no SO_PEERCRED. Callers on these platforms are
/// expected to run in "simulated" mode and not gate on peer identity.
#[cfg(not(unix))]
pub fn peer_cred_fd(_fd: i32) -> std::io::Result<PeerCred> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "SO_PEERCRED is not available on this platform",
    ))
}

/// Read the peer credentials of a connected `std` Unix socket.
#[cfg(unix)]
pub fn peer_cred(stream: &std::os::unix::net::UnixStream) -> std::io::Result<PeerCred> {
    use std::os::unix::io::AsRawFd;
    peer_cred_fd(stream.as_raw_fd())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cred(uid: u32, gid: u32, pid: i32) -> PeerCred {
        PeerCred { uid, gid, pid }
    }

    #[test]
    fn builtin_policy_parses_and_gates_keyd_to_root() {
        let policy = IpcPolicy::builtin();
        assert!(policy.authorize("keyd", cred(0, 0, 1)).is_ok());
        assert!(policy.authorize("keyd", cred(1000, 1000, 2)).is_err());
    }

    #[test]
    fn unknown_service_is_denied() {
        let policy = IpcPolicy::builtin();
        let err = policy
            .authorize("does-not-exist", cred(0, 0, 1))
            .unwrap_err();
        assert!(err.contains("no IPC policy entry"), "{err}");
    }

    #[test]
    fn gid_rule_can_authorize_a_group() {
        let policy = IpcPolicy::from_toml(
            r#"
            [services.example]
            allowed_gids = [42]
            "#,
        )
        .unwrap();
        assert!(policy.authorize("example", cred(500, 42, 9)).is_ok());
        assert!(policy.authorize("example", cred(500, 7, 9)).is_err());
    }

    #[test]
    fn allow_all_bypasses_uid_and_gid() {
        let policy = IpcPolicy::from_toml(
            r#"
            [services.public]
            allow_all = true
            "#,
        )
        .unwrap();
        assert!(policy.authorize("public", cred(12345, 12345, 9)).is_ok());
    }

    #[test]
    fn root_is_not_implicitly_trusted_for_a_uid_listed_service() {
        // A service that only lists uid 1000 must still reject root.
        let policy = IpcPolicy::from_toml(
            r#"
            [services.shellonly]
            allowed_uids = [1000]
            "#,
        )
        .unwrap();
        assert!(policy.authorize("shellonly", cred(1000, 0, 1)).is_ok());
        assert!(policy.authorize("shellonly", cred(0, 0, 1)).is_err());
    }

    #[test]
    fn malformed_policy_is_rejected() {
        assert!(IpcPolicy::from_toml("this is not = = toml").is_err());
    }
}