// runtime/nilrt/src/bin/nilfastbootd.rs — USB Fastboot Protocol Daemon
use std::thread;
use std::time::Duration;

fn main() {
    println!("[nilfastbootd] Fastboot USB Gadget Interface listening...");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
