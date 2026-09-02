// softbus/src/main.rs — NilOS Distributed SoftBus Daemon (nilbus)
//
// On Linux/Unix: full async daemon with mDNS-SD + QUIC/TLS 1.3.
// On Windows dev-hosts: stub that prints a message and exits cleanly
//   (the daemon is a Linux-only binary in production).

#[cfg(unix)]
mod ctl;
#[cfg(unix)]
mod mdns;
#[cfg(unix)]
mod quic;

// ── Linux / Unix implementation ───────────────────────────────────────────────
#[cfg(unix)]
#[tokio::main]
async fn main() {
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use std::sync::{Arc, Mutex};

    println!("=========================================================");
    println!("        NilOS Distributed SoftBus Daemon (nilbus)        ");
    println!("=========================================================");

    let device_id = get_or_create_device_id();
    println!("[nilbus] Device ID: {device_id}");

    // ── 1. Generate self-signed TLS certificate ───────────────────────────────
    let (cert, key) = match quic::generate_self_signed_cert(&device_id) {
        Ok(pair) => pair,
        Err(e) => { eprintln!("[nilbus] FATAL: cert generation failed: {e}"); std::process::exit(1); }
    };

    // ── 2. Bind QUIC server endpoint ──────────────────────────────────────────
    let bind_addr: SocketAddr = format!("0.0.0.0:{QUIC_PORT}").parse().unwrap();
    let endpoint = match quic::make_server_endpoint(bind_addr, cert, key) {
        Ok(ep) => Arc::new(ep),
        Err(e) => { eprintln!("[nilbus] FATAL: QUIC bind failed: {e}"); std::process::exit(1); }
    };

    // ── 3. Start mDNS-SD peer discovery ──────────────────────────────────────
    let peers: mdns::PeerMap = Arc::new(Mutex::new(HashMap::new()));

    if let Err(e) = mdns::start_discovery(&device_id, QUIC_PORT, peers.clone()).await {
        eprintln!("[nilbus] WARNING: mDNS discovery failed: {e}");
    }

    // ── 4. Start control socket ───────────────────────────────────────────────
    let ctrl = Arc::new(ctl::SoftBusControl::new(
        CONTROL_SOCK,
        peers.clone(),
        endpoint.clone(),
        &device_id,
    ));

    let ctrl_task = {
        let ctrl = ctrl.clone();
        tokio::spawn(async move {
            if let Err(e) = ctrl.run().await {
                eprintln!("[nilbus:ctl] Control socket error: {e}");
            }
        })
    };

    // ── 5. QUIC accept loop ───────────────────────────────────────────────────
    let accept_task = {
        let ep = endpoint.clone();
        tokio::spawn(async move { ctl::run_quic_accept_loop(ep).await })
    };

    println!("[nilbus] All subsystems active. Discovering peers via mDNS-SD...");

    tokio::select! {
        _ = ctrl_task   => eprintln!("[nilbus] Control socket task exited"),
        _ = accept_task => eprintln!("[nilbus] QUIC accept task exited"),
    }
}

// ── Windows dev-host stub ─────────────────────────────────────────────────────
#[cfg(not(unix))]
#[tokio::main]
async fn main() {
    println!("[nilbus] SoftBus daemon is a Linux-only binary.");
    println!("[nilbus] Run this inside QEMU or on a Linux target.");
}

// ── Shared helpers ────────────────────────────────────────────────────────────

#[cfg(unix)]
const QUIC_PORT: u16 = 7979;
#[cfg(unix)]
const CONTROL_SOCK: &str = "/run/nilos/bus.sock";
#[cfg(unix)]
const DEVICE_ID_FILE: &str = "/data/nilos/device_id";

#[cfg(unix)]
fn get_or_create_device_id() -> String {
    if let Ok(id) = std::fs::read_to_string(DEVICE_ID_FILE) {
        let id = id.trim().to_string();
        if !id.is_empty() { return id; }
    }
    let hostname = std::fs::read_to_string("/etc/hostname")
        .unwrap_or_else(|_| "nilos-device".to_string());
    let id = format!("{}-bus", hostname.trim());
    let _ = std::fs::create_dir_all("/data/nilos");
    let _ = std::fs::write(DEVICE_ID_FILE, &id);
    id
}
