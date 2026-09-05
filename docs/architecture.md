# 🏛️ Onuron OS Architecture Specification

Onuron OS is structured as a modern, layered mobile operating system engineered for memory safety, security isolation, and hardware portability.

Official architectural ecosystem:
> **"Onuron OS — powered by NilLang + Alap"**

---

## 1. High-Level Architectural Stack

```
┌─────────────────────────────────────────────────────────────┐
│                      Onuron Applications                    │
│   Native NilLang (.nilax)  ·  Alap Framework  ·  Android   │
├─────────────────────────────────────────────────────────────┤
│                    App Runtime & Framework                  │
│   NilUI Declarative Engine · Sandboxing (namespaces/seccomp)│
│       Permission Broker · nilpkg Package Manager            │
├─────────────────────────────────────────────────────────────┤
│                       UI Shell                              │
│   nilshell (Wayland / DRM KMS) · SoftBus Distributed Mesh   │
│        Input Routing Engine · Convergence (Mobile → Desktop)│
├─────────────────────────────────────────────────────────────┤
│                     Core System Services                    │
│   nilinit (PID 1) · inputd · powerd · netd · nilbus (P2P)   │
│       audiod (Audio) · btd (Bluetooth) · camerad (Camera)   │
├─────────────────────────────────────────────────────────────┤
│               Hardware Abstraction Layer (HAL)              │
│       NilHAL C-ABI · Safe Rust dlopen · Halium Bridge       │
├─────────────────────────────────────────────────────────────┤
│                      Linux LTS Kernel                       │
│    GKI Baseline · DRM/KMS Drivers · SELinux Enforcing       │
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Core Architectural Principles

### 1. Memory Safety by Default
All userspace services, daemons, runtimes, package management, and system init are authored in **Rust**. This eliminates over 70% of historical operating system vulnerabilities (buffer overflows, use-after-free, double frees, data races) without requiring heavy runtime garbage collection.

### 2. Micro-Service Supervision
Unlike monolithic init systems or heavy Java frameworks, Onuron OS uses `nilinit`—a lean PID 1 supervisor managing lightweight independent daemons. Daemons communicate across local UNIX domain sockets with support for **socket activation**, allowing background services to stay idle until requested.

### 3. Separation of Concerns & Portability
The system strictly separates hardware drivers from the userspace through the **NilHAL** interface. Driver vendors or legacy Android BSPs (Board Support Packages) interface through well-defined C-ABI boundaries or `libhybris` bridges, preserving system compatibility across kernel upgrades.

### 4. Distributed Multi-Device Fabric (SoftBus)
Cross-device capabilities are baked into the protocol layer rather than bolted on through proprietary cloud backends. Near-field P2P discovery (mDNS, BLE, Wi-Fi Aware) and encrypted QUIC streams enable hardware-to-hardware handoff, shared clipboards, and remote displays.

---

## 3. Userspace Workspace Layout

The Onuron OS codebase is organized as a unified Cargo workspace:

- `nilinit`: PID 1 supervisor, virtual filesystem initialization, SELinux policy loader, and standardized boot logs.
- `services/inputd`: Linux `evdev` event processor for multitouch, power, and volume keys.
- `services/powerd`: Linux `/sys/class/power_supply` reader, wakelock tracker, and power governor.
- `services/netd`: Network interface monitor, link state, and DNS configuration.
- `services/nild`: System coordinator daemon.
- `services/nilkeyd`: Hardware-backed fscrypt v2 encryption key lifecycle daemon.
- `softbus`: Distributed device discovery daemon (mDNS + QUIC TLS 1.3).
- `shell`: Display server and mobile UI compositor (`nilshell`).
- `runtime/nilrt`: Process isolation sandbox with Linux namespaces, seccomp BPF filters, and permission broker.
- `runtime/nilui`: Reactive declarative UI framework with spring physics.
- `runtime/nilui-gpu`: High-performance Vulkan 2D graphics engine.
- `pkg/nilpkg`: Atomic package manager with Ed25519 signatures and SHA-256 integrity verification.
