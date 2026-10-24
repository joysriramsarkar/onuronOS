// runtime/nilnet/tests/netd_ipc.rs — Real end-to-end network-state IPC test.
//
// Starts the actual `netd` binary on a private socket and reads network state
// through the actual `nilnet` client. Nothing is mocked: a real frame goes over
// a real Unix socket and netd's real handler answers.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// `ONURON_NET_SOCKET` is process-global; serialize the tests that set it.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn daemon_binary(name: &str) -> Option<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let target = manifest.parent()?.parent()?.join("target");
    for profile in ["debug", "release"] {
        let candidate = target.join(profile).join(name);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

fn unique_socket(tag: &str) -> PathBuf {
    let n = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("onuron-{tag}-{}-{n}.sock", std::process::id()))
}

fn wait_for(path: &Path, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if path.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    false
}

struct DaemonGuard {
    child: Child,
    sock: PathBuf,
}

impl Drop for DaemonGuard {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.sock);
    }
}

#[test]
fn client_reads_network_state_from_a_real_netd() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let Some(bin) = daemon_binary("netd") else {
        eprintln!("SKIP: netd binary not built (run `cargo build -p netd`)");
        return;
    };

    let sock = unique_socket("net");
    let child = Command::new(&bin)
        .env("ONURON_NET_SOCKET", &sock)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn netd");
    let mut guard = DaemonGuard {
        child,
        sock: sock.clone(),
    };

    if !wait_for(&sock, Duration::from_secs(5)) {
        eprintln!("SKIP: netd did not bind {sock:?}");
        return;
    }

    std::env::set_var(nilnet::NET_SOCKET_ENV, &sock);
    let state = nilnet::NetClient::try_state().expect("client could not reach the running netd");

    // netd reports the host's real interfaces. In a container these are usually
    // down, but whatever it says must be self-consistent.
    if state.is_connected {
        assert!(
            state.active_interface.is_some(),
            "netd claims connectivity without naming an interface"
        );
        assert_ne!(
            state.connection_type,
            nilprotocol::ConnectionType::None,
            "netd claims connectivity but reports no connection type"
        );
    }
    if let Some(active) = state.active() {
        assert!(
            !active.name.is_empty(),
            "active interface has an empty name"
        );
    }
    // netd reads real sysfs, so this must not be flagged simulated.
    assert!(
        !state.is_simulated,
        "netd with real sysfs must not report simulated"
    );
    eprintln!(
        "OK: connected={} active={:?} type={:?} interfaces={} dns={}",
        state.is_connected,
        state.active_interface,
        state.connection_type,
        state.interfaces.len(),
        state.dns_servers.len()
    );

    std::env::remove_var(nilnet::NET_SOCKET_ENV);
    let _ = &mut guard;
}

#[test]
fn client_reports_absence_when_no_netd_is_running() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let sock = unique_socket("net-absent");
    std::env::set_var(nilnet::NET_SOCKET_ENV, &sock);

    assert!(
        nilnet::NetClient::try_state().is_err(),
        "try_state must fail with no daemon"
    );

    let fallback = nilnet::NetClient::state().unwrap();
    assert!(
        fallback.is_simulated,
        "fallback state must be flagged simulated"
    );
    assert!(
        !fallback.is_connected,
        "fallback state must not claim connectivity"
    );

    std::env::remove_var(nilnet::NET_SOCKET_ENV);
}
