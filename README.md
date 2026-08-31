# 📘 NilOS (নীল ওএস)

**NilOS** is an open-source, bloat-free, dynamic mobile operating system combining:
- **Android**: Hardware ecosystem compatibility via Linux LTS kernel & Halium/libhybris bridges + Containerized Android app layer.
- **HarmonyOS**: Fluid 120Hz declarative UI (NilUI), physics spring animations, and Distributed SoftBus cross-device collaboration.
- **Linux**: Memory-safe Rust userspace, immutable rootfs, SELinux enforcement, and zero telemetry.

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
├── shell/                      # wlroots-based 120Hz Wayland Compositor (nilshell)
├── softbus/                    # Distributed SoftBus daemon (mDNS + QUIC TLS)
├── pkg/nilpkg/                 # Atomic, Ed25519-signed Package Manager
├── services/                   # System Daemons (power, telephony, fscrypt, notify, IME, etc.)
├── android/                    # Android compatibility layer & in-container agent
├── apps/                       # Native NilOS Applications & Demos
├── security/selinux/           # Comprehensive SELinux CIL Security Policies & CI
├── etc/nilos/                  # System services configuration & design tokens
└── docs/                       # Developer Documentation & Device Porting Guide
```

---

## 🚀 Building NilOS

```bash
# Setup build dependencies and cross toolchains
./build/setup-toolchain.sh

# Build the complete OS image for x86_64 or ARM64
./build/build.sh x86_64-generic

# Run inside QEMU with 120Hz display & hardware acceleration
./build/qemu-run.sh
```

---

## 📜 License & Open-Source Guarantee

NilOS is distributed under the **[GNU General Public License v3.0 (GPLv3)](LICENSE)**.

> **Copyleft Protection**: Anyone is free to use, modify, contribute to, and build derivative operating systems from NilOS, provided that all modifications and derivative systems remain **100% free and open-source under the GNU GPLv3 license**.
