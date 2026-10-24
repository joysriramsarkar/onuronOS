// runtime/nilui-gpu/tests/wayland_cross_process.rs — cross-process compositor test
//
// The unit tests in `src/wayland/server.rs` drive a client and the server inside
// one process. That shares an address space, so it cannot prove the descriptor
// passing works between processes, and it cannot catch a presenter that never
// runs.
//
// This test starts the real `wayland_compositor` binary, connects the real
// `wayland_surface_producer` binary to it, and checks that the compositor
// reports having composited the surface. Both are separate processes talking
// over a real Unix socket with real SCM_RIGHTS descriptor passing.

use std::io::Read;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// A unique directory for this test's socket, removed on drop.
struct TempRuntime {
    dir: PathBuf,
}

impl TempRuntime {
    fn new(tag: &str) -> Self {
        let n = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let dir =
            std::env::temp_dir().join(format!("onuron-wl-it-{tag}-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create runtime dir");
        Self { dir }
    }

    fn socket(&self) -> PathBuf {
        self.dir.join("wayland-0")
    }
}

impl Drop for TempRuntime {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Kills a child process on drop, so a failed assertion cannot leak a process.
struct ChildGuard(Child);

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// Wait until `predicate` holds, or give up after `timeout`.
fn wait_until(timeout: Duration, mut predicate: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if predicate() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    predicate()
}

fn read_log(path: &std::path::Path) -> String {
    let mut s = String::new();
    if let Ok(mut f) = std::fs::File::open(path) {
        let _ = f.read_to_string(&mut s);
    }
    s
}

#[test]
fn compositor_composites_a_surface_from_a_separate_client_process() {
    let rt = TempRuntime::new("e2e");
    let compositor_bin = env!("CARGO_BIN_EXE_wayland_compositor");
    let producer_bin = env!("CARGO_BIN_EXE_wayland_surface_producer");

    // The compositor runs until it is killed; its stdout is captured so the
    // test can assert on what it actually did rather than assume.
    let log_path = rt.dir.join("compositor.log");
    let log = std::fs::File::create(&log_path).expect("create log");
    let compositor = Command::new(compositor_bin)
        .args(["--width", "128", "--height", "128"])
        .env("XDG_RUNTIME_DIR", &rt.dir)
        .stdout(Stdio::from(log))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn compositor");
    let _guard = ChildGuard(compositor);

    // Wait for the socket rather than sleeping a fixed amount.
    assert!(
        wait_until(Duration::from_secs(10), || rt.socket().exists()),
        "compositor did not create its socket at {}",
        rt.socket().display()
    );

    // Run the client. A non-zero exit means the handshake failed.
    let out = Command::new(producer_bin)
        .args(["--width", "64", "--height", "64", "--frames", "3"])
        .env("XDG_RUNTIME_DIR", &rt.dir)
        .output()
        .expect("spawn producer");
    let stdout = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        out.status.success(),
        "producer failed\nstdout:\n{stdout}\nstderr:\n{}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The producer must have completed the whole flow, not just connected.
    assert!(
        stdout.contains("surface created (64x64)"),
        "the surface was not created at the expected size\nstdout:\n{stdout}"
    );
    for n in 0..3 {
        assert!(
            stdout.contains(&format!("frame {n} committed and released")),
            "the compositor did not release frame {n}\nstdout:\n{stdout}"
        );
    }
    assert!(
        stdout.contains("pushed 3 frame(s) and tore down cleanly"),
        "the producer did not finish cleanly\nstdout:\n{stdout}"
    );

    // The compositor must have composited at least one surface. Without this
    // check the test would pass even if the pixels never reached it.
    let mut log_text = String::new();
    assert!(
        wait_until(Duration::from_secs(10), || {
            log_text = read_log(&log_path);
            log_text.contains("1 surface(s)")
        }),
        "compositor never reported compositing the surface.\nlog:\n{log_text}"
    );

    // Re-read after the wait so the assertion sees the latest log content.
    log_text = read_log(&log_path);
    assert!(
        log_text.contains("commit from client 1 surface 5: 64x64"),
        "the compositor did not log the surface geometry it received.\nlog:\n{log_text}"
    );
}

#[test]
fn compositor_refuses_to_start_on_a_taken_socket() {
    let rt = TempRuntime::new("busy");
    let compositor_bin = env!("CARGO_BIN_EXE_wayland_compositor");

    // First compositor owns the socket.
    let log = std::fs::File::create(rt.dir.join("a.log")).expect("log a");
    let first = Command::new(compositor_bin)
        .args(["--width", "64", "--height", "64"])
        .env("XDG_RUNTIME_DIR", &rt.dir)
        .stdout(Stdio::from(log))
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn first compositor");
    let _guard = ChildGuard(first);
    assert!(
        wait_until(Duration::from_secs(10), || rt.socket().exists()),
        "first compositor did not bind"
    );

    // A second one must fail rather than take the socket over.
    let out = Command::new(compositor_bin)
        .args(["--width", "64", "--height", "64"])
        .env("XDG_RUNTIME_DIR", &rt.dir)
        .output()
        .expect("spawn second compositor");
    assert!(
        !out.status.success(),
        "a second compositor must not start on an occupied socket"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("could not bind"),
        "the failure should name the problem, got: {stderr}"
    );
}
