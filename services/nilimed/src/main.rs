// services/nilimed/src/main.rs — Bengali IME Daemon
// Communicates via nilprotocol framed binary IPC over /run/nilos/ime.sock with raw text fallback

#[cfg(unix)]
use std::io::{Read, Write};

mod engine;
pub use engine::PhoneticEngine;

use nilprotocol::{
    Frame, ImeTransliteratePayload, ImeTransliterateResultPayload, MessageType,
};

pub fn handle_ipc_request(engine: &PhoneticEngine, frame: &Frame) -> Frame {
    let msg_type = MessageType::from(frame.message_type);
    match msg_type {
        MessageType::Ping => Frame::new(MessageType::Pong, frame.request_id, b"pong".to_vec()),
        MessageType::ImeTransliterate => {
            let input = if let Ok(req) = frame.parse_json::<ImeTransliteratePayload>() {
                req.text
            } else {
                String::from_utf8_lossy(&frame.payload).trim().to_string()
            };
            let output = engine.transliterate(&input);
            let payload = ImeTransliterateResultPayload {
                transliterated: output,
            };
            Frame::with_json(
                MessageType::ImeTransliterateResult,
                frame.request_id,
                &payload,
            )
            .unwrap_or_else(|_| {
                Frame::new(
                    MessageType::ErrorResponse,
                    frame.request_id,
                    b"encode error".to_vec(),
                )
            })
        }
        _ => Frame::new(
            MessageType::ErrorResponse,
            frame.request_id,
            b"unsupported msg".to_vec(),
        ),
    }
}

fn main() {
    println!("\x1b[1;36m[nilimed]\x1b[0m Bengali Phonetic IME Daemon started.");
    let engine = PhoneticEngine::new();

    #[cfg(unix)]
    if let Ok(listener) = nilsd::first_listener_or_bind("/run/nilos/ime.sock") {
        let policy = nilsd::auth::load_default_policy();
        for stream in listener.incoming() {
            if let Ok(mut s) = stream {
                // C1: unauthorized clients must not drive the IME.
                if !nilsd::auth::authorize_stream(&policy, "nilimed", &s) {
                    continue;
                }
                let mut buf = [0u8; 512];
                if let Ok(n) = s.read(&mut buf) {
                    if n >= 4 && &buf[..4] == &nilprotocol::PROTOCOL_MAGIC {
                        let mut cursor = std::io::Cursor::new(&buf[..n]);
                        if let Ok(frame) = Frame::read_from(&mut cursor) {
                            let resp = handle_ipc_request(&engine, &frame);
                            let _ = resp.write_to(&mut s);
                            continue;
                        }
                    }
                    let input = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    let output = engine.transliterate(&input);
                    let _ = s.write_all(output.as_bytes());
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        println!(
            "[nilimed] Simulated Bengali IME Engine active: 'ami' -> '{}'",
            engine.transliterate("ami")
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ime_ipc_transliteration() {
        let engine = PhoneticEngine::new();
        let req = Frame::with_json(
            MessageType::ImeTransliterate,
            12,
            &ImeTransliteratePayload {
                text: "ami".to_string(),
            },
        )
        .unwrap();

        let resp = handle_ipc_request(&engine, &req);
        assert_eq!(
            resp.message_type,
            u16::from(MessageType::ImeTransliterateResult)
        );
        let res = resp.parse_json::<ImeTransliterateResultPayload>().unwrap();
        assert_eq!(res.transliterated, "আমি");
    }

    #[test]
    fn test_ime_ipc_ping() {
        let engine = PhoneticEngine::new();
        let ping = Frame::new(MessageType::Ping, 10, vec![]);
        let pong = handle_ipc_request(&engine, &ping);
        assert_eq!(pong.message_type, u16::from(MessageType::Pong));
        assert_eq!(pong.payload, b"pong");
    }
}
