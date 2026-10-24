// runtime/nilui-gpu/src/bin/wayland_surface_producer.rs — a standalone Wayland client
//
// Pushes animated frames at the Onuron compositor through the shared
// `nilui_gpu::wayland::client` library. It exists so the passthrough can be
// exercised as two real processes over a real socket, and it is the smallest
// example of what a surface producer does.
//
// This is the shape an Android graphics bridge takes: something outside the
// compositor has pixels in CPU memory and needs them on screen. The bridge's
// own job is getting the pixels into a `frame_mut()` slice; everything after
// that is this code.
//
// Usage:
//   wayland_surface_producer [--socket PATH] [--width N] [--height N] [--frames N]
//
// Environment:
//   XDG_RUNTIME_DIR   where `wayland-0` is looked for when --socket is not given

use nilui_gpu::wayland::client::SurfaceProducer;

struct Args {
    socket: std::path::PathBuf,
    width: u32,
    height: u32,
    frames: u32,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        socket: SurfaceProducer::default_socket_path(),
        width: 256,
        height: 256,
        frames: 3,
    };
    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "--socket" => {
                args.socket = std::path::PathBuf::from(it.next().ok_or("--socket needs a path")?)
            }
            "--width" => {
                args.width = it
                    .next()
                    .ok_or("--width needs a number")?
                    .parse()
                    .map_err(|e| format!("--width: {e}"))?
            }
            "--height" => {
                args.height = it
                    .next()
                    .ok_or("--height needs a number")?
                    .parse()
                    .map_err(|e| format!("--height: {e}"))?
            }
            "--frames" => {
                args.frames = it
                    .next()
                    .ok_or("--frames needs a number")?
                    .parse()
                    .map_err(|e| format!("--frames: {e}"))?
            }
            "--help" | "-h" => {
                println!(
                    "wayland_surface_producer [--socket PATH] [--width N] [--height N] [--frames N]"
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(args)
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("wayland_surface_producer: {e}");
            std::process::exit(1);
        }
    };

    println!(
        "[ producer ] connecting to {} ({}x{}, {} frame(s))",
        args.socket.display(),
        args.width,
        args.height,
        args.frames
    );

    let mut p = match SurfaceProducer::connect(&args.socket, args.width, args.height) {
        Ok(p) => p,
        Err(e) => {
            eprintln!(
                "[  FAIL  ] {e}\n\
                 Is wayland_compositor running with the same XDG_RUNTIME_DIR?"
            );
            std::process::exit(1);
        }
    };
    println!(
        "[   OK   ] connected and surface created ({}x{})",
        p.width(),
        p.height()
    );

    // Push a distinct pattern per frame so frames can be told apart.
    let (w, h) = (p.width(), p.height());
    for n in 0..args.frames {
        {
            let frame = p.frame_mut();
            for y in 0..h {
                for x in 0..w {
                    let r = x * 255 / w.max(1);
                    let g = y * 255 / h.max(1);
                    let b = (n * 40) & 0xFF;
                    frame[(y * w + x) as usize] = 0xFF00_0000 | (r << 16) | (g << 8) | b;
                }
            }
        }

        match p.commit() {
            Ok(()) => println!("[   OK   ] frame {n} committed and released"),
            Err(e) => {
                eprintln!("[  FAIL  ] frame {n}: {e}");
                std::process::exit(1);
            }
        }
    }

    if let Err(e) = p.close() {
        eprintln!("[  WARN  ] teardown: {e}");
    }
    println!(
        "[   OK   ] pushed {} frame(s) and tore down cleanly",
        args.frames
    );
}
