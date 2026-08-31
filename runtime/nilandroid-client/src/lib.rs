// runtime/nilandroid-client/src/lib.rs — Intent invocation bridge client
use std::io::{Read, Write};
use std::os::unix::net::UnixStream;

pub struct AndroidBridge;

impl AndroidBridge {
    pub fn start_activity(action: &str, uri: &str) -> Result<(), String> {
        if let Ok(mut stream) = UnixStream::connect("/run/nilos/android.sock") {
            stream.write_all(format!("START {} {}\n", action, uri).as_bytes())
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        println!("[android-client] Simulated Intent START: {} {}", action, uri);
        Ok(())
    }
}
