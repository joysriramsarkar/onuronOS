// services/alarmd/src/main.rs — RTC Wake Alarm Scheduler
use std::thread;
use std::time::Duration;

fn main() {
    println!("[alarmd] RTC Wake Alarm Scheduler active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
