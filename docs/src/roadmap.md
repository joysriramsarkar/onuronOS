# 🗺️ NilOS Phased Roadmap & Milestone Deliverables

NilOS adopts a strict phased engineering approach designed to maximize maintainability, focus resources, and deliver testable, reproducible vertical slices.

---

## 1. Phased Execution Overview

```
Phase 1: "It Boots"
├── Goal: Reliable QEMU boot with kernel, init, and core services
└── Status: ACTIVE / WORKING

Phase 2: "It is a Usable OS"
├── Goal: Touch input, networking, audio, basic settings, package manager
└── Status: PLANNED

Phase 3: "It is a Mobile OS"
├── Goal: 1 target ARM64 real device port, native display/GPU, camera, radio
└── Status: FUTURE

Phase 4: "Android Compatibility"
├── Goal: Containerized Android runtime (LXC/Waydroid) + binder-shim
└── Status: FUTURE
```

---

## 2. Phase Deliverables & Scope

### Phase 1: "It Boots" (Current Objective)
- **Linux LTS Kernel**: Clean boot configuration for x86_64 virtualization.
- **`nilinit` (PID 1)**: Virtual filesystem initialization (`/proc`, `/sys`, `/dev`, `/run`, `/tmp`), cgroups v2, and service supervision.
- **Configuration Engine**: Parsing `/etc/nilos/services.toml` and starting core daemons (`nild`, `nilkeyd`, `nilbus`).
- **Compositor Scaffolding**: Launching `nilshell` and displaying boot banner.
- **QEMU Automation**: Single-command builds and instant boots on Windows (`build/qemu-boot.ps1`) and Linux (`build/qemu-boot.sh`).

### Phase 2: "It is a Usable OS"
- **Touch & Keyboard Input**: Wayland input processing for virtual keyboard and gestures.
- **Networking Stack**: Integration of `netd` with `iwd` (Wi-Fi) and basic DHCP client.
- **Audio Subsystem**: Integration with PipeWire for playback and volume controls.
- **Package Manager (`nilpkg`)**: Installation, verification, and sandboxed execution of native apps.
- **System Settings & OOBE**: Out-of-box initial setup wizard and configuration application.

### Phase 3: "It is a Mobile OS"
- **Target Device Selection**: Selecting a community-standard ARM64 device (e.g., Pixel 3a/4a, PinePhone Pro, or Fairphone).
- **HAL & Driver Integration**: Display, touch, battery gauge, Wi-Fi, Bluetooth, and camera drivers.
- **Power Governance**: Suspend-to-RAM, wake-locks, and deep idle management via `powerd`.
- **Telephony & SMS**: Cellular data, voice calls, and SMS via `oFono` and `ModemManager`.

### Phase 4: "Android Compatibility"
- **Container Infrastructure**: Unprivileged LXC container configuration for AOSP rootfs.
- **`binder-shim`**: Bridging Android intents to NilOS native handlers.
- **Graphics/Input Passthrough**: Routing Android surface output to `nilshell` Wayland compositor.
- **MicroG Setup**: Open-source push notifications and location providers.

---

## 3. Governance & Open-Source Principles

- **Copyleft Guarantee**: NilOS is licensed under **GNU GPLv3**. All derivatives and vendor modifications must remain open source.
- **Community First**: Decisions are conducted via public RFCs (Request For Comments) with open discussion.
