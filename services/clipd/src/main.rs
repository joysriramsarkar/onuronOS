// services/clipd/src/main.rs — System Clipboard Manager with Bounded Size & TTL Expiry
use std::thread;
use std::time::{Duration, Instant};

pub const MAX_CLIPBOARD_BYTES: usize = 64 * 1024;
pub const DEFAULT_TTL_SECS: u64 = 300;

#[derive(Debug, Clone)]
pub struct ClipboardStore {
    content: Option<(String, Instant)>,
    ttl: Duration,
}

impl Default for ClipboardStore {
    fn default() -> Self {
        Self::new(Duration::from_secs(DEFAULT_TTL_SECS))
    }
}

impl ClipboardStore {
    pub fn new(ttl: Duration) -> Self {
        Self { content: None, ttl }
    }

    pub fn set(&mut self, text: &str) -> Result<(), String> {
        if text.len() > MAX_CLIPBOARD_BYTES {
            return Err(format!(
                "clipboard payload exceeds maximum size ({} > {})",
                text.len(),
                MAX_CLIPBOARD_BYTES
            ));
        }
        self.content = Some((text.to_string(), Instant::now()));
        Ok(())
    }

    pub fn get(&mut self) -> Option<String> {
        if let Some((ref val, ts)) = self.content {
            if ts.elapsed() <= self.ttl {
                return Some(val.clone());
            }
        }
        self.content = None;
        None
    }

    pub fn clear(&mut self) {
        self.content = None;
    }

    pub fn handle_command(&mut self, raw: &str) -> String {
        let trimmed = raw.trim();
        if trimmed == "GET" {
            match self.get() {
                Some(v) => format!("VALUE {}\n", v),
                None => "EMPTY\n".to_string(),
            }
        } else if trimmed == "CLEAR" {
            self.clear();
            "OK\n".to_string()
        } else if let Some(rest) = trimmed.strip_prefix("SET ") {
            match self.set(rest) {
                Ok(()) => "OK\n".to_string(),
                Err(e) => format!("ERR {}\n", e),
            }
        } else {
            "ERR unknown command\n".to_string()
        }
    }
}

fn main() {
    println!("[clipd] System Clipboard Manager starting...");
    let _ = nilsd::notify_ready("clipd", Some("/run/nilos/clip.sock"));
    let mut store = ClipboardStore::default();

    #[cfg(unix)]
    {
        use std::io::{Read, Write};
        let policy = nilsd::auth::load_default_policy();
        if let Ok(listener) = nilsd::first_listener_or_bind("/run/nilos/clip.sock") {
            println!("[clipd] Active on /run/nilos/clip.sock");
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    if !nilsd::auth::authorize_stream(&policy, "clipd", &s) {
                        continue;
                    }
                    let mut buf = [0u8; 4096];
                    if let Ok(n) = s.read(&mut buf) {
                        let req = String::from_utf8_lossy(&buf[..n]);
                        let resp = store.handle_command(&req);
                        let _ = s.write_all(resp.as_bytes());
                    }
                }
            }
            return;
        }
    }

    let _ = &mut store;
    println!("[clipd] System Clipboard Manager active.");
    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clipboard_set_get_clear_and_size_limit() {
        let mut store = ClipboardStore::default();
        assert_eq!(store.handle_command("GET"), "EMPTY\n");
        assert_eq!(store.handle_command("SET OnuronText"), "OK\n");
        assert_eq!(store.handle_command("GET"), "VALUE OnuronText\n");
        assert_eq!(store.handle_command("CLEAR"), "OK\n");
        assert_eq!(store.handle_command("GET"), "EMPTY\n");

        let huge = "x".repeat(MAX_CLIPBOARD_BYTES + 10);
        assert!(store.set(&huge).is_err());
    }

    #[test]
    fn test_clipboard_ttl_expiry() {
        let mut store = ClipboardStore::new(Duration::from_millis(1));
        store.set("secret").unwrap();
        thread::sleep(Duration::from_millis(10));
        assert!(store.get().is_none());
    }
}
