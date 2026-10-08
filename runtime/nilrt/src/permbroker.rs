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

    /// Never panic on a clock before the epoch (unset RTC on a fresh device).
    fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn load(&mut self) {
        match fs::read_to_string(&self.db_path) {
            Ok(data) => match serde_json::from_str(&data) {
                Ok(parsed) => self.records = parsed,
                Err(e) => {
                    // A corrupt database must not be overwritten in place:
                    // doing so would silently erase every app's grants. Move
                    // it aside and start clean at the original path.
                    eprintln!(
                        "[permbroker] Corrupt permission database {}: {}",
                        self.db_path, e
                    );
                    let quarantine = format!("{}.corrupt", self.db_path);
                    match fs::rename(&self.db_path, &quarantine) {
                        Ok(_) => eprintln!(
                            "[permbroker] Quarantined corrupt database as {}",
                            quarantine
                        ),
                        Err(re) => {
                            eprintln!(
                                "[permbroker] Could not quarantine corrupt database: {}",
                                re
                            );
                            // Fall back to writing elsewhere rather than
                            // destroying evidence of the corruption.
                            self.db_path = quarantine;
                        }
                    }
                }
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => eprintln!("[permbroker] Could not read {}: {}", self.db_path, e),
        }
    }

    /// Write via a temporary file plus rename so a crash mid-write cannot
    /// truncate the existing database and silently revoke every grant.
    pub fn save(&self) {
        let data = match serde_json::to_string_pretty(&self.records) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("[permbroker] Could not serialize permission database: {}", e);
                return;
            }
        };
        if let Some(parent) = std::path::Path::new(&self.db_path).parent() {
            let _ = fs::create_dir_all(parent);
        }
        let tmp = format!("{}.tmp.{}", self.db_path, std::process::id());
        if let Err(e) = fs::write(&tmp, &data) {
            eprintln!("[permbroker] Could not write {}: {}", tmp, e);
            return;
        }
        if let Err(e) = fs::rename(&tmp, &self.db_path) {
            eprintln!("[permbroker] Could not replace {}: {}", self.db_path, e);
            let _ = fs::remove_file(&tmp);
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
                // saturating_sub: a future timestamp (clock set backwards or a
                // hand-edited database) must not underflow and revoke or panic.
                if rec.granted && current.saturating_sub(rec.last_used_timestamp) > max_age_secs {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_db(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("permbroker-{}-{}.json", std::process::id(), name))
            .to_string_lossy()
            .into_owned()
    }

    #[test]
    fn grants_persist_across_reload() {
        let path = temp_db("persist");
        let _ = fs::remove_file(&path);
        {
            let mut broker = PermissionBroker::new(&path);
            broker.grant_permission("org.onuron.notes", "storage.read");
            assert!(broker.check_permission("org.onuron.notes", "storage.read"));
        }
        let mut reopened = PermissionBroker::new(&path);
        assert!(reopened.check_permission("org.onuron.notes", "storage.read"));
        assert!(!reopened.check_permission("org.onuron.notes", "camera"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn future_timestamp_does_not_revoke_or_panic() {
        let path = temp_db("future");
        let _ = fs::remove_file(&path);
        let mut broker = PermissionBroker::new(&path);
        broker.records.insert(
            "app".into(),
            HashMap::from([(
                "camera".to_string(),
                PermissionRecord { granted: true, last_used_timestamp: u64::MAX },
            )]),
        );
        // Must not panic and must keep the grant: the timestamp is not stale.
        broker.auto_revoke_unused(7 * 86400);
        assert!(broker.check_permission("app", "camera"));
        let _ = fs::remove_file(&path);
    }

    #[test]
    fn corrupt_database_is_quarantined_not_erased() {
        let path = temp_db("corrupt");
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(format!("{}.corrupt", path));
        fs::write(&path, b"{ not json").unwrap();
        let mut broker = PermissionBroker::new(&path);
        // The corrupt bytes must be preserved for diagnosis.
        assert_eq!(
            fs::read_to_string(format!("{}.corrupt", path)).unwrap(),
            "{ not json"
        );
        // The original path is usable again for fresh grants.
        broker.grant_permission("org.onuron.calc", "display");
        assert!(broker.check_permission("org.onuron.calc", "display"));
        let _ = fs::remove_file(&path);
        let _ = fs::remove_file(format!("{}.corrupt", path));
    }

    #[test]
    fn save_is_atomic_and_leaves_no_temp_files() {
        let path = temp_db("atomic");
        let _ = fs::remove_file(&path);
        let mut broker = PermissionBroker::new(&path);
        broker.grant_permission("app", "network");
        broker.save();
        let leftovers: Vec<_> = fs::read_dir(std::path::Path::new(&path).parent().unwrap())
            .unwrap()
            .flatten()
            .filter(|e| e.file_name().to_string_lossy().contains("atomic.json.tmp"))
            .collect();
        assert!(leftovers.is_empty(), "temporary files left behind: {:?}", leftovers);
        let _ = fs::remove_file(&path);
    }
}
