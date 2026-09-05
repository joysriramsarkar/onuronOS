# 📘 Onuron OS (অনুরণ ওএস)

> **A lightweight, secure, Linux-based mobile operating system powered by the Alap (আলাপ) cross-platform framework, with a 100% Rust userspace, native NilLang (.nil) app ecosystem (.nilax), and containerized Android compatibility.**

Onuron OS combines the reliability of the Linux LTS kernel, the memory safety and efficiency of a 100% Rust userspace, a declarative UI shell, and an isolated containerized Android compatibility layer—built with a bloat-free, zero-telemetry philosophy.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

---

## 📊 Subsystem Maturity & Reality

To maintain radical engineering honesty and credibility, Onuron OS uses a 5-tier maturity classification:
- 🟢 **Production-ready**: Fully implemented, hardened, and verified in real environments.
- 🔵 **Functional prototype**: Usable in QEMU / development prototypes, core logic working.
- 🟡 **Experimental**: Under active development; partial implementation or architecture scaffold.
- 🟠 **Stub / simulated**: Skeleton daemon or UI-level simulation; underlying hardware/protocol not yet wired.
- 🔴 **Not implemented**: Architecture planned or designed; implementation pending.

| Subsystem / Feature | Maturity | Details & Reality |
|---|---|---|
| **Linux Kernel Boot** | 🔵 Functional prototype | Linux LTS 6.6 x86_64, bootable under QEMU with initramfs |
| **System Init (`nilinit`)** | 🔵 Functional prototype | PID 1 init, clean `[  OK  ]` boot logging, mounts, supervision, socket activation |
| **Storage Hierarchy** | 🔵 Functional prototype | `/data` ext4 persistent disk on virtio-blk + tmpfs fallback; mobile layout (`/system`, `/vendor`, `/data/user/`) |
| **QEMU Boot Automation** | 🔵 Functional prototype | Persistent `nilos.img` disk + virtio-blk + user-mode NAT networking |
| **First-Boot Setup (OOBE)** | 🔵 Functional prototype | Name & PIN setup wizard, writes configuration to `/data/config/` |
| **Lock Screen** | 🔵 Functional prototype | PIN verification, clock/date display, unlock lifecycle |
| **Home Launcher** | 🔵 Functional prototype | App grid, status bar, notification shade |
| **Phone App** | 🟠 Stub / Simulated | Dialer UI & contact list functional; **simulated VoLTE** (real modem AT layer planned) |
| **Messages App** | 🔵 Functional prototype | SMS message threads, composer, persistent storage under `/data/sms/` |
| **Files App** | 🔵 Functional prototype | Directory explorer for `/data`, `/etc`, `/tmp`, `/data/app` |
| **Settings App** | 🔵 Functional prototype | System settings UI for Network, Display, Security, Battery, Storage |
| **Permission Broker** | 🔵 Functional prototype | JSON-persisted grants with 7-day auto-revoke policy |
| **Namespace Sandbox** | 🔵 Functional prototype | Linux `unshare(CLONE_NEWPID\|CLONE_NEWNS\|CLONE_NEWIPC\|CLONE_NEWUTS)` + `chroot` isolation |
| **Seccomp BPF Filter** | 🔵 Functional prototype | Real `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)` syscall allowlist (~110 syscalls) |
| **SoftBus Distributed Mesh** | 🔵 Functional prototype | Real mDNS-SD peer discovery + Quinn QUIC/TLS 1.3 transport |
| **Input Daemon (`inputd`)** | 🟡 Experimental | Linux `evdev` (`/dev/input/event*`) reader for multitouch, power, volume keys, and IPC |
| **Power Daemon (`powerd`)** | 🟡 Experimental | Linux `/sys/class/power_supply` reader, wakelock tracker, suspend control |
| **Network Daemon (`netd`)** | 🟡 Experimental | Linux `/sys/class/net` monitor, link status, DNS, and network IPC |
| **Package Manager (`nilpkg`)** | 🟡 Experimental | Atomic install to `/data/app/`; **Ed25519 digital signature + SHA-256 integrity verification** |
| **Shell Compositor (`nilshell`)** | 🟡 Experimental | ANSI/minifb console rendering; **DRM/KMS dumb-buffer & Wayland compositor in development** |
| **ARM64 Device Port** | 🟡 Experimental | NilHAL GKI/Treble abstraction skeleton; QEMU aarch64 virt & PinePhone target |
| **Android Compatibility** | 🟠 Stub / Planned | JSON agent protocol wrapper; LXC/Waydroid container provisioning in progress |

---

## 🗺️ 6-Phase Engineering Roadmap

Onuron OS adheres to a sequential 6-phase engineering trajectory:

