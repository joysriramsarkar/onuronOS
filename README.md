# 📘 Onuron OS (অনুরণ ওএস)

> **A lightweight, secure, Linux-based mobile OS powered by the Alap (আলাপ) cross-platform framework, with a 100% Rust userspace, native NilLang (.nil) app ecosystem (.nilax), and optional Android container compatibility.**

NilOS combines the reliability of the Linux LTS kernel, the memory safety and efficiency of a 100% Rust userspace, a smooth declarative UI shell, and an isolated containerized Android compatibility layer—built with a bloat-free, zero-telemetry philosophy.

---

## 📊 Current Status

| Subsystem / Feature | Maturity | Details |
|---|---|---|
| **x86_64 Boot** | 🟢 Complete | Linux LTS 6.6 + `nilinit` PID 1, verified in QEMU |
| **QEMU Boot & Automation** | 🟢 Complete | Persistent 256 MB `nilos.img` disk + virtio-blk + NAT networking |
| **Onuron Mobile Shell** | 🟢 Complete | OOBE → Lock Screen → Home Launcher → 8 apps (ANSI/minifb rendered) |
| **Persistent Storage** | 🟢 Complete | `/data` on `/dev/vda` (ext4), fallback to tmpfs; all user data persists |
| **OOBE First-Boot Wizard** | 🟢 Complete | Name + PIN setup, writes `/data/nilos/oobe_done` flag |
| **Lock Screen** | 🟢 Complete | PIN unlock, clock/date/weather display |
| **Home Launcher** | 🟢 Complete | 8-app grid, status bar, notification shade |
| **Phone App** | 🟢 Working | Dialer pad, call log, contact list (simulated VoLTE) |
| **Messages App** | 🟢 Working | SMS threads, compose, persistent storage to `/data/sms/` |
| **Files App** | 🟢 Working | Live directory browser for `/data`, `/etc`, `/tmp`, `/data/app` |
| **Settings App** | 🟢 Working | 8 sections (Network, Sound, Display, Security, Battery, Storage, etc.) |
| **Permission Broker** | 🟢 Working | JSON-persisted grant/revoke with 7-day auto-revoke |
| **System Supervision** | 🟢 Working | `nilinit` supervises 11 registered daemons with socket activation |
| **Namespace Sandbox** | 🟢 Working | Real `unshare(CLONE_NEWPID\|CLONE_NEWNS\|CLONE_NEWIPC\|CLONE_NEWUTS)` + `pivot_root`/`chroot` |
| **Seccomp BPF Filter** | 🟢 Working | Real `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)` — ~110-syscall allowlist |
| **SoftBus Distributed Mesh** | 🟢 Working | Real mDNS-SD peer discovery + quinn QUIC/TLS 1.3 transport |
| **Android Container Agent** | 🟢 Working | JSON protocol dispatcher wrapping `am start`/broadcast/`pm list` |
| **Package Manager (`nilpkg`)** | 🟡 Prototype | Atomic install to `/data/app/`; SHA-256 signing **not yet implemented** (custom FNV hash placeholder) |
| **Shell Compositor** | 🟡 Prototype | ANSI/minifb pixel-buffer renderer — not a Wayland compositor |
| **ARM64 Device Port** | 🟡 In Progress | NilHAL GKI/Treble abstraction skeleton; PinePhone/Pixel 3a target |
| **Android Compatibility** | 🟡 In Progress | LXC/Waydroid container architecture + binder-shim agent (container not yet provisioned) |


---

## 🗺️ 4-Phase Roadmap

To ensure maintainability and realistic engineering progress, NilOS is divided into four sequential phases:

