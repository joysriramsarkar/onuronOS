// services/nilimed/src/main.rs — Bengali IME Daemon
#[cfg(unix)]
use std::io::{Read, Write};

mod engine;
use engine::PhoneticEngine;

fn main() {
    println!("[nilimed] Bengali Phonetic IME Daemon started.");
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
                let mut buf = [0u8; 256];
                if let Ok(n) = s.read(&mut buf) {
                    let input = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                    let output = engine.transliterate(&input);
                    let _ = s.write_all(output.as_bytes());
                }
            }
        }
    }

    #[cfg(not(unix))]
    {
        println!("[nilimed] Simulated Bengali IME Engine active: 'ami' -> '{}'", engine.transliterate("ami"));
    }
}

