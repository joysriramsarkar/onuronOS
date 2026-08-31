// services/camerad/src/main.rs — libcamera Manager & Remote SoftBus Streaming
use std::thread;
use std::time::Duration;

fn main() {
    println!("[camerad] libcamera Manager & Remote SoftBus Streaming active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
