// services/niltrace/src/main.rs — ftrace to Chrome JSON Profiler
use std::thread;
use std::time::Duration;

fn main() {
    println!("[niltrace] ftrace to Chrome JSON Profiler active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}
