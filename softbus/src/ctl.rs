// softbus/src/ctl.rs — Unix socket control bridge + QUIC peer handoff routing
#![cfg(unix)]

use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixListener as TokioUnixListener;

use crate::mdns::PeerMap;
use crate::quic::QuicPeerSession;
use quinn::Endpoint;

pub struct SoftBusControl {
    socket_path: String,
    peers: PeerMap,
    quic_endpoint: Arc<Endpoint>,
    device_id: String,
}

impl SoftBusControl {
    pub fn new(
        socket_path: &str,
        peers: PeerMap,
        quic_endpoint: Arc<Endpoint>,
        device_id: &str,
    ) -> Self {
        let _ = std::fs::remove_file(socket_path);
        Self {
            socket_path: socket_path.to_string(),
            peers,
            quic_endpoint,
            device_id: device_id.to_string(),
        }
    }

    pub async fn run(self: Arc<Self>) -> std::io::Result<()> {
        let listener = TokioUnixListener::bind(&self.socket_path)?;
        println!("[nilbus:ctl] Control socket on {}", self.socket_path);

        loop {
            let (mut stream, _) = listener.accept().await?;
            let ctrl = self.clone();

            tokio::spawn(async move {
                let mut buf = [0u8; 2048];
                if let Ok(n) = stream.read(&mut buf).await {
                    if n > 0 {
                        let cmd = String::from_utf8_lossy(&buf[..n]);
                        let response = ctrl.handle_command(cmd.trim()).await;
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                }
            });
        }
    }

    async fn handle_command(&self, cmd: &str) -> String {
        if cmd == "LIST_PEERS" {
            let map = self.peers.lock().unwrap();
            if map.is_empty() { return "EMPTY\n".to_string(); }
            let list: Vec<String> = map.values()
                .map(|p| format!("{}@{}:{}", p.device_id, p.addr, p.quic_port))
                .collect();
            return list.join(",") + "\n";
        }

        if cmd.starts_with("HANDOFF ") {
            let parts: Vec<&str> = cmd.splitn(4, ' ').collect();
            if parts.len() < 4 {
                return "ERR: usage: HANDOFF <peer> <app> <hex-payload>\n".to_string();
            }
            return self.do_handoff(parts[1], parts[2], parts[3]).await;
        }

        if cmd.starts_with("PING ") {
            let peer_id = cmd.trim_start_matches("PING ").trim();
            let map = self.peers.lock().unwrap();
            return if map.contains_key(peer_id) {
                format!("OK peer={peer_id} reachable\n")
            } else {
                format!("FAIL peer={peer_id} not found\n")
            };
        }

        format!("ERR: unknown command: {cmd}\n")
    }

    async fn do_handoff(&self, peer_id: &str, app_id: &str, hex_payload: &str) -> String {
        let payload = match hex::decode(hex_payload.trim()) {
            Ok(b) => b,
            Err(e) => return format!("ERR: invalid hex payload: {e}\n"),
        };

        let peer_addr: SocketAddr = {
            let map = self.peers.lock().unwrap();
            match map.get(peer_id) {
                Some(p) => SocketAddr::new(p.addr, p.quic_port),
                None => return format!("ERR: peer not found: {peer_id}\n"),
            }
        };

        match QuicPeerSession::connect(&self.quic_endpoint, peer_addr, peer_id).await {
            Ok(session) => {
                let mut msg = format!("HANDOFF {app_id}\n").into_bytes();
                msg.extend_from_slice(&payload);
                match session.send_packet(&msg).await {
                    Ok(_) => format!("OK HANDOFF→{peer_id}/{app_id} {} bytes\n", payload.len()),
                    Err(e) => format!("ERR: send failed: {e}\n"),
                }
            }
            Err(e) => format!("ERR: QUIC connect to {peer_id}: {e}\n"),
        }
    }
}

pub async fn run_quic_accept_loop(endpoint: Arc<Endpoint>) {
    println!("[nilbus:quic] Accept loop running...");
    while let Some(incoming) = endpoint.accept().await {
        tokio::spawn(async move {
            match incoming.await {
                Ok(conn) => {
                    let remote = conn.remote_address();
                    println!("[nilbus:quic] Incoming connection from {remote}");
                    loop {
                        match conn.accept_uni().await {
                            Ok(mut recv) => {
                                let mut len_buf = [0u8; 4];
                                if recv.read_exact(&mut len_buf).await.is_err() { break; }
                                let len = u32::from_le_bytes(len_buf) as usize;
                                if len > 16 * 1024 * 1024 { break; }
                                let mut buf = vec![0u8; len];
                                if recv.read_exact(&mut buf).await.is_err() { break; }
                                if let Ok(text) = std::str::from_utf8(&buf) {
                                    println!("[nilbus:quic] ← {remote}: {}",
                                        text.lines().next().unwrap_or("(binary)"));
                                }
                            }
                            Err(e) => {
                                println!("[nilbus:quic] connection closed from {remote}: {e}");
                                break;
                            }
                        }
                    }
                }
                Err(e) => println!("[nilbus:quic] incoming accept error: {e}"),
            }
        });
    }
}
