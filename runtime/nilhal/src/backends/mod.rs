// runtime/nilhal/src/backends/mod.rs — NilHAL Backend Architecture
pub mod qemu;
pub mod linux;
pub mod android;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BackendType {
    Qemu,
    Linux,
    Android,
    NativeArm64,
}

impl std::fmt::Display for BackendType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BackendType::Qemu => write!(f, "QEMU / Simulator (Mode 1)"),
            BackendType::Android => write!(f, "Android Hosted S25 (Mode 2)"),
            BackendType::Linux => write!(f, "Linux Hardware (Mode 3-Dev)"),
            BackendType::NativeArm64 => write!(f, "Native ARM64 Bare Metal (Mode 3)"),
        }
    }
}
