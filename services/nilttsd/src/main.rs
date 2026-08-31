// services/nilttsd/src/main.rs — Bangla & Multilingual TTS Voice Synthesizer
use std::thread;
use std::time::Duration;

fn main() {
    println!("[nilttsd] Bangla & Multilingual TTS Voice Synthesizer active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
