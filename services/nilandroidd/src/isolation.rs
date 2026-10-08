// services/nilandroidd/src/isolation.rs — Onuron OS Android Container Isolation Boundary
//
// Governed by OnuronOS Architecture & AGENTS.md:
// 1. Android compatibility is an isolated subsystem, never the foundation.
// 2. Sandboxing, SELinux, and namespace isolation must NEVER be bypassed.
// 3. The guest container has zero direct access to raw host devices (/dev/input, /dev/kmsg, /dev/mem, direct DRM).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Namespace isolation specification for Android guest container
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct NamespaceIsolation {
    pub mount_namespace: bool,
    pub pid_namespace: bool,
    pub ipc_namespace: bool,
    pub net_namespace: bool,
    pub uts_namespace: bool,
    pub user_namespace: bool,
}

impl Default for NamespaceIsolation {
    fn default() -> Self {
        Self {
            mount_namespace: true,
            pid_namespace: true,
            ipc_namespace: true,
            net_namespace: true,
            uts_namespace: true,
            user_namespace: true,
        }
    }
}

/// Resource constraints (cgroup v2) for the guest container
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub memory_max_bytes: u64,       // e.g. 3 GiB default cap
    pub memory_swap_max_bytes: u64,
    pub cpu_quota_us: u64,           // e.g. 800_000 per 1_000_000 period
    pub cpu_period_us: u64,
    pub max_pids: u32,               // Prevent fork-bombing host
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            memory_max_bytes: 3 * 1024 * 1024 * 1024, // 3 GiB
            memory_swap_max_bytes: 512 * 1024 * 1024,
            cpu_quota_us: 800_000,
            cpu_period_us: 1_000_000,
            max_pids: 4096,
        }
    }
}

/// Hardware & device node access policy for Android container
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAccessPolicy {
    pub allow_raw_evdev: bool,         // MUST BE FALSE
    pub allow_raw_kmsg: bool,          // MUST BE FALSE
    pub allow_raw_mem: bool,           // MUST BE FALSE
    pub allow_direct_drm_master: bool, // MUST BE FALSE
    pub allowed_devices: HashSet<PathBuf>,
}

impl Default for DeviceAccessPolicy {
    fn default() -> Self {
        let mut allowed = HashSet::new();
        allowed.insert(PathBuf::from("/dev/null"));
        allowed.insert(PathBuf::from("/dev/zero"));
        allowed.insert(PathBuf::from("/dev/full"));
        allowed.insert(PathBuf::from("/dev/random"));
        allowed.insert(PathBuf::from("/dev/urandom"));
        allowed.insert(PathBuf::from("/dev/ashmem"));
        allowed.insert(PathBuf::from("/dev/binder"));
        allowed.insert(PathBuf::from("/dev/hwbinder"));
        allowed.insert(PathBuf::from("/dev/vndbinder"));

        Self {
            allow_raw_evdev: false,
            allow_raw_kmsg: false,
            allow_raw_mem: false,
            allow_direct_drm_master: false,
            allowed_devices: allowed,
        }
    }
}

/// Canonical container security profile ensuring Android compatibility cannot compromise native OnuronOS
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContainerProfile {
    pub rootfs: PathBuf,
    pub selinux_context: String,
    pub namespaces: NamespaceIsolation,
    pub limits: ResourceLimits,
    pub devices: DeviceAccessPolicy,
}

impl Default for ContainerProfile {
    fn default() -> Self {
        Self {
            rootfs: PathBuf::from("/data/android/rootfs"),
            selinux_context: "u:r:android_container:s0".to_string(),
            namespaces: NamespaceIsolation::default(),
            limits: ResourceLimits::default(),
            devices: DeviceAccessPolicy::default(),
        }
    }
}

impl ContainerProfile {
    /// Validate that this profile strictly enforces OnuronOS canonical isolation rules
    pub fn validate_isolation(&self) -> Result<(), String> {
        if !self.namespaces.mount_namespace {
            return Err("Mount namespace must be enabled for Android container isolation".into());
        }
        if !self.namespaces.pid_namespace {
            return Err("PID namespace must be enabled for Android container isolation".into());
        }
        if !self.namespaces.ipc_namespace {
            return Err("IPC namespace must be enabled to prevent host SysV/POSIX IPC leak".into());
        }
        if self.devices.allow_raw_evdev {
            return Err("Direct evdev input access is forbidden; must route through inputd/bridge".into());
        }
        if self.devices.allow_raw_mem || self.devices.allow_raw_kmsg {
            return Err("Raw /dev/mem or /dev/kmsg access is strictly forbidden".into());
        }
        if self.devices.allow_direct_drm_master {
            return Err("Direct DRM master access is forbidden; compositor owns the display".into());
        }
        if self.limits.memory_max_bytes == 0 || self.limits.memory_max_bytes > 16 * 1024 * 1024 * 1024 {
            return Err("Unbounded or invalid memory limits configured".into());
        }
        if !self.selinux_context.starts_with("u:r:android_container") {
            return Err(format!("Invalid SELinux domain '{}'; expected android_container", self.selinux_context));
        }
        Ok(())
    }

    /// Check if a given device access request by the guest container is permitted
    pub fn is_device_permitted(&self, dev_path: &Path) -> bool {
        let path_str = dev_path.to_string_lossy();
        if path_str.starts_with("/dev/input") || path_str == "/dev/kmsg" || path_str == "/dev/mem" || path_str == "/dev/port" {
            return false;
        }
        self.devices.allowed_devices.contains(dev_path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile_is_valid() {
        let profile = ContainerProfile::default();
        assert!(profile.validate_isolation().is_ok());
    }

    #[test]
    fn test_forbidden_devices_rejected() {
        let profile = ContainerProfile::default();

        assert!(!profile.is_device_permitted(Path::new("/dev/input/event0")));
        assert!(!profile.is_device_permitted(Path::new("/dev/kmsg")));
        assert!(!profile.is_device_permitted(Path::new("/dev/mem")));
        assert!(!profile.is_device_permitted(Path::new("/dev/port")));

        assert!(profile.is_device_permitted(Path::new("/dev/null")));
        assert!(profile.is_device_permitted(Path::new("/dev/urandom")));
        assert!(profile.is_device_permitted(Path::new("/dev/binder")));
    }

    #[test]
    fn test_bypass_attempt_fails_validation() {
        let mut profile = ContainerProfile::default();
        profile.devices.allow_raw_evdev = true;
        assert!(profile.validate_isolation().is_err());

        let mut profile2 = ContainerProfile::default();
        profile2.namespaces.mount_namespace = false;
        assert!(profile2.validate_isolation().is_err());

        let mut profile3 = ContainerProfile::default();
        profile3.selinux_context = "u:r:kernel:s0".to_string();
        assert!(profile3.validate_isolation().is_err());
    }
}
