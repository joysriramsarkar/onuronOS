// services/clipd/src/main.rs — System Clipboard Manager
use std::thread;
use std::time::Duration;

fn main() {
    println!("[clipd] System Clipboard Manager active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
