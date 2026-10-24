// runtime/nilui-gpu/src/bin/wayland_compositor.rs — run the Onuron Wayland compositor
//
// Accepts Wayland clients, composites their committed surfaces, and presents the
// result through the DRM/KMS path (falling back to an offscreen buffer when no
// display is reachable, exactly as `KmsPresenter` already does).
//
// Usage:
//   wayland_compositor [--socket PATH] [--width N] [--height N] [--layout fullscreen|tiled] [--once]
//
// Environment:
//   XDG_RUNTIME_DIR   where `wayland-0` is created when --socket is not given
//
// Exit codes:
//   0  ran to completion
//   2  ran, but no panel was reached (offscreen backend); frames were still
//      composited, they just had nowhere to go

use nilui_gpu::wayland::compose::{composite, Layout};
use nilui_gpu::wayland::server::{default_socket_path, OutputInfo, WaylandServer};
use nilui_gpu::KmsPresenter;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// Exit code when frames were composited but no physical panel was reached.
const EXIT_NO_PANEL: i32 = 2;

struct Args {
    socket: std::path::PathBuf,
    width: u32,
    height: u32,
    layout: Layout,
    /// Composite and present once, then exit. Useful for CI and smoke tests.
    once: bool,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        socket: default_socket_path(),
        width: 1080,
        height: 2400,
        layout: Layout::Fullscreen,
        once: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--socket" => {
                let v = it.next().ok_or("--socket needs a path")?;
                args.socket = std::path::PathBuf::from(v);
            }
            "--width" => {
                args.width = it
                    .next()
                    .ok_or("--width needs a number")?
                    .parse()
                    .map_err(|e| format!("--width: {e}"))?;
            }
            "--height" => {
                args.height = it
                    .next()
                    .ok_or("--height needs a number")?
                    .parse()
                    .map_err(|e| format!("--height: {e}"))?;
            }
            "--layout" => {
                let v = it.next().ok_or("--layout needs a value")?;
                args.layout = match v.as_str() {
                    "fullscreen" => Layout::Fullscreen,
                    "tiled" => Layout::Tiled,
                    other => return Err(format!("unknown layout {other}")),
                };
            }
            "--once" => args.once = true,
            "--help" | "-h" => {
                println!(
                    "wayland_compositor [--socket PATH] [--width N] [--height N] \
                     [--layout fullscreen|tiled] [--once]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

/// Wait until the compositor signals a commit, or the stop flag is set.
///
/// Uses `poll` with a timeout rather than a blocking read, so a shutdown
/// request is honoured even when no client ever connects.
fn wait_for_wake(
    wake: &nilui_gpu::wayland::server::WakeHandle,
    stop: &AtomicBool,
) -> std::io::Result<()> {
    // A private eventfd is used purely as a timeout source so the wait cannot
    // block indefinitely. `poll` on it always times out.
    let mut fds = [
        libc::pollfd {
            fd: wake.raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
        libc::pollfd {
            fd: timer_fd(),
            events: libc::POLLIN,
            revents: 0,
        },
    ];
    // SAFETY: `fds` is a valid array of pollfd with `len` matching.
    let rc = unsafe { libc::poll(fds.as_mut_ptr(), fds.len() as libc::nfds_t, 100) };
    if rc < 0 {
        let e = std::io::Error::last_os_error();
        if e.kind() == std::io::ErrorKind::Interrupted {
            return Ok(());
        }
        return Err(e);
    }
    if fds[0].revents & libc::POLLIN != 0 {
        // Drain the counter so the next poll blocks again.
        wake.wait()?;
    }
    let _ = stop;
    Ok(())
}

/// A descriptor that never becomes readable, used only as a poll timeout.
fn timer_fd() -> libc::c_int {
    // eventfd with no writers: poll waits on it until the timeout expires.
    static TIMER: std::sync::OnceLock<libc::c_int> = std::sync::OnceLock::new();
    *TIMER.get_or_init(|| {
        // SAFETY: eventfd with valid flags.
        let fd = unsafe { libc::eventfd(0, libc::EFD_CLOEXEC) };
        assert!(fd >= 0, "eventfd for poll timeout");
        fd
    })
}

/// Set by the signal handler, which is why it is a static: a handler cannot
/// capture, and only lock-free atomics are safe to touch from one.
static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn on_signal(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::SeqCst);
}

/// Arrange for SIGINT and SIGTERM to request a clean shutdown.
fn install_signal_handler(stop: Arc<AtomicBool>) {
    // SAFETY: `on_signal` only stores to a static atomic, which is
    // async-signal-safe. `signal` is given valid handler pointers.
    unsafe {
        libc::signal(libc::SIGINT, on_signal as *const () as libc::sighandler_t);
        libc::signal(libc::SIGTERM, on_signal as *const () as libc::sighandler_t);
    }
    // Mirror the static into the shared flag so both routes work.
    let _ = std::thread::spawn(move || loop {
        if SHUTDOWN.load(Ordering::SeqCst) {
            stop.store(true, Ordering::SeqCst);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    });
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("wayland_compositor: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "[ wayland_compositor ] Onuron Wayland compositor {}x{}",
        args.width, args.height
    );

    let output = OutputInfo {
        width: args.width,
        height: args.height,
        refresh_mhz: 60_000,
    };

    let server = match WaylandServer::bind(&args.socket, output) {
        Ok(s) => Arc::new(s),
        Err(e) => {
            // A clear failure beats a compositor that silently does not listen.
            eprintln!("[  FAIL  ] could not bind {}: {e}", args.socket.display());
            std::process::exit(1);
        }
    };
    println!("[   OK   ] listening on {}", server.path().display());
    for g in server.globals() {
        println!(
            "[   OK   ] global {} v{} ({})",
            g.interface, g.version, g.name
        );
    }

    let state = server.state();

    // Presenter: real card if one is present, offscreen otherwise. The backend
    // is reported so the operator knows which happened.
    let mut presenter = KmsPresenter::with_resolution(args.width, args.height, 60);
    println!(
        "[  INFO  ] display backend: {} ({})",
        presenter.drm_device.backend.label(),
        presenter
            .drm_device
            .backend
            .reason()
            .unwrap_or("card opened successfully")
    );

    // Accept clients on a background thread.
    let accept_server = Arc::clone(&server);
    let accept_thread = std::thread::spawn(move || {
        let _ = accept_server.run();
    });

    // SIGINT/SIGTERM set a flag the loop polls. Storing to a lock-free atomic
    // is one of the few things that is async-signal-safe, so the handler needs
    // no locks and cannot deadlock. Reading stdin instead would be wrong: a
    // backgrounded process has stdin at EOF and would exit immediately.
    let stop = Arc::new(AtomicBool::new(false));
    install_signal_handler(Arc::clone(&stop));

    let mut last_generation = u64::MAX;
    let mut presented = 0u64;
    let mut reached_panel = false;
    let wake = server.wake_handle();

    loop {
        if stop.load(Ordering::SeqCst) {
            break;
        }

        // Block until a client commits, instead of sleeping and re-checking.
        // Polling would drop a frame whose surface is destroyed before the next
        // poll, which is exactly what a short-lived client does.
        // The stop flag is checked on a timeout so a signal is still noticed.
        if let Err(e) = wait_for_wake(&wake, &stop) {
            eprintln!("[  WARN  ] poll failed: {e}");
        }
        if stop.load(Ordering::SeqCst) {
            break;
        }

        let (generation, frames) = {
            let mut st = state.lock().unwrap_or_else(|e| e.into_inner());
            let gen = st.generation();
            let destroyed = st.take_destroyed();
            if !destroyed.is_empty() {
                for k in destroyed {
                    println!("[  INFO  ] surface {} destroyed", k.object);
                }
            }
            let frames = st.take_frames();
            (gen, frames)
        };

        for f in &frames {
            println!(
                "[  INFO  ] commit from client {} surface {}: {}x{}",
                f.key.client, f.key.object, f.width, f.height
            );
        }

        // Only recomposite when something actually changed, so an idle
        // compositor does not burn CPU redrawing an identical frame.
        if generation != last_generation {
            last_generation = generation;

            presenter.acquire_next_slot();
            let back = presenter.back_buffer_mut();
            back.clear(0xFF000000);
            let drawn = {
                let st = state.lock().unwrap_or_else(|e| e.into_inner());
                composite(back, &st, args.layout)
            };

            let stats = presenter.present_frame();
            presented += 1;
            if stats.page_flip_success {
                reached_panel = true;
            }
            println!(
                "[  INFO  ] frame {}: {} surface(s) | {:.2}ms | {}",
                stats.frame_index, drawn, stats.frame_time_ms, stats.backend
            );
        }

        if args.once && presented > 0 {
            break;
        }
    }

    server.shutdown();
    let _ = accept_thread.join();
    println!("[   OK   ] presented {presented} frame(s)");

    if !reached_panel {
        // State this plainly: frames were composited, but nothing lit a display.
        println!("[  WARN  ] no physical panel was reached; frames stayed offscreen");
    }

    // Drop the server before exiting so its socket is unlinked. Calling
    // `process::exit` first would skip the destructor and leave a stale socket
    // that the next start has to work around.
    drop(server);
    std::process::exit(if reached_panel { 0 } else { EXIT_NO_PANEL });
}
