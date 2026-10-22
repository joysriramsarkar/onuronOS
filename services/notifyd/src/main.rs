// services/notifyd/src/main.rs — Notification Broker
use nilnotify::{NotificationClient, NotificationItem};
#[cfg(unix)]
use std::io::{Read, Write};
use std::path::Path;

pub fn process_notification_message(store_path: &Path, raw: &str) -> Option<NotificationItem> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let parts: Vec<&str> = trimmed.splitn(3, '|').collect();
    let (app, title, body) = match parts.as_slice() {
        [a, t, b] => (a.trim(), t.trim(), b.trim()),
        [t, b] => ("system", t.trim(), b.trim()),
        [b] => ("system", "System Notification", b.trim()),
        _ => return None,
    };
    NotificationClient::post_to(store_path, app, title, body).ok()
}

fn main() {
    println!("[notifyd] Notification Broker starting...");
    let _ = nilsd::notify_ready("notifyd", Some("/run/nilos/notify.sock"));
    #[cfg(unix)]
    {
        let policy = nilsd::auth::load_default_policy();
        let store_path = NotificationClient::default_store_path();
        if let Ok(listener) = nilsd::first_listener_or_bind("/run/nilos/notify.sock") {
            println!("[notifyd] Notification Broker active on /run/nilos/notify.sock");
            for stream in listener.incoming() {
                if let Ok(mut s) = stream {
                    // C1: only explicitly authorized peers may post banners.
                    if !nilsd::auth::authorize_stream(&policy, "notifyd", &s) {
                        continue;
                    }
                    let mut buf = [0u8; 1024];
                    if let Ok(n) = s.read(&mut buf) {
                        let msg = String::from_utf8_lossy(&buf[..n]);
                        if let Some(item) = process_notification_message(&store_path, &msg) {
                            println!(
                                "[NOTIFY BANNER] [{}] {}: {}",
                                item.app, item.title, item.body
                            );
                            let _ = s.write_all(b"OK\n");
                        } else {
                            let _ = s.write_all(b"ERR empty\n");
                        }
                    }
                }
            }
        }
    }
    #[cfg(not(unix))]
    {
        println!(
            "[notifyd] Notification store ready at {}.",
            NotificationClient::default_store_path().display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_notification_message_formats() {
        let dir = std::env::temp_dir().join("onuron_notifyd_test");
        let _ = std::fs::remove_dir_all(&dir);
        let store = dir.join("notifications.json");

        let item1 =
            process_notification_message(&store, "nilupd|System Update|Slot B staged").unwrap();
        assert_eq!(item1.app, "nilupd");
        assert_eq!(item1.title, "System Update");
        assert_eq!(item1.body, "Slot B staged");

        let item2 = process_notification_message(&store, "Quick Alert").unwrap();
        assert_eq!(item2.app, "system");
        assert_eq!(item2.body, "Quick Alert");

        assert!(process_notification_message(&store, "   ").is_none());

        let all = NotificationClient::load_from(&store);
        assert_eq!(all.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
