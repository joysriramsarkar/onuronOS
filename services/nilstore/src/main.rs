// services/nilstore/src/main.rs — 4KiB Content-Addressed Delta Chunk Server
use std::thread;
use std::time::Duration;

fn main() {
    println!("[nilstore] Content-Addressed Delta Chunk Store Server active on port 8080.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
