# ⚡ Onuron OS Boot Process & Init Lifecycle

This document describes the end-to-end boot sequence of Onuron OS, from firmware handoff to system supervision.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

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

## 2. Partition Topology & Mobile Storage Hierarchy

Onuron OS utilizes standard GPT / UFS partition layouts with A/B slot redundancy to guarantee fail-safe over-the-air updates:

| Partition | Type | Mount Point | Purpose |
|---|---|---|---|
| `boot_a` / `boot_b` | Raw | N/A | Linux LTS kernel + boot DTB |
| `system_a` / `system_b` | ext4 / EROFS | `/system` | Immutable base system image & binaries |
| `vendor_a` / `vendor_b` | ext4 / EROFS | `/vendor` | Device-specific HAL blobs and firmware |
| `cache` | ext4 | `/cache` | Temporary OTA download buffer |
| `recovery` | ext4 | `/recovery` | Standalone disaster recovery environment |
| `metadata` | ext4 | `/metadata` | Encryption key metadata & rollback counters |
| `userdata` | ext4 (fscrypt v2) | `/data` | User application data, encrypted per-user |

### Subdirectories under `/data`:
- `/data/user/<uid>/` — Sandboxed isolated home directories per application UID
- `/data/app/` — Atomic `.nilax` package installations
- `/data/system/` — System settings, PIN hashes, state database
- `/data/media/` — Photos, music, downloads, external storage mounts
- `/data/config/` — System configuration flags (e.g. `oobe_done`)

---

## 3. `nilinit`: PID 1 System Supervisor

`nilinit` serves as the initial userspace process (PID 1). Written in Rust, it provides:
1. **Early Mounts**: Mounts standard Linux virtual filesystems:
   - `/proc` (`proc`)
   - `/sys` (`sysfs`)
   - `/dev` (`devtmpfs`)
   - `/run` (`tmpfs`)
   - `/tmp` (`tmpfs`)
2. **Clean Standardized Logging**: Emits clean `[  OK  ]` status notifications to `/dev/kmsg` and `/dev/console`.
3. **Cgroups & Slices**: Sets up unified cgroups v2 hierarchy (`/sys/fs/cgroup/onuron.slice`).
4. **SELinux Policy Injection**: Reads `/etc/selinux/targeted/policy/policy.33` and writes to `/sys/fs/selinux/load`.
5. **Service Supervision & Socket Activation**: Parses `/etc/nilos/services.toml`.

### Service Topology (`services.toml`)

```toml
# Unified evdev input daemon
[[services]]
name = "inputd"
exec = "/usr/bin/inputd"
restart = "always"

# Linux sysfs power governor & battery daemon
[[services]]
name = "powerd"
exec = "/usr/bin/powerd"
restart = "always"

# Network manager daemon
[[services]]
name = "netd"
exec = "/usr/bin/netd"
restart = "always"

# Display shell compositor
[[services]]
name = "nilshell"
exec = "/usr/bin/nilshell"
restart = "always"

# Lazy, socket-activated service (spawns only when client writes to socket)
[[services]]
name = "notifyd"
exec = "/usr/bin/notifyd"
socket_activation = "/run/onuron/notify.sock"
```

---

## 4. Socket Activation Lifecycle

For lazy services (e.g., `notifyd`, `nilimed`, `nilttsd`):
1. `nilinit` binds the listening UNIX socket at startup (e.g., `/run/onuron/notify.sock`).
2. Sockets are placed in non-blocking mode with event polling.
3. When a client application attempts to connect, `nilinit` detects the pending connection.
4. `nilinit` launches the service with file descriptors passed via environment variables (`LISTEN_FDS=1`, `LISTEN_FDNAMES=notifyd`).
5. The spawned daemon adopts the descriptor via the `nilsd` crate and services the request immediately.
