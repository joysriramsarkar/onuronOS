// runtime/nilui-gpu/src/bin/bootsplash.rs — Animated Bootsplash
use std::thread;
use std::time::Duration;

fn main() {
    println!("=========================================================");
    println!("             [NilOS Glowing Bootsplash]                  ");
    println!("=========================================================");
    for i in (0..=100).step_by(25) {
        println!("[bootsplash] Loading OS Core... {}%", i);
        thread::sleep(Duration::from_millis(50));
    }
}
