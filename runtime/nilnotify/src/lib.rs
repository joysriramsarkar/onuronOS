// runtime/nilnotify/src/lib.rs — Onuron OS Notification Client & Persistent Store
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

pub const NOTIFICATIONS_FILE: &str = "/data/notifications/notifications.json";
pub const MAX_NOTIFICATIONS: usize = 64;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NotificationItem {
    pub id: u64,
    pub app: String,
    pub title: String,
    pub body: String,
    pub timestamp: u64,
    pub read: bool,
}

pub struct Client;

impl Client {
    pub fn is_ready() -> bool {
        true
    }
}

pub struct NotificationClient;

impl NotificationClient {
    pub fn default_store_path() -> PathBuf {
        let p = Path::new(NOTIFICATIONS_FILE);
        if p.parent().map(|d| d.exists()).unwrap_or(false) || Path::new("/data").exists() {
            p.to_path_buf()
        } else {
            std::env::temp_dir().join("onuron_notifications.json")
        }
    }

    pub fn load_from(path: &Path) -> Vec<NotificationItem> {
        if !path.exists() {
            return Vec::new();
        }
        match fs::read_to_string(path) {
            Ok(text) => serde_json::from_str::<Vec<NotificationItem>>(&text).unwrap_or_default(),
            Err(_) => Vec::new(),
        }
    }

    pub fn save_atomic(path: &Path, items: &[NotificationItem]) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        let tmp = path.with_extension("tmp");
        let bytes = serde_json::to_vec_pretty(items).map_err(|e| e.to_string())?;
        {
            let mut file = File::create(&tmp).map_err(|e| e.to_string())?;
            file.write_all(&bytes).map_err(|e| e.to_string())?;
            file.sync_all().map_err(|e| e.to_string())?;
        }
        fs::rename(&tmp, path).map_err(|e| e.to_string())
    }

    pub fn post_to(
        path: &Path,
        app: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<NotificationItem, String> {
        let mut items = Self::load_from(path);
        let next_id = items.iter().map(|n| n.id).max().unwrap_or(0) + 1;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let item = NotificationItem {
            id: next_id,
            app: app.into(),
            title: title.into(),
            body: body.into(),
            timestamp: now,
            read: false,
        };
        items.insert(0, item.clone());
        if items.len() > MAX_NOTIFICATIONS {
            items.truncate(MAX_NOTIFICATIONS);
        }
        Self::save_atomic(path, &items)?;
        Ok(item)
    }

    pub fn dismiss_from(path: &Path, id: u64) -> Result<bool, String> {
        let mut items = Self::load_from(path);
        let prev_len = items.len();
        items.retain(|n| n.id != id);
        if items.len() < prev_len {
            Self::save_atomic(path, &items)?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn clear_all(path: &Path) -> Result<(), String> {
        Self::save_atomic(path, &[])
    }

    pub fn post(
        app: impl Into<String>,
        title: impl Into<String>,
        body: impl Into<String>,
    ) -> Result<NotificationItem, String> {
        let app_s = app.into();
        let title_s = title.into();
        let body_s = body.into();

        #[cfg(unix)]
        {
            use std::os::unix::net::UnixStream;
            if let Ok(mut stream) = UnixStream::connect("/run/nilos/notify.sock") {
                let wire = format!("{}|{}|{}\n", app_s, title_s, body_s);
                let _ = stream.write_all(wire.as_bytes());
            }
        }

        Self::post_to(&Self::default_store_path(), app_s, title_s, body_s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_post_list_dismiss_and_clear_notifications() {
        let dir = std::env::temp_dir().join("onuron_nilnotify_test");
        let _ = fs::remove_dir_all(&dir);
        let file = dir.join("notifications.json");

        assert!(NotificationClient::load_from(&file).is_empty());

        let n1 = NotificationClient::post_to(
            &file,
            "nilpkg",
            "Package Installed",
            "org.onuron.hello v1.0.0",
        )
        .unwrap();
        let n2 = NotificationClient::post_to(&file, "powerd", "Battery Notice", "Battery at 85%")
            .unwrap();
        assert_eq!(n1.id, 1);
        assert_eq!(n2.id, 2);

        let list = NotificationClient::load_from(&file);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].title, "Battery Notice");

        assert!(NotificationClient::dismiss_from(&file, 1).unwrap());
        assert_eq!(NotificationClient::load_from(&file).len(), 1);

        NotificationClient::clear_all(&file).unwrap();
        assert!(NotificationClient::load_from(&file).is_empty());

        let _ = fs::remove_dir_all(&dir);
    }
}
