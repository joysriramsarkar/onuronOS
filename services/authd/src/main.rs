// services/authd/src/main.rs — Fingerprint Biometric Authentication Daemon (authd)
// Manages biometric fingerprint template enrollment, matching, hardware devfs interfacing,
// and security lockout policies for OnuronOS lockscreen and authentication prompts.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

pub const AUTH_SOCK_PATH: &str = "/run/nilos/auth.sock";
pub const FP_DEV_PATH_FPC: &str = "/dev/fpc1020";
pub const FP_DEV_PATH_GOODIX: &str = "/dev/goodix_fp";
pub const MATCH_THRESHOLD: f32 = 0.85; // 85% feature similarity required
pub const MAX_FAILED_ATTEMPTS: u32 = 5;
pub const LOCKOUT_DURATION_SECS: u64 = 30;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct FingerprintTemplate {
    pub id: u32,
    pub name: String,
    pub user_id: u32,
    pub enrolled_at: u64,
    pub feature_vector: Vec<u8>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct AuthResponse {
    pub success: bool,
    pub matched_id: Option<u32>,
    pub score: f32,
    pub locked_out: bool,
    pub error: Option<String>,
}

/// Computes feature similarity score between enrolled template and probe vector (0.0 to 1.0)
pub fn compute_similarity_score(template: &[u8], probe: &[u8]) -> f32 {
    if template.is_empty() || probe.is_empty() {
        return 0.0;
    }

    let min_len = template.len().min(probe.len());
    let max_len = template.len().max(probe.len());

    let mut total_bit_diff = 0u32;
    for i in 0..min_len {
        total_bit_diff += (template[i] ^ probe[i]).count_ones();
    }
    // Add penalty for length difference
    total_bit_diff += ((max_len - min_len) * 8) as u32;

    let total_bits = (max_len * 8) as f32;
    let matching_bits = total_bits - (total_bit_diff as f32);
    (matching_bits / total_bits).max(0.0)
}

pub struct AuthdState {
    pub templates: Vec<FingerprintTemplate>,
    pub failed_attempts: u32,
    pub locked_until_ts: u64,
    pub next_template_id: u32,
}

pub struct AuthdService {
    pub state: Arc<Mutex<AuthdState>>,
    pub storage_dir: PathBuf,
}

impl AuthdService {
    pub fn new(storage_dir: PathBuf) -> Self {
        let mut svc = Self {
            state: Arc::new(Mutex::new(AuthdState {
                templates: Vec::new(),
                failed_attempts: 0,
                locked_until_ts: 0,
                next_template_id: 1,
            })),
            storage_dir,
        };
        svc.load_templates();
        svc
    }

    fn load_templates(&mut self) {
        let file_path = self.storage_dir.join("fingerprints.json");
        if let Ok(content) = fs::read_to_string(file_path) {
            if let Ok(templates) = serde_json::from_str::<Vec<FingerprintTemplate>>(&content) {
                let mut st = self.state.lock().unwrap();
                let max_id = templates.iter().map(|t| t.id).max().unwrap_or(0);
                st.next_template_id = max_id + 1;
                st.templates = templates;
            }
        }
    }

    pub fn save_templates(&self) -> std::io::Result<()> {
        let _ = fs::create_dir_all(&self.storage_dir);
        let file_path = self.storage_dir.join("fingerprints.json");
        let st = self.state.lock().unwrap();
        let json = serde_json::to_string_pretty(&st.templates)?;
        fs::write(file_path, json)
    }

    pub fn enroll_fingerprint(
        &self,
        user_id: u32,
        name: &str,
        feature_vector: Vec<u8>,
    ) -> Result<FingerprintTemplate, String> {
        if feature_vector.is_empty() {
            return Err("Empty feature vector cannot be enrolled".to_string());
        }

        let mut st = self.state.lock().unwrap();
        let id = st.next_template_id;
        st.next_template_id += 1;

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let tmpl = FingerprintTemplate {
            id,
            name: name.to_string(),
            user_id,
            enrolled_at: timestamp,
            feature_vector,
        };

        st.templates.push(tmpl.clone());
        drop(st);
        let _ = self.save_templates();

        println!("[authd] Enrolled fingerprint '{}' (ID {}) for user {}", name, id, user_id);
        Ok(tmpl)
    }

    pub fn authenticate(&self, user_id: u32, probe_vector: &[u8], current_ts: u64) -> AuthResponse {
        let mut st = self.state.lock().unwrap();

        // Check if currently locked out
        if current_ts < st.locked_until_ts {
            let remaining = st.locked_until_ts - current_ts;
            return AuthResponse {
                success: false,
                matched_id: None,
                score: 0.0,
                locked_out: true,
                error: Some(format!(
                    "Biometric authentication locked out. Retry in {} seconds.",
                    remaining
                )),
            };
        } else if st.locked_until_ts > 0 {
            st.locked_until_ts = 0;
            st.failed_attempts = 0;
        }

        // Find best match among templates for this user
        let mut best_score = 0.0f32;
        let mut matched_id = None;

        for tmpl in st.templates.iter().filter(|t| t.user_id == user_id) {
            let score = compute_similarity_score(&tmpl.feature_vector, probe_vector);
            if score > best_score {
                best_score = score;
                if score >= MATCH_THRESHOLD {
                    matched_id = Some(tmpl.id);
                }
            }
        }

        if let Some(id) = matched_id {
            // Authentication successful! Reset lockout counter
            st.failed_attempts = 0;
            st.locked_until_ts = 0;
            println!(
                "[authd] Biometric auth SUCCESS for user {}: matched template ID {}, score {:.3}",
                user_id, id, best_score
            );
            AuthResponse {
                success: true,
                matched_id: Some(id),
                score: best_score,
                locked_out: false,
                error: None,
            }
        } else {
            // Failure: increment failed attempts
            st.failed_attempts += 1;
            println!(
                "[authd] Biometric auth FAILURE for user {}: score {:.3} (attempt {}/{})",
                user_id, best_score, st.failed_attempts, MAX_FAILED_ATTEMPTS
            );

            let locked = if st.failed_attempts >= MAX_FAILED_ATTEMPTS {
                st.locked_until_ts = current_ts + LOCKOUT_DURATION_SECS;
                println!(
                    "[authd] User {} locked out for {} seconds due to consecutive failures.",
                    user_id, LOCKOUT_DURATION_SECS
                );
                true
            } else {
                false
            };

            AuthResponse {
                success: false,
                matched_id: None,
                score: best_score,
                locked_out: locked,
                error: Some(if locked {
                    "Too many failed attempts. Locked out for 30 seconds.".to_string()
                } else {
                    "Fingerprint not recognized. Try again.".to_string()
                }),
            }
        }
    }
}

/// Checks if hardware fingerprint device exists in devfs
pub fn detect_fingerprint_hardware() -> Option<&'static str> {
    if Path::new(FP_DEV_PATH_FPC).exists() {
        Some(FP_DEV_PATH_FPC)
    } else if Path::new(FP_DEV_PATH_GOODIX).exists() {
        Some(FP_DEV_PATH_GOODIX)
    } else {
        None
    }
}

