// services/nilperf/src/main.rs — Launch Latency & Frame Pacing Profiler
use std::thread;
use std::time::Duration;

fn main() {
    println!("[nilperf] Launch Latency & Frame Pacing Profiler active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
