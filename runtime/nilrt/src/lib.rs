// runtime/nilrt/src/lib.rs — NilOS Runtime Core Modules
pub mod sandbox;
pub mod seccomp;
pub mod permbroker;
pub mod selinux;
pub mod permissions;
pub mod lifecycle;

pub use sandbox::{plan_sandbox_mounts, spawn_sandboxed, SandboxConfig};
pub use seccomp::apply_app_seccomp;
pub use permbroker::PermissionBroker;
pub use permissions::{check_app_permissions, AppPolicy, PermissionDecision};
pub use selinux::setexeccon;