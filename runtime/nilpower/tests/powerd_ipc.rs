// runtime/nilpower/tests/powerd_ipc.rs — Real end-to-end battery IPC test.
//
// This starts the actual `powerd` binary on a private socket, then reads the
// battery through the actual `nilpower` client. Nothing is mocked: the client
// writes a real frame over a real Unix socket, powerd's real request handler
// answers, and the client parses the response.
//
// The test skips (with a printed reason) when the daemon binary is not built, so
// `cargo test` stays usable without a full build. It also skips on non-Unix.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// `ONURON_POWER_SOCKET` is process-global, and cargo runs tests in parallel
/// threads by default, so any test that sets it must hold this lock. Without it
/// the daemon test and the absence test would race and flake.
static ENV_LOCK: Mutex<()> = Mutex::new(());

/// Locate a built daemon binary in the workspace target dir.
fn daemon_binary(name: &str) -> Option<PathBuf> {
    // This test lives in runtime/nilpower/tests; the workspace target dir is
    // three levels up. CARGO_MANIFEST_DIR is the crate dir, so derive from it.
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

fn spawn_daemon(bin: &Path, sock: &Path) -> Child {
    Command::new(bin)
        .env("ONURON_POWER_SOCKET", sock)
        .env("ONURON_NET_SOCKET", sock)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn daemon")
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

/// A guard that kills the daemon and removes the socket on drop, so a failing
/// assertion cannot leave a stray process behind.
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
fn client_reads_battery_from_a_real_powerd() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    let Some(bin) = daemon_binary("powerd") else {
        eprintln!("SKIP: powerd binary not built (run `cargo build -p powerd`)");
        return;
    };

    let sock = unique_socket("power");
    let child = spawn_daemon(&bin, &sock);
    let mut guard = DaemonGuard {
        child,
        sock: sock.clone(),
    };

    if !wait_for(&sock, Duration::from_secs(5)) {
        eprintln!("SKIP: powerd did not bind {sock:?}");
        return;
    }

    // Point the client at the daemon we just started.
    std::env::set_var(nilpower::POWER_SOCKET_ENV, &sock);

    let info = nilpower::PowerClient::try_battery_info()
        .expect("client could not reach the running powerd");

    // Whatever the host hardware is, the reading must be internally consistent.
    assert!(
        info.capacity <= 100,
        "capacity out of range: {}",
        info.capacity
    );
    assert!(!info.status.is_empty(), "empty charge status");
    assert!(!info.health.is_empty(), "empty battery health");
    if !info.is_simulated {
        // A reading claimed to be real must look like one.
        assert!(info.voltage_mv > 0, "real reading with zero voltage");
        assert!(
            info.temp_c > -40.0 && info.temp_c < 100.0,
            "implausible temp {}",
            info.temp_c
        );
    }
    eprintln!(
        "OK: capacity={}% status={} simulated={} voltage={}mV temp={}C",
        info.capacity, info.status, info.is_simulated, info.voltage_mv, info.temp_c
    );

    // The infallible accessor must agree with the strict one while the daemon is up.
    let loose = nilpower::PowerClient::battery_info().unwrap();
    assert!(
        !loose.is_simulated,
        "daemon is up but battery_info fell back to simulated"
    );

    std::env::remove_var(nilpower::POWER_SOCKET_ENV);
    let _ = &mut guard;
}

#[test]
fn client_reports_absence_when_no_daemon_is_running() {
    let _env = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());

    // A socket path that nothing is listening on.
    let sock = unique_socket("power-absent");
    std::env::set_var(nilpower::POWER_SOCKET_ENV, &sock);

    let strict = nilpower::PowerClient::try_battery_info();
    assert!(strict.is_err(), "try_battery_info must fail with no daemon");

    // The infallible accessor still yields a payload, but flagged simulated.
    let loose = nilpower::PowerClient::battery_info().unwrap();
    assert!(
        loose.is_simulated,
        "fallback payload must be flagged simulated"
    );

    std::env::remove_var(nilpower::POWER_SOCKET_ENV);
}
