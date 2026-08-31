// runtime/nilrt/src/permbroker.rs — Centralized Permission Broker with 7-Day Auto-Revoke
use std::collections::HashMap;
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PermissionRecord {
    pub granted: bool,
    pub last_used_timestamp: u64,
}

pub struct PermissionBroker {
    db_path: String,
    records: HashMap<String, HashMap<String, PermissionRecord>>,
}

impl PermissionBroker {
    pub fn new(db_path: &str) -> Self {
        let mut broker = Self {
            db_path: db_path.to_string(),
            records: HashMap::new(),
        };
        broker.load();
        broker.auto_revoke_unused(7 * 86400);
        broker
    }

    fn now() -> u64 {
        SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs()
    }

    pub fn load(&mut self) {
        if let Ok(data) = fs::read_to_string(&self.db_path) {
            if let Ok(parsed) = serde_json::from_str(&data) {
                self.records = parsed;
            }
        }
    }

    pub fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(&self.records) {
            let _ = fs::write(&self.db_path, data);
        }
    }

    pub fn check_permission(&mut self, app_id: &str, perm: &str) -> bool {
        let app_perms = self.records.entry(app_id.to_string()).or_default();
        if let Some(record) = app_perms.get_mut(perm) {
            if record.granted {
                record.last_used_timestamp = Self::now();
                self.save();
                return true;
            }
        }
        false
    }

    pub fn grant_permission(&mut self, app_id: &str, perm: &str) {
        let app_perms = self.records.entry(app_id.to_string()).or_default();
        app_perms.insert(perm.to_string(), PermissionRecord {
            granted: true,
            last_used_timestamp: Self::now(),
        });
        self.save();
    }

    pub fn auto_revoke_unused(&mut self, max_age_secs: u64) {
        let current = Self::now();
        let mut revoked_count = 0;
        for (app, perms) in self.records.iter_mut() {
            for (perm, rec) in perms.iter_mut() {
                if rec.granted && (current - rec.last_used_timestamp > max_age_secs) {
                    println!("[permbroker] Auto-revoked inactive permission '{}' for app '{}'", perm, app);
                    rec.granted = false;
                    revoked_count += 1;
                }
            }
        }
        if revoked_count > 0 {
            self.save();
        }
    }
}
