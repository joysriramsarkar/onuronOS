# 🗺️ Onuron OS Phased Roadmap & Milestone Deliverables

Onuron OS adheres to a strict phased engineering approach designed to maximize maintainability, focus resources, and deliver testable, reproducible vertical slices.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

---

## 1. 6-Phase Execution Roadmap

```
Phase 1: "Bootable Prototype"
├── Goal: Reliable QEMU boot with kernel, init, virtual filesystems, and early console
└── Status: 🔵 COMPLETE (x86_64 QEMU)

Phase 2: "Usable Prototype & Shell Architecture"
├── Goal: Persistent /data disk, OOBE wizard, PIN lockscreen, home launcher, SoftBus P2P
└── Status: 🔵 COMPLETE (ANSI/minifb prototype)

Phase 3: "Real Hardware & Core Subsystems" (CURRENT FOCUS)
├── Goal: DRM/KMS framebuffer, evdev input daemon, power & battery daemon, network abstraction,
│         Ed25519 signed nilpkg, ARM64 QEMU & PinePhone target
└── Status: 🟡 ACTIVE ENGINEERING

Phase 4: "Native Application Platform"
├── Goal: Native NilLang (.nil) apps, Alap framework, NilUI declarative engine,
│         .nilax package signature verification, per-UID sandboxing
└── Status: 🟡 IN PROGRESS

Phase 5: "Android Compatibility"
├── Goal: Containerized Android runtime (LXC/Waydroid) + binder-shim IPC
└── Status: 🟠 PLANNED

Phase 6: "Production Hardening"
├── Goal: Verified Boot, A/B OTA atomic updates, fscrypt v2 encryption, SELinux Enforcing,
│         Automated security & boot CI test matrix
└── Status: 🔴 FUTURE
```

---

## 2. Phase Deliverables & Scope

### Phase 1: "Bootable Prototype" ✅
- **Linux LTS Kernel**: Clean boot configuration for x86_64 virtualization.
- **`nilinit` (PID 1)**: Virtual filesystem initialization (`/proc`, `/sys`, `/dev`, `/run`, `/tmp`), cgroups v2, standardized `[  OK  ]` boot logging, and service supervision.
- **Configuration Engine**: Parsing `/etc/nilos/services.toml` and starting core daemons (`nild`, `nilkeyd`, `nilbus`).
- **QEMU Automation**: Single-command builds and instant boots on Windows (`build/qemu-boot.ps1`) and Linux (`build/qemu-boot.sh`).

### Phase 2: "Usable Prototype & Shell Architecture" ✅
- **Storage Persistence**: Mounting ext4 `/data` on `/dev/vda` with fallback to tmpfs.
- **OOBE First-Boot**: Setup wizard writing PIN and user info to `/data/config/`.
- **Lockscreen & Launcher**: PIN unlock screen, status bar, and app launcher.
- **SoftBus Mesh**: Distributed device discovery via mDNS-SD + Quinn QUIC/TLS 1.3 transport.

### Phase 3: "Real Hardware & Core Subsystems" (Current Priority)
- **DRM/KMS Display Pipeline**: Direct `/dev/dri/card0` dumb-buffer framebuffer management and double-buffering page flips.
- **Unified Input Subsystem (`inputd`)**: Linux `/dev/input/event*` (`evdev`) event reader for multitouch, power key, volume keys, and IPC dispatch.
- **Power Management (`powerd`)**: Reading `/sys/class/power_supply/`, tracking battery capacity/temperature, handling wakelocks, and screen timeout.
- **Network Management (`netd`)**: Monitoring `/sys/class/net/` interfaces, carrier states, IP assignment, and DNS servers.
- **Cryptographic Package Security (`nilpkg`)**: Ed25519 digital signature signing and SHA-256 integrity verification for `.nilax` packages.
- **ARM64 Reference Port**: QEMU aarch64 (`-M virt`) and PinePhone (Allwinner A64) single-target focus.

### Phase 4: "Native Application Platform"
- **NilLang Native Apps**: First-class execution of compiled `.nil` applications.
- **Alap Framework & NilUI**: Hardware-accelerated UI components and layout engine.
- **Per-UID App Sandboxing**: Dedicated mount namespaces, seccomp BPF filters, and `/data/user/<uid>/` isolation.
- **Standardized Native APIs**: `onuron.app`, `onuron.ui`, `onuron.storage`, `onuron.network`, `onuron.power`.

### Phase 5: "Android Compatibility"
- **Container Infrastructure**: Unprivileged container configuration for headless AOSP rootfs.
- **`binder-shim`**: Translating Android Binder transactions into native Onuron OS handlers.
- **Wayland Surface Passthrough**: Routing Android application graphical buffers to `nilshell`.
- **MicroG Setup**: Open-source location and notification services.

### Phase 6: "Production Hardening"
- **Hardware-Rooted Verified Boot**: Cryptographic chain of trust from Bootloader to Kernel and Init.
- **A/B Partitioning & Atomic OTA**: Streaming signed image updates with automatic rollback on boot failure.
- **Fscrypt v2**: Hardware-backed per-user credential encryption.
- **Automated Boot & Security CI**: Continuous QEMU integration boot test on every git commit.

---

## 3. Subsystem Implementation Matrix (Implemented vs. Planned)

| Subsystem | Implemented | In Progress | Planned |
|---|---|---|---|
| **Display** | ANSI console renderer, minifb pixel buffer | DRM/KMS dumb buffers, frame flipping | Vulkan GPU compositor, Wayland IPC |
| **Input** | Terminal stdin reading | `services/inputd` evdev reader, touch coords | Multitouch gestures, virtual keyboard IME |
| **Power** | Basic daemon stub | `/sys/class/power_supply` reader, wakelocks | Suspend-to-RAM (`/sys/power/state`), CPU governor |
| **Network** | Stub daemon | Interface & carrier monitor (`netd`) | wpa_supplicant Wi-Fi IPC, cellular modem AT |
| **Packages** | Atomic install to `/data/app` | Ed25519 signatures, SHA-256 verification | Repository sync, delta updates, permission bind |
| **Security** | Namespace sandbox, Seccomp allowlist | UID isolation `/data/user/<uid>` | SELinux Enforcing CI, Verified Boot |

---

## 4. Governance & Open-Source Principles

- **Copyleft Guarantee**: Onuron OS is licensed under **GNU GPLv3**. All derivatives and vendor modifications must remain open source.
- **Community First**: Decisions are conducted via public RFCs (Request For Comments) with open technical discussion.
