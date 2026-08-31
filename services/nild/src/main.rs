// services/nild/src/main.rs — Power, Telephony (oFono), Wi-Fi (iwd) & HAL Daemon
use std::thread;
use std::time::Duration;

mod hal_dispatch;
use hal_dispatch::HalDispatcher;

fn main() {
    println!("=========================================================");
    println!("           NilOS System Daemon (nild)                    ");
    println!("=========================================================");

    let _dispatcher = HalDispatcher::init();
    println!("[nild] iwd Wi-Fi & oFono Telephony sub-managers initialized.");
    println!("[nild] Power governance profile: BALANCED (120Hz dynamic refresh)");

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
