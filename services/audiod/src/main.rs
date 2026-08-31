// services/audiod/src/main.rs — PipeWire Audio Focus Router
use std::thread;
use std::time::Duration;

fn main() {
    println!("[audiod] PipeWire Audio Focus Router active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