```
┌─────────────────────────────────────────────────────────────┐
│  Phase 1: Bootable Prototype (Completed)                    │
│  Linux kernel → nilinit (PID 1) → virtual filesystems       │
│  → QEMU boot automation                                     │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 2: Usable Prototype & Shell Architecture (Completed) │
│  Persistent /data · PIN Lockscreen · OOBE · Launcher ·      │
│  Files · Settings · SoftBus P2P Mesh                        │
└──────────────────────────────┬──────────────────────────────┘
                               ▼ (CURRENT FOCUS)
┌─────────────────────────────────────────────────────────────┐
│  Phase 3: Real Hardware & Core Subsystems                   │
│  DRM/KMS Framebuffer · evdev Input Daemon (inputd) ·        │
│  Power Daemon (powerd) · Network Daemon (netd) ·            │
│  Ed25519 Signed nilpkg · ARM64 QEMU & PinePhone Target      │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 4: Native Application Platform                       │
│  NilLang Native Apps (.nil) · Alap Framework · NilUI ·      │
│  .nilax Package Verification · Granular UID Sandboxing      │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 5: Containerized Android Compatibility               │
│  LXC/Waydroid Container · Binder shim translation ·         │
│  Headless Android Framework integration                     │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 6: Production Hardening                              │
│  Verified Boot · A/B OTA Updates · fscrypt v2 Encryption ·   │
│  SELinux Enforcing · Automated Security Test Matrix         │
└──────────────────────────────┘
```

---

## 📚 Technical Documentation

Modular architectural specifications are located in [`docs/`](docs/):

- **[Architecture](docs/architecture.md)** — Architectural layers, memory-safety principles, and IPC design.
- **[Boot Process](docs/boot-process.md)** — Boot chain, storage hierarchy, `nilinit` PID 1, and socket activation.
- **[DRM/KMS Compositor](docs/drm-kms-compositor.md)** — DRM/KMS framebuffer architecture, page-flipping, and Wayland IPC.
- **[Security](docs/security.md)** — SELinux policy, namespace sandboxing, Ed25519 package verification, and zero-telemetry charter.
- **[UI System](docs/ui-system.md)** — NilUI declarative framework, `nilui-gpu` renderer, and `nilshell` compositor.
- **[Hardware Support](docs/hardware-support.md)** — Linux LTS baseline, ARM64 target strategy, PinePhone, and driver model.
- **[Android Compatibility](docs/android-compatibility.md)** — Headless container approach, binder-shim translation, and lifecycle management.
- **[Roadmap & Milestones](docs/roadmap.md)** — 6-phase engineering milestones, team deliverables, and governance.

---

## 📁 Repository Structure

```
onuronOS/
├── Cargo.toml                  # Cargo Workspace configuration
├── build/                      # Build, toolchain, disk image, and QEMU boot scripts
├── kernel/                     # Kernel defconfig fragments (Base, x86, ARM64, Halium)
├── hal/                        # Hardware Abstraction Layer (C-ABI & Drivers)
├── nilinit/                    # PID 1 System Init & Supervised Socket Activation
├── runtime/
│   ├── nilsd/                  # Socket activation helper library
│   ├── nilhal/                 # Safe dlopen HAL loader & diagnostic CLI
│   ├── nilrt/                  # Namespace Sandbox, seccomp BPF, permbroker
│   ├── nilui/                  # Declarative reactive UI framework & animations
│   ├── nilui-gpu/              # Vulkan 2D renderer, SDF rects & HarfBuzz shaping
│   └── nilbus-client/          # SoftBus P2P IPC client library
├── shell/                      # Mobile Shell & Compositor (nilshell)
├── softbus/                    # Distributed SoftBus daemon (mDNS-SD + QUIC/TLS 1.3)
├── pkg/nilpkg/                 # Signed Atomic Package Manager (SHA-256 + Ed25519)
├── services/                   # System Daemons
│   ├── inputd/                 # Unified Linux evdev input daemon (multitouch, keys)
│   ├── powerd/                 # Battery governor, sysfs monitor & wakelock manager
│   ├── netd/                   # Network interface, link state & DNS manager
│   ├── btd/                    # Bluetooth daemon abstraction
│   ├── audiod/                 # Audio routing daemon
│   ├── camerad/                # Camera daemon
│   ├── thermald/               # Thermal sensor monitoring & throttling
│   └── ...                     # Additional supervised micro-daemons
├── android/                    # Android compatibility layer & in-container agent
├── apps/                       # Native Applications & Shell Demos
├── security/selinux/           # Comprehensive SELinux CIL Security Policies & CI
├── etc/nilos/                  # System services configuration & design tokens
└── docs/                       # Modular Architecture & Subsystem Documentation
```

---

## 🚀 Building & Running Onuron OS

### Quick Boot in QEMU

Onuron OS can be built and booted in QEMU on Windows, Linux, or macOS:

```powershell
# Windows (PowerShell)
.\build\qemu-boot.ps1
```

```bash
# Linux / macOS / WSL
./build/qemu-boot.sh
```

---

## 📜 License & Open-Source Guarantee

Onuron OS is distributed under the **[GNU General Public License v3.0 (GPLv3)](LICENSE)**.

> **Copyleft Protection**: Anyone is free to use, modify, contribute to, and build derivative operating systems from Onuron OS, provided that all modifications and derivative systems remain **100% free and open-source under the GNU GPLv3 license**.
