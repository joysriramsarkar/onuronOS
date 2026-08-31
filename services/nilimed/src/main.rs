// services/nilimed/src/main.rs — Bengali IME Daemon
use std::os::unix::net::UnixListener;
use std::io::{Read, Write};

mod engine;
use engine::PhoneticEngine;

fn main() {
    println!("[nilimed] Bengali Phonetic IME Daemon started.");
    let engine = PhoneticEngine::new();
    let listener = nilsd::first_listener_or_bind("/run/nilos/ime.sock").unwrap();

    for stream in listener.incoming() {
        if let Ok(mut s) = stream {
            let mut buf = [0u8; 256];
            if let Ok(n) = s.read(&mut buf) {
                let input = String::from_utf8_lossy(&buf[..n]).trim().to_string();
                let output = engine.transliterate(&input);
                let _ = s.write_all(output.as_bytes());
            }
        }
    }
}
