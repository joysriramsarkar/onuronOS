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
| **Linux Kernel Boot (x86_64)** | 🔵 Functional prototype | Linux LTS 6.6 x86_64, bootable under QEMU with initramfs; boot completes in ~200ms with 7 core services active |
| **System Init (nilinit)** | 🔵 Functional prototype | PID 1 init, mounts /proc /sys /dev /run, cgroups v2, SELinux, supervision, socket activation, recovery path |
| **Storage Hierarchy** | 🔵 Functional prototype | /data ext4 persistent disk on virtio-blk + tmpfs fallback; mobile layout (/system, /vendor, /data/app, /data/system, /data/config) |
| **QEMU Boot Automation** | 🔵 Functional prototype | Headless QEMU boot smoke test, persistent data disk, two-phase reboot persistence test |
| **DRM/KMS Direct Compositor (nilui-gpu)** | 🔵 Functional prototype | Real /dev/dri/card* ioctl path: card enumeration, GETRESOURCES/GETCONNECTOR/GETENCODER/GETCRTC discovery, preferred-mode selection, CREATE_DUMB+MAP_DUMB+ADDFB2 scanout buffers, SETCRTC, and VBLANK-synchronised page flips that block on DRM_EVENT_FLIP_COMPLETE; plus a 2D rasterizer, touch compositor, bootsplash and presentation demo. ioctl request numbers and UAPI struct sizes are asserted against the kernel headers. NOT yet validated on physical hardware: no DRM node exists in the dev container, so the panel path is unproven |
| **Wayland Compositor (nilui-gpu, hand-rolled protocol)** | 🔵 Functional prototype | A Wayland server written directly against the wire protocol (no libwayland): message framing, wl_display/wl_registry/wl_compositor/wl_surface/wl_shm/wl_shm_pool/wl_buffer/wl_callback/wl_output/wl_seat/wl_touch, SCM_RIGHTS descriptor passing, memfd pools, and wl_display.error before close. Committed surfaces are composited on the CPU (scale-to-fit, source-over alpha, fullscreen/tiled layouts) into the existing DRM/KMS present path, with an eventfd wake so a presenter blocks instead of polling. Verified by 101 in-crate tests plus a cross-process test that runs the real `wayland_compositor` and `wayland_surface_producer` binaries against each other over a real socket (and that test was confirmed to fail when compositing is disabled). NOT implemented: linux-dmabuf/GPU buffer import, wl_region clipping, wl_surface damage-limited redraw, pointer/keyboard events, subsurfaces, and xdg-shell. No physical panel has run this |
| **First-Boot Setup (OOBE)** | 🔵 Functional prototype | Name & PIN setup wizard, writes configuration to /data/config |
| **Lock Screen (PIN)** | 🔵 Functional prototype | Salted + stretched (100k-round SHA-256) PIN record with constant-time verify and live system clock/date |
| **Shell Design System (OnuronOS 1.7)** | 🔵 Functional prototype | Sampled dark/light tokens, measured 390x796 metrics, vector icon set and shared widgets; 33 reference screens implemented in the simulator, validated by headless export and a behaviour selftest |
| **Home Launcher** | 🔵 Functional prototype | 4-column native grid, live clock/date, live package/note status pill, Android launcher section, status-bar entry points, app drawer for the remaining native apps |
| **Phone App** | 🔵 Functional prototype | Dialer UI, persistent contacts (/data/contacts/) and call history (/data/calls/); VoLTE call layer uses simulated fallback when modem hardware is absent [SIMULATED] **[SIMULATED]** |
| **Messages App** | 🔵 Functional prototype | SMS message threads, composer, reply & delete with atomic persistent storage under /data/sms/ |
| **Files App** | 🔵 Functional prototype | Directory explorer for /data, /etc, /tmp, /data/app |
| **Settings App** | 🔵 Functional prototype | System settings UI for Network, Sound (nilaudio), Display, Security (PIN change), Battery, Storage usage, and Accessibility (i18n); SELinux/fscrypt report simulated fallback when unmounted [SIMULATED] **[SIMULATED]** |
| **Notification Center** | 🔵 Functional prototype | Persistent notification store (/data/notifications/notifications.json) wired to notifyd (/run/nilos/notify.sock) with post, dismiss, clear, and SMS integration |
| **Permission Broker** | 🔵 Functional prototype | JSON-persisted grants with 7-day auto-revoke; atomic writes and corrupt-database quarantine; clock-skew safe |
| **Namespace Sandbox** | 🔵 Functional prototype | unshare(CLONE_NEWPID|CLONE_NEWNS|CLONE_NEWIPC|CLONE_NEWUTS) + private mount propagation + chroot/pivot_root with chdir(/) escape fix |
| **Seccomp BPF Filter** | 🔵 Functional prototype | Real prctl(PR_SET_SECCOMP, SECCOMP_MODE_FILTER) syscall allowlist (~110 syscalls) |
| **SoftBus Distributed Mesh** | 🔵 Functional prototype | Real mDNS-SD peer discovery + Quinn QUIC/TLS 1.3 transport; UI peer list is simulated [SIMULATED] **[SIMULATED]** |
| **Input Daemon (inputd)** | 🔵 Functional prototype | Linux evdev & Android Host bridge with gesture recognition (Tap, DoubleTap, LongPress, Swipe, Drag, Pinch); not yet verified on physical target |
| **Power Daemon (powerd)** | 🔵 Functional prototype | NilHAL sysfs & Android BatteryManager bridge, wakelock governor, screen timeout, performance modes. Battery telemetry is served over the versioned `nilprotocol` frame contract (`PowerGetBattery`/`PowerBatteryInfo`) and consumed via `runtime/nilpower`; the daemon reports a failed IPC bind as `[ FAIL ]` rather than a false OK |
| **Network Daemon (netd)** | 🔵 Functional prototype | NilHAL Linux sysfs & Android ConnectivityManager bridge, link status, DNS, and network IPC. Interface state is served over the versioned `nilprotocol` frame contract (`NetGetState`/`NetStateInfo`) and consumed via `runtime/nilnet`; the daemon reports a failed IPC bind as `[ FAIL ]` rather than a false OK |
| **Native Language & Platform (nillang / nilc)** | 🔵 Functional prototype | Native NilLang parser, AST, portable bytecode compiler (nilc), declarative UI engine (NilVM), and end-to-end integration with nilpkg & nilrt |
| **Cryptographic Verified Boot (mkvbmeta)** | 🔵 Functional prototype | DM-verity root hash tree generation and Ed25519 signature signing & validation tool |
| **Package Manager (nilpkg)** | 🔵 Functional prototype | Signed .nilax package format (nilpkg pack), atomic unpack & install, upgrade with rollback, uninstall, crash-recovery journal, cross-process locking, key trust/revocation, and shell integration |
| **Canonical Framed IPC (nilprotocol)** | 🔵 Functional prototype | Length-prefixed binary wire frame (ONUR magic, versioned headers, bounded 1 MiB payloads) wired across core daemons |
| **Audio Policy Server & Mixer (audiod)** | 🔵 Functional prototype | Stream focus arbitration (EmergencyCall > VoiceCall > Alarm > Notification > Media), dynamic volume ducking, ALSA PCM routing (Speaker, Earpiece, Headset, Bluetooth); audio output simulated when ALSA hardware is absent [SIMULATED] **[SIMULATED]** |
| **Camera HAL & Streaming Daemon (camerad)** | 🔵 Functional prototype | Linux V4L2 & vendor HAL sensor discovery, image capture pipeline with frame buffers, LED torch control, preview state management; frame buffer simulated when V4L2 hardware is absent [SIMULATED] **[SIMULATED]** |
| **Bluetooth Subsystem (btd)** | 🔵 Functional prototype | Linux sysfs /sys/class/bluetooth/ & RFKILL adapter discovery, device pairing state, scan discovery |
| **Cellular Telephony Daemon (telephonyd)** | 🔵 Functional prototype | Modem detection via sysfs & dev nodes (/dev/ttyUSB*, /dev/cdc-wdm*), SIM state management, VoLTE registration, call lifecycle, SMS store; call layer simulated when modem hardware is absent [SIMULATED] **[SIMULATED]** |
| **Android Boot Image Tooling (mkbootimg.py)** | 🔵 Functional prototype | Pure-Python Android boot image (boot.img) packager & unpacker supporting headers v0-v4, DTB & initramfs bundling, reproducible builds, and fastboot flashing |
| **Hardware Watchdog (nilwdt)** | 🔵 Functional prototype | Feeds /dev/watchdog with configurable interval/timeout, subsystem health monitoring, trip-on-hang, and graceful disarm |
| **Android Compatibility Layer** | 🔵 Functional prototype | Real container lifecycle in nilandroidd: rootfs probe requires an executable Android init, launch performs unshare(PID|NS|IPC|UTS|NET) + pivot_root + fresh /proc,/sys,/dev with /system,/vendor,/data bind-mounted from container storage, cgroup v2 limits applied to the real init PID; state is derived from the live child process and reported as unavailable when no rootfs is installed |
| **Android screen (nilshell)** | 🟠 Stub / simulated | The screen reports the real container state from nilandroidd, but no guest Android userspace or graphics passthrough is bundled, so no Android UI can be shown [SIMULATED] **[SIMULATED]** |
| **Diagnostic Terminal (nilshell)** | 🔵 Functional prototype | Real command execution: builtins run in-process (cd/pwd/export/unset/echo/which/clear/exit), every other command is resolved against $PATH and spawned as a child process with stdin/stdout/stderr wired, output capped at 64 KiB and commands killed after 20 s; exit status is reported verbatim |
| **NilHAL Unified Subsystem** | 🔵 Functional prototype | Rust trait abstraction with mock in-memory FakeHAL for automated CI, plus scaffold backends for Linux sysfs, evdev, and Android host |
| **S25 Hosted Mobile Runtime** | 🔵 Functional prototype | Samsung Galaxy S25 (Snapdragon 8 Elite) 120Hz AMOLED runtime & JNI bridge (android-host/) |
| **Operating Modes (1, 2, 3)** | 🟡 Experimental | Mode 1 (QEMU) working; Mode 2 (Galaxy S25 Hosted Lab) in progress; Mode 3 (Future Bare Metal Phone) not implemented |
| **Status Bar (service-backed telemetry)** | 🔵 Functional prototype | Battery, Wi-Fi/cellular and clock indicators are read from the owning services (`powerd` via `nilpower`, `netd` via `nilnet`), not from `/sys` directly. Each indicator falls back to a visibly-marked `[SIMULATED]` value only when its service is unreachable, and the cellular bars honour the local airplane-mode toggle when no modem hardware exists |
| **Alap Framework (alap)** | 🔵 Functional prototype | Declarative mobile application framework (state reactivity, component graph, lifecycle hooks, router stack, typed system service contracts) adhering to ADR-0009 |
| **Canonical Target Registry** | 🔵 Functional prototype | ADR-0002 canonical target registry and fail-closed flashing safety gates (qemu-x86_64, qemu-aarch64, android-host-arm64, oneplus-fajita) |
| **Calculator App** | 🔵 Functional prototype | Scientific arithmetic evaluator (precedence, parentheses, exponents, modulo, trig/log/sqrt, Bengali digits, persistent history) with interactive UI |
| **Notes App** | 🔵 Functional prototype | Persistent notebook (/data/notes/notes.json) with atomic writes, creation, editing, tagging, priority, search, and deletion |
| **Music Player App** | 🔵 Functional prototype | Filesystem audio scanner (/data/music), playlist persistence, and audiod (nilaudio) volume/mute/route control; audio output simulated when ALSA hardware is absent [SIMULATED] **[SIMULATED]** |
| **Camera Viewfinder App** | 🔵 Functional prototype | Viewfinder UI wired to nilcam::CameraClient (get_info, capture_frame to /data/dcim, set_torch, sensor switch); frame buffer simulated when V4L2 hardware is absent [SIMULATED] **[SIMULATED]** |
| **SoftBus Test App** | 🔵 Functional prototype | Demonstration of SoftBus mesh capabilities (busdemo, bridgedemo, animdemo) |
| **Browser (native web engine)** | 🔵 Functional prototype | Real HTTP/HTTPS fetch over ureq/rustls with a 20 s timeout and 512 KiB body cap; HTML reduced to text (script/style contents dropped, block tags become line breaks, entities decoded), title extraction, bounded scrolling; CSS, JavaScript and images are not rendered and the UI says so. One engine (runtime/nilweb) backs both the shell Browser screen and the standalone `browser` app |
| **NilBrowser (simulator host webview)** | 🟡 Experimental | Windows-only WebView2 (Evergreen Chromium) embed used by the desktop simulator host; not part of the mobile native platform and never built on Linux/ARM [SIMULATED] **[SIMULATED]** |
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

Onuron OS is distributed under the **[GNU Affero General Public License v3.0 (AGPLv3)](LICENSE)**.

> **Copyleft Protection**: Anyone is free to use, modify, contribute to, and build derivative operating systems from Onuron OS, provided that all modifications and derivative systems remain **100% free and open-source under the AGPLv3 license** — including when the software is offered to users over a network, in which case the corresponding source must be made available. Commercial use is permitted under these terms; see the ecosystem-wide policy at [onuron_project/docs/LICENSING.md](https://github.com/joysriramsarkar/onuron_project/blob/main/docs/LICENSING.md).
