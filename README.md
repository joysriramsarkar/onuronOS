# 📘 Onuron OS (অনুরণ ওএস)

> **A lightweight, secure, Linux-based mobile operating system powered by the Alap (আলাপ) cross-platform framework, with a 100% Rust native userspace, native NilLang (.nil) app ecosystem (.nilax), and containerized Android compatibility.**

Onuron OS combines the reliability of the Linux LTS kernel, the memory safety and efficiency of a 100% Rust native userspace (PID 1 `nilinit`, NilHAL, system daemons, `nilrt` runtime, and declarative UI shell; Android host lab integration uses a thin Java/JNI bridge), and an isolated containerized Android compatibility layer—built with a bloat-free, zero-telemetry philosophy.

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

<!-- BEGIN GENERATED: maturity -->
| Subsystem / Feature | Maturity | Details & Reality |
|---|---|---|
| **Linux Kernel Boot** | 🔵 Functional prototype | Linux LTS 6.6 x86_64, bootable under QEMU with initramfs |
| **System Init (`nilinit`)** | 🔵 Functional prototype | PID 1 init, clean `[  OK  ]` boot logging, mounts, supervision, socket activation |
| **Storage Hierarchy** | 🔵 Functional prototype | `/data` ext4 persistent disk on virtio-blk + tmpfs fallback; mobile layout |
| **QEMU Boot Automation** | 🔵 Functional prototype | Persistent `nilos.img` disk + virtio-blk + automated headless boot smoke testing harness |
| **DRM/KMS Direct Compositor (`nilui-gpu`)** | 🔵 Functional prototype | Direct `/dev/dri/card0` modesetting, dumb buffer allocation, 120Hz/60Hz triple buffering, 2D rasterizer, interactive touch compositor, bootsplash & presentation demo |
| **First-Boot Setup (OOBE)** | 🔵 Functional prototype | Name & PIN setup wizard, writes configuration to `/data/config/` |
| **Lock Screen** | 🔵 Functional prototype | Salted + stretched (100k-round SHA-256) PIN record with constant-time verify; displayed clock/date/weather are static demo values **[SIMULATED]** |
| **Home Launcher** | 🔵 Functional prototype | App grid, status bar, notification shade; hero clock/date and status-bar battery/signal are static demo values **[SIMULATED]** |
| **Phone App** | 🟠 Stub / simulated | Dialer UI & contact list functional; **simulated VoLTE** (real modem AT layer planned); recent-call list is fabricated **[SIMULATED]** |
| **Messages App** | 🔵 Functional prototype | SMS message threads, composer, persistent storage under `/data/sms/`; ships with seeded demo threads **[SIMULATED]** |
| **Files App** | 🔵 Functional prototype | Directory explorer for `/data`, `/etc`, `/tmp`, `/data/app` |
| **Settings App** | 🔵 Functional prototype | System settings UI for Network, Display, Security, Battery, Storage; security/status lines (SELinux, fscrypt) are unverified **[SIMULATED]** |
| **Notification Center** | 🟠 Stub / simulated | Notification shade renders a fabricated notification history (real SMS threads are mixed in) **[SIMULATED]** |
| **Permission Broker** | 🔵 Functional prototype | JSON-persisted grants with 7-day auto-revoke; atomic writes and corrupt-database quarantine; clock-skew safe |
| **Namespace Sandbox** | 🔵 Functional prototype | `unshare(CLONE_NEWPID|CLONE_NEWNS|CLONE_NEWIPC|CLONE_NEWUTS)` + private mount propagation + `chroot`/`pivot_root` with `chdir("/")` escape fix |
| **Seccomp BPF Filter** | 🔵 Functional prototype | Real `prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER)` syscall allowlist (~110 syscalls) |
| **SoftBus Distributed Mesh** | 🔵 Functional prototype | Real mDNS-SD peer discovery + Quinn QUIC/TLS 1.3 transport; the UI's discovered-peer list is fabricated **[SIMULATED]** |
| **Input Daemon (`inputd`)** | 🔵 Functional prototype | Linux `evdev` & Android Host bridge with gesture recognition (Tap, DoubleTap, LongPress, Swipe, Drag, Pinch); not yet verified on a physical target |
| **Power Daemon (`powerd`)** | 🔵 Functional prototype | NilHAL sysfs & Android BatteryManager bridge, wakelock governor, screen timeout, performance modes |
| **Network Daemon (`netd`)** | 🔵 Functional prototype | NilHAL Linux sysfs & Android ConnectivityManager bridge, link status, DNS, and network IPC |
| **Native Language & Platform (`nillang` / `nilc`)** | 🔵 Functional prototype | Native NilLang parser, AST, portable bytecode compiler (`nilc`), declarative UI engine (`NilVM`), and end-to-end integration with `nilpkg` & `nilrt` |
| **Cryptographic Verified Boot (`mkvbmeta`)** | 🔵 Functional prototype | DM-verity root hash tree generation and Ed25519 signature signing & validation tool (`build/mkvbmeta.py`) |
| **Package Manager (`nilpkg`)** | 🔵 Functional prototype | Signed `.nilax` package format (`nilpkg pack`), atomic unpack & install, upgrade with rollback, crash-recovery journal, cross-process locking, key revocation, and shell integration; store catalogue simulated **[SIMULATED]** |
| **Canonical Framed IPC (`nilprotocol`)** | 🔵 Functional prototype | Length-prefixed binary wire frame (`ONUR` magic, versioned headers, bounded 1 MiB payloads) wired across core daemons (`powerd`, `inputd`, `netd`, `audiod`, `nilimed`, `nilupd`, `nilkeyd`, `nilandroidd`, `btd`, `telephonyd`, `camerad`) |
| **Audio Policy Server & Mixer (`audiod`)** | 🔵 Functional prototype | Stream focus arbitration (EmergencyCall > VoiceCall > Alarm > Notification > Media), dynamic volume ducking, ALSA PCM routing (Speaker, Earpiece, Headset, Bluetooth), and canonical framed IPC (`/run/onuron/audio.sock`) |
| **Camera HAL & Streaming Daemon (`camerad`)** | 🔵 Functional prototype | Linux V4L2 & vendor HAL sensor discovery, image capture pipeline with frame buffers, LED torch control, preview state management, and canonical framed IPC (`/run/nilos/camera.sock`) |
| **Bluetooth Subsystem (`btd`)** | 🔵 Functional prototype | Linux sysfs `/sys/class/bluetooth/` & RFKILL adapter discovery, device pairing state, scan discovery, and canonical framed IPC (`/run/nilos/bt.sock`) |
| **Cellular Telephony Daemon (`telephonyd`)** | 🔵 Functional prototype | Modem detection via sysfs & dev nodes (`/dev/ttyUSB*`, `/dev/cdc-wdm*`, `/sys/class/net/wwan*`), SIM state management, VoLTE registration, call lifecycle, SMS store, and framed IPC (`/run/nilos/telephony.sock`) |
| **Android Boot Image Tooling (`mkbootimg.py`)** | 🔵 Functional prototype | Pure-Python Android boot image (boot.img) packager & unpacker supporting headers v0-v4, DTB & initramfs bundling, reproducible builds, and fastboot flashing |
| **Hardware Watchdog (`nilwdt`)** | 🔵 Functional prototype | Feeds `/dev/watchdog` with configurable interval/timeout, subsystem health monitoring, trip-on-hang, and graceful disarm |
| **Android Compatibility Layer** | 🟠 Stub / simulated | Android screen shows placeholder container state; guest container isolation verified (namespaces, cgroups, device whitelist); no LXC/Waydroid container is launched from the UI yet **[SIMULATED]** |
| **Diagnostic Terminal (`nilshell`)** | 🟠 Stub / simulated | `ps`, `services` and `net` output is fabricated; `ls`/`cat`/`mem` read real kernel/filesystem data **[SIMULATED]** |
| **NilHAL Unified Subsystem** | 🔵 Functional prototype | Rust trait abstraction with mock in-memory `FakeHAL` for automated CI, plus scaffold backends for Linux sysfs, evdev, and Android host |
| **S25 Hosted Mobile Runtime** | 🔵 Functional prototype | Samsung Galaxy S25 (Snapdragon 8 Elite) 120Hz AMOLED runtime & JNI bridge (`android-host/`) |
| **Operating Modes (1, 2, 3)** | 🟡 Experimental | Mode 1 (QEMU) working; Mode 2 (Galaxy S25 Hosted Lab) in progress; Mode 3 (Future Bare Metal Phone) not implemented |
| **Status Bar (simulated indicators)** | 🟠 Stub / simulated | Battery, signal and clock indicators are demo values, marked `⚠SIM` on screen across the shell **[SIMULATED]** |
<!-- END GENERATED: maturity -->
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
