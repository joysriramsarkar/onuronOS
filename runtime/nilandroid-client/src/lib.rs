// runtime/nilandroid-client/src/lib.rs — Intent invocation bridge client
#[cfg(unix)]
use std::io::Write;
#[cfg(unix)]
use std::os::unix::net::UnixStream;

pub struct AndroidBridge;

impl AndroidBridge {
    pub fn start_activity(package: &str, activity: &str) -> Result<(), String> {
        #[cfg(unix)]
        if let Ok(mut stream) = UnixStream::connect("/run/nilos/android.sock") {
            let payload = format!(
                "{{\"action\":\"START_ACTIVITY\",\"package\":\"{}\",\"activity\":\"{}\"}}\n",
                package, activity
            );
            stream
                .write_all(payload.as_bytes())
                .map_err(|e| e.to_string())?;
            return Ok(());
        }
        #[cfg(not(unix))]
        println!(
            "[android-client] Host bridge Intent START: {}/{}",
            package, activity
        );
        Ok(())
    }

    pub fn ping() -> Result<bool, String> {
        #[cfg(unix)]
        if let Ok(mut stream) = UnixStream::connect("/run/nilos/android.sock") {
            stream
                .write_all(b"{\"action\":\"PING\"}\n")
                .map_err(|e| e.to_string())?;
            return Ok(true);
        }
        Ok(false)
    }
}
