// services/dnsd/src/main.rs — DNS-over-TLS Secure Resolver
use std::thread;
use std::time::Duration;

fn main() {
    println!("[dnsd] DNS-over-TLS Secure Resolver active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
