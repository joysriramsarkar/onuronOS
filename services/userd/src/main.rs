// services/userd/src/main.rs — Multi-User & Work Profile Lifecycle Manager
use std::thread;
use std::time::Duration;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProfileKind {
    Owner,
    SecondaryUser,
    WorkProfile,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserProfile {
    pub user_id: u32,
    pub base_uid: u32,
    pub name: String,
    pub kind: ProfileKind,
    pub active: bool,
}

pub struct UserManager {
    profiles: Vec<UserProfile>,
}

impl Default for UserManager {
    fn default() -> Self {
        Self::new()
    }
}

impl UserManager {
    pub fn new() -> Self {
        Self {
            profiles: vec![UserProfile {
                user_id: 0,
                base_uid: 1000,
                name: "Owner".into(),
                kind: ProfileKind::Owner,
                active: true,
            }],
        }
    }

    pub fn create_profile(&mut self, name: &str, kind: ProfileKind) -> Result<UserProfile, String> {
        let clean = name.trim();
        if clean.is_empty() {
            return Err("profile name cannot be empty".into());
        }
        let next_id = self.profiles.iter().map(|p| p.user_id).max().unwrap_or(0) + 10;
        let base_uid = 1000 + next_id * 1000;
        let profile = UserProfile {
            user_id: next_id,
            base_uid,
            name: clean.to_string(),
            kind,
            active: false,
        };
        self.profiles.push(profile.clone());
        Ok(profile)
    }

    pub fn set_active(&mut self, user_id: u32, active: bool) -> bool {
        if let Some(p) = self.profiles.iter_mut().find(|p| p.user_id == user_id) {
            if p.user_id == 0 && !active {
                // Primary owner profile cannot be deactivated
                return false;
            }
            p.active = active;
            true
        } else {
            false
        }
    }

    pub fn remove_profile(&mut self, user_id: u32) -> Result<(), String> {
        if user_id == 0 {
            return Err("cannot remove primary owner profile (0)".into());
        }
        let prev = self.profiles.len();
        self.profiles.retain(|p| p.user_id != user_id);
        if self.profiles.len() < prev {
            Ok(())
        } else {
            Err("profile not found".into())
        }
    }

    pub fn list(&self) -> &[UserProfile] {
        &self.profiles
    }
}

fn main() {
    println!("[userd] Multi-User & Work Profile Lifecycle Manager active.");
    let _ = nilsd::notify_ready("userd", None);
    let _mgr = UserManager::new();

    loop {
        thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_and_work_profile_lifecycle() {
        let mut mgr = UserManager::new();
        assert_eq!(mgr.list().len(), 1);
        assert!(mgr.remove_profile(0).is_err());

        let work = mgr
            .create_profile("Work", ProfileKind::WorkProfile)
            .unwrap();
        assert_eq!(work.user_id, 10);
        assert_eq!(work.base_uid, 11000);
        assert!(mgr.set_active(10, true));
        assert!(mgr.remove_profile(10).is_ok());
        assert_eq!(mgr.list().len(), 1);
    }
}