```
┌─────────────────────────────────────────────────────────────┐
│  Phase 1: "It Boots" ✅ COMPLETE                             │
│  Linux kernel → nilinit (PID 1) → filesystems → services    │
│  → nilshell ANSI compositor → QEMU graphical boot           │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 2: "It is a Usable OS" ✅ COMPLETE                   │
│  Persistent /data disk · OOBE Wizard · PIN Lockscreen        │
│  Home Launcher · Phone · Messages · Files · Settings ·      │
│  NilPkg · SoftBus · Android Dashboard · Terminal            │
└──────────────────────────────┬──────────────────────────────┘
                               ▼ (NEXT)
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 3: "It is a Mobile OS"                               │
│  ARM64 target device · Display · GPU · Camera · Audio ·     │
│  Wi-Fi · Bluetooth                                          │
└──────────────────────────────┬──────────────────────────────┘
                               ▼
┌─────────────────────────────────────────────────────────────┐
│  Phase 4: "Android Compatibility"                           │
│  Containerized Android runtime (LXC/Waydroid) · binder-shim │
│  · microG integration                                       │
└─────────────────────────────────────────────────────────────┘
```

---

## 📚 Documentation

The technical architecture is documented in modular specifications under [`docs/`](docs/):

- **[Architecture](docs/architecture.md)** — Architectural layers, memory-safety principles, and IPC design.
- **[Boot Process](docs/boot-process.md)** — Verified boot, A/B partition layout, `nilinit` PID 1, and socket activation.
- **[Security](docs/security.md)** — SELinux CIL policy, namespace sandboxing, `fscrypt v2` encryption, and zero-telemetry charter.
- **[UI System](docs/ui-system.md)** — NilUI declarative framework, `nilui-gpu` Vulkan renderer, and `nilshell` Wayland compositor.
- **[Hardware Support](docs/hardware-support.md)** — Linux LTS baseline, Android GKI/Treble, Halium/libhybris bridging, and driver strategy.
- **[Android Compatibility](docs/android-compatibility.md)** — Headless container approach, binder-shim translation, and lifecycle management.
- **[Roadmap & Milestones](docs/roadmap.md)** — Detailed milestone deliverables, team budgeting, and governance.

*(For historical deep-dive reference, the complete master blueprint is preserved in [blueprint.md](blueprint.md).)*

---

## 📁 Repository Structure

```
nilos/
├── Cargo.toml                  # Cargo Workspace configuration
├── build/                      # Build, Toolchain, Image, Installer & OTA Scripts
├── kernel/                     # Kernel defconfig fragments (Base, x86, Halium)
├── hal/                        # Hardware Abstraction Layer (C-ABI & Drivers)
├── nilinit/                    # PID 1 System Init & Supervised Socket Activation
├── runtime/
│   ├── nilsd/                  # Socket activation helper library
│   ├── nilhal/                 # Safe dlopen HAL loader & diagnostic CLI
│   ├── nilrt/                  # Namespace Sandbox, seccomp BPF, permbroker
│   ├── nilui/                  # Declarative reactive UI framework & animations
│   ├── nilui-gpu/              # Vulkan 2D renderer, SDF rects & HarfBuzz shaping
│   └── nilbus-client/          # SoftBus P2P IPC client library
├── shell/                      # ANSI/minifb Mobile Shell Compositor (nilshell)
├── softbus/                    # Distributed SoftBus daemon (mDNS-SD + QUIC/TLS 1.3)
├── pkg/nilpkg/                 # Atomic Package Manager (signing: planned)
├── services/                   # System Daemons (power, telephony, fscrypt, notify, IME, etc.)
├── android/                    # Android compatibility layer & in-container agent
├── apps/                       # Native NilOS Applications & Demos
├── security/selinux/           # Comprehensive SELinux CIL Security Policies & CI
├── etc/nilos/                  # System services configuration & design tokens
└── docs/                       # Modular Developer & Architecture Documentation
```

---

## 🚀 Building & Running NilOS

### Quick Boot in QEMU (Phase 1)

NilOS can be built and booted in QEMU on Windows, Linux, or macOS:

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

NilOS is distributed under the **[GNU General Public License v3.0 (GPLv3)](LICENSE)**.

> **Copyleft Protection**: Anyone is free to use, modify, contribute to, and build derivative operating systems from NilOS, provided that all modifications and derivative systems remain **100% free and open-source under the GNU GPLv3 license**.

