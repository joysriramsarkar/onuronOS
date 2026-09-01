# ⚡ NilOS Boot Process & Init Lifecycle

This document describes the end-to-end boot sequence of NilOS, from firmware handoff to system supervision.

---

## 1. Multi-Stage Boot Pipeline

```
┌─────────────────┐
│     Stage 0     │  Firmware / Bootloader (AVB / Ed25519 Verified Boot)
│   (Bootloader)  │  Validates boot signature & vbmeta.img
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Stage 1     │  Kernel (vmlinuz-lts) unpacks initramfs into tmpfs
│   (initramfs)   │  Executes /init (nilinit) in early environment
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Stage 2     │  Mounts immutable system_a/system_b rootfs
│  (System Root)  │  Loads SELinux policy, switches root, supervises services
└─────────────────┘
```

---

## 2. Partition Topology (A/B Redundancy)

NilOS utilizes standard GPT / UFS partition layouts with A/B slot redundancy to guarantee fail-safe over-the-air updates:

| Partition | Type | Mount Point | Purpose |
|---|---|---|---|
| `boot_a` / `boot_b` | Raw | N/A | Linux LTS kernel + boot DTB |
| `system_a` / `system_b` | ext4 / EROFS | `/` (read-only) | Immutable base system image & binaries |
| `vendor_a` / `vendor_b` | ext4 / EROFS | `/vendor` | Device-specific HAL blobs and firmware |
| `userdata` | ext4 (fscrypt v2) | `/data` | User application data, encrypted per-user |
| `vbmeta_a` / `vbmeta_b`| Raw | N/A | Cryptographic hash tree signatures |

---

## 3. `nilinit`: PID 1 System Supervisor

`nilinit` serves as the initial userspace process (PID 1). Written in Rust, it provides:
1. **Early Mounts**: Mounts standard Linux virtual filesystems:
   - `/proc` (`proc`)
   - `/sys` (`sysfs`)
   - `/dev` (`devtmpfs`)
   - `/run` (`tmpfs`)
   - `/tmp` (`tmpfs`)
2. **Cgroups & Slices**: Sets up unified cgroups v2 hierarchy (`/sys/fs/cgroup/nilos.slice`).
3. **SELinux Policy Injection**: Reads `/etc/selinux/targeted/policy/policy.33` and writes to `/sys/fs/selinux/load`.
4. **Service Supervision & Socket Activation**: Parses `/etc/nilos/services.toml`.

### Service Topology (`services.toml`)

```toml
# Eagerly started core daemon
[[services]]
name = "nild"
exec = "/usr/bin/nild"
restart = "always"

# Distributed SoftBus
[[services]]
name = "nilbus"
exec = "/usr/bin/nilbus"
restart = "always"

# Wayland display server
[[services]]
name = "nilshell"
exec = "/usr/bin/nilshell"
restart = "always"

# Lazy, socket-activated service (spawns only when client writes to socket)
[[services]]
name = "notifyd"
exec = "/usr/bin/notifyd"
socket_activation = "/run/nilos/notify.sock"
```

---

## 4. Socket Activation Lifecycle

For lazy services (e.g., `notifyd`, `nilimed`, `nilttsd`):
1. `nilinit` binds the listening UNIX socket at startup (e.g., `/run/nilos/notify.sock`).
2. Sockets are placed in non-blocking mode with event polling.
3. When a client application attempts to connect, `nilinit` detects the pending connection.
4. `nilinit` launches the service with file descriptors passed via environment variables (`LISTEN_FDS=1`, `LISTEN_FDNAMES=notifyd`).
5. The spawned daemon adopts the descriptor via the `nilsd` crate and services the request immediately.
