# 🏛️ NilOS Architecture Specification

NilOS is structured as a modern, layered mobile operating system engineered for memory safety, security isolation, and hardware portability.

---

## 1. High-Level Architectural Stack

```
┌─────────────────────────────────────────────────────────────┐
│                       NilOS Apps                            │
│   Native Apps (NilUI/Rust)  ·  PWA / Web  ·  Android Apps   │
├─────────────────────────────────────────────────────────────┤
│                    App Runtime & Framework                  │
│   NilUI Declarative Engine · Sandboxing (namespaces/seccomp)│
│       Permission Broker · nilpkg Package Manager            │
├─────────────────────────────────────────────────────────────┤
│                       UI Shell                              │
│   nilshell (Wayland wlroots) · SoftBus Distributed Surface  │
│        Gesture Engine · Convergence (Mobile → Desktop)      │
├─────────────────────────────────────────────────────────────┤
│                     Core System Services                    │
│   nilinit (PID 1) · nild · nilkeyd (fscrypt) · nilbus (P2P) │
│       PipeWire (Audio) · iwd (Wi-Fi) · oFono (Telephony)    │
├─────────────────────────────────────────────────────────────┤
│               Hardware Abstraction Layer (HAL)              │
│       NilHAL C-ABI · Safe Rust dlopen · Halium Bridge       │
├─────────────────────────────────────────────────────────────┤
│                      Linux LTS Kernel                       │
│    GKI Baseline · Android Treble Drivers · SELinux Enforcing│
└─────────────────────────────────────────────────────────────┘
```

---

## 2. Core Architectural Principles

### 1. Memory Safety by Default
All userspace services, daemons, runtimes, package management, and system init are authored in **Rust**. This eliminates over 70% of historical operating system vulnerabilities (buffer overflows, use-after-free, double frees, data races) without requiring heavy garbage collection.

### 2. Micro-Service Supervision
Unlike monolithic init systems or heavy Java frameworks, NilOS uses `nilinit`—a lean PID 1 supervisor managing lightweight independent daemons. Daemons communicate across local UNIX domain sockets with support for **socket activation**, allowing background services to stay idle until requested.

### 3. Separation of Concerns & Portability
The system strictly separates hardware drivers from the userspace through the **NilHAL** interface. Driver vendors or legacy Android BSPs (Board Support Packages) interface through well-defined C-ABI boundaries or `libhybris` bridges, preserving system compatibility across kernel upgrades.

### 4. Distributed Multi-Device Fabric (SoftBus)
Cross-device capabilities are baked into the protocol layer rather than bolted on through proprietary cloud backends. Near-field P2P discovery (mDNS, BLE, Wi-Fi Aware) and encrypted QUIC streams enable hardware-to-hardware handoff, shared clipboards, and remote displays.

---

## 3. Userspace Workspace Layout

The NilOS codebase is organized as a unified Cargo workspace:

- `nilinit`: PID 1 supervisor, virtual filesystem initialization, SELinux policy loader.
- `services/nild`: Core system daemon for power governance, hardware status, and radio coordination.
- `services/nilkeyd`: Hardware-backed fscrypt v2 encryption key lifecycle daemon.
- `softbus`: Distributed device discovery daemon (mDNS + QUIC TLS).
- `shell`: wlroots-based 120Hz Wayland display server (`nilshell`).
- `runtime/nilrt`: Process isolation sandbox with Linux namespaces, seccomp BPF filters, and permission broker.
- `runtime/nilui`: Reactive declarative UI framework with spring physics.
- `runtime/nilui-gpu`: High-performance Vulkan 2D graphics engine.
- `pkg/nilpkg`: Reproducible, cryptographically signed atomic package manager.
