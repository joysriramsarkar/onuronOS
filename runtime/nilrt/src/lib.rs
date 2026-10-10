// runtime/nilrt/src/lib.rs — NilOS Runtime Core Modules
pub mod sandbox;
pub mod seccomp;
pub mod permbroker;
pub mod selinux;
pub mod permissions;
pub mod lifecycle;
pub mod cgroup;

pub use sandbox::{plan_sandbox_mounts, spawn_sandboxed, SandboxConfig};
pub use seccomp::apply_app_seccomp;
pub use permbroker::PermissionBroker;
pub use permissions::{check_app_permissions, AppPolicy, PermissionDecision};
pub use selinux::{get_selinux_mode, is_selinux_enforcing, setexeccon, SelinuxMode};
pub use cgroup::{attach_pid_to_cgroup, create_app_cgroup, CgroupLimits};