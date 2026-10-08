# OnuronOS Canonical Context

## Identity

OnuronOS is an independent Linux-based mobile operating system.

It is **not** an AOSP fork and is not intended to be a conventional Android ROM.

Its native architecture is:

```text
Linux LTS
    ↓
nilinit
    ↓
NilHAL
    ↓
Onuron system services
    ↓
runtime/nilrt
    ↓
NilLang + Alap + NilUI
    ↓
signed native `.nilax` applications
```

Android compatibility is a separate subsystem implemented through:

```text
android/
android-host/
```

It must remain isolated from the native application architecture.

## Core identity

OnuronOS is defined by these project-owned ideas:

- Rust-first userspace
- `nilinit` as PID 1 / service supervisor
- NilHAL as the hardware boundary
- NilLang + Alap as the native developer platform
- NilUI as the native UI framework
- `.nilax` as the native application/package identity
- `nilpkg` as the native package-management layer
- `nilshell` as the native shell
- SoftBus as a local-first distributed/device layer
- security and privacy by default

## Reality rule

Repository structure must not be confused with feature maturity.

Classify features as:

```text
IMPLEMENTED
FUNCTIONAL PROTOTYPE
EXPERIMENTAL
STUB/SIMULATED
PLANNED
```

Use evidence from code, tests, build paths, and hardware validation before describing something as implemented or supported.

## Architectural boundaries

### Kernel

Linux LTS provides the kernel foundation, drivers, scheduling, memory management,
networking, filesystem facilities, and standard hardware interfaces.

### nilinit

`nilinit` is the native PID 1 and supervises the Onuron service graph.

### NilHAL

NilHAL is the hardware abstraction boundary.

Application and higher-level framework code should not contain device-specific
hardware access logic.

### System services

Core system functions should be decomposed into small, separately supervised
services with explicit IPC contracts.

### Native runtime

`runtime/nilrt` provides the native runtime/isolation boundary for applications.

Applications should receive only the permissions and capabilities they need.

### Native developer platform

NilLang + Alap + NilUI form the native development path.

The intended application flow is:

```text
NilLang source
→ Alap/framework APIs
→ build
→ `.nilax`
→ sign
→ install
→ `nilrt`
→ permissions/sandbox
→ native UI/services
```

### SoftBus

SoftBus is a local-first distributed layer for cooperating devices and services.

Security policy, capability boundaries, and explicit trust are required.

### Android compatibility

Android compatibility is an optional compatibility subsystem, not the foundation
of native OnuronOS.

Keep Android lifecycle, packaging, graphics/input bridging, and compatibility
logic behind explicit compatibility boundaries.

## Fixed choices

The project currently treats these as fixed architectural choices:

- PID 1: `nilinit`
- Linux LTS kernel
- Rust-first userspace
- NilHAL hardware boundary
- NilLang + Alap native platform
- `.nilax` native package identity
- `nilpkg`
- `nilshell`
- SoftBus
- SELinux + seccomp + namespaces as security foundations
- signed transactional / A-B update direction
- QEMU as the primary development target
- one supported reference device for hardware bring-up

## Deliberately undecided choices

These are not to be treated as hidden assumptions:

- final package compression
- final GPU backend
- final audio backend
- exact NilLang bytecode format
- complete SoftBus transport
- Android container vs VM
- commercial app-store strategy
- convergence/desktop UI strategy

Use an ADR when one of these becomes an implementation decision.

## Standard vs custom

Use standard technologies where they provide mature compatibility and security,
and keep custom engineering focused on Onuron's identity.

### Prefer standard/mature foundations

Linux kernel, DRM/KMS, evdev, ALSA, camera/media infrastructure,
cryptography/TLS, filesystem encryption, codecs/compression, hardware drivers,
and boot/security infrastructure.

### Keep Onuron-owned

`nilinit`, NilHAL interfaces, Onuron service contracts, NilLang, Alap, NilUI,
`.nilax`, `nilpkg`, SoftBus policy, native permissions, `nilshell`, and
Onuron update orchestration.

## Development order

Preferred order:

```text
architecture/status
→ reproducible QEMU
→ nilinit hardening
→ NilHAL contract
→ IPC/versioning
→ fake HAL + integration tests
→ `.nilax` signing/package flow
→ app identity + sandbox
→ one native app end-to-end
→ NilUI boundary
→ reference hardware
→ mobile hardware foundations
→ Android compatibility
→ production hardening
```

The goal is not maximum feature count. The goal is a coherent, testable native OS.