fn main() {
    println!("[authd] Fingerprint Biometric Authentication Daemon starting...");
    let hw = detect_fingerprint_hardware().unwrap_or("simulated / mock");
    println!("[authd] Hardware driver interface: {}", hw);

    let storage_dir = PathBuf::from("/data/system/auth");
    let service = AuthdService::new(storage_dir);

    let count = service.state.lock().unwrap().templates.len();
    println!("[authd] Loaded {} enrolled fingerprint templates.", count);

    loop {
        std::thread::sleep(Duration::from_secs(60));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_similarity_score_exact_match() {
        let v1 = vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];
        let v2 = vec![0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc];
        let score = compute_similarity_score(&v1, &v2);
        assert!((score - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_similarity_score_slight_variation() {
        let v1 = vec![0x00; 32];
        let mut v2 = vec![0x00; 32];
        // Flip 2 bits out of 256 bits (similarity should be 254 / 256 = 0.992)
        v2[0] = 0x03;
        let score = compute_similarity_score(&v1, &v2);
        assert!(score > 0.99);
    }

    #[test]
    fn test_enrollment_and_authentication_flow() {
        let tmp = std::env::temp_dir().join(format!("test_authd_{}", std::process::id()));
        let svc = AuthdService::new(tmp.clone());

        let tmpl_data = vec![0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x11, 0x22];
        let enrolled = svc
            .enroll_fingerprint(1000, "Right Thumb", tmpl_data.clone())
            .expect("Enroll");
        assert_eq!(enrolled.name, "Right Thumb");

        // Auth with identical probe
        let resp_ok = svc.authenticate(1000, &tmpl_data, 1000);
        assert!(resp_ok.success);
        assert_eq!(resp_ok.matched_id, Some(enrolled.id));
        assert!(resp_ok.score > 0.99);

        // Auth with wrong user
        let resp_wrong_user = svc.authenticate(1001, &tmpl_data, 1000);
        assert!(!resp_wrong_user.success);

        // Auth with completely different probe
        let wrong_probe = vec![0x55; 8];
        let resp_fail = svc.authenticate(1000, &wrong_probe, 1000);
        assert!(!resp_fail.success);

        let _ = fs::remove_dir_all(tmp);
    }

    #[test]
    fn test_lockout_after_max_failed_attempts() {
        let tmp = std::env::temp_dir().join(format!("test_authd_lockout_{}", std::process::id()));
        let svc = AuthdService::new(tmp.clone());

        let tmpl_data = vec![0x01, 0x02, 0x03, 0x04];
        svc.enroll_fingerprint(1000, "Index", tmpl_data).expect("Enroll");

        let bad_probe = vec![0xfe, 0xfd, 0xfc, 0xfb];

        // Fail 4 times
        for i in 1..=4 {
            let r = svc.authenticate(1000, &bad_probe, 1000 + i);
            assert!(!r.success);
            assert!(!r.locked_out);
        }

        // 5th failure triggers lockout
        let r5 = svc.authenticate(1000, &bad_probe, 1005);
        assert!(!r5.success);
        assert!(r5.locked_out);

        // Immediate retry is blocked
        let r_blocked = svc.authenticate(1000, &bad_probe, 1010);
        assert!(r_blocked.locked_out);
        assert!(r_blocked.error.unwrap().contains("locked out"));

        // After 30 seconds lockout expires
        let r_recovered = svc.authenticate(1000, &bad_probe, 1005 + 31);
        assert!(!r_recovered.locked_out); // lockout ended

        let _ = fs::remove_dir_all(tmp);
    }
}
