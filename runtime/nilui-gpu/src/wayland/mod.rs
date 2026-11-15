// runtime/nilui-gpu/src/wayland/ — Wayland protocol implementation
//
// A hand-rolled Wayland server, for the same reason `drm.rs` hand-rolls its
// ioctls: no `libwayland-server` headers are available, `deny.toml` gates
// unmaintained crates, and the musl release build statically links.

pub mod client;
pub mod compose;
pub mod protocol;
pub mod server;
pub mod shm;
pub mod wire;
