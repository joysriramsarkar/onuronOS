// runtime/nilrt/src/lib.rs — NilOS Runtime Core Modules
pub mod cgroup;
pub mod lifecycle;
pub mod permbroker;
pub mod permissions;
pub mod sandbox;
pub mod seccomp;
pub mod selinux;

pub use cgroup::{attach_pid_to_cgroup, create_app_cgroup, CgroupLimits};
pub use permbroker::PermissionBroker;
pub use permissions::{check_app_permissions, AppPolicy, PermissionDecision};
pub use sandbox::{plan_sandbox_mounts, spawn_sandboxed, SandboxConfig};
pub use seccomp::apply_app_seccomp;
pub use selinux::{get_selinux_mode, is_selinux_enforcing, setexeccon, SelinuxMode};
