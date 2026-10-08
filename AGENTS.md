# OnuronOS — Agent Instructions

## 1. Project identity

OnuronOS is an independent, Linux-based mobile operating system.

It is **not** an AOSP fork and must not be redesigned as a normal Android ROM
unless the maintainer explicitly requests such a change.

The native platform identity is:

Linux LTS
→ nilinit
→ NilHAL
→ Onuron system services
→ runtime/nilrt
→ NilLang + Alap + NilUI
→ signed native `.nilax` applications

Android compatibility is a separate compatibility subsystem under:
- `android/`
- `android-host/`

Do not make Android compatibility the architectural foundation of OnuronOS.

## 2. Canonical architectural rules

The project must preserve these boundaries:

- Linux LTS is the kernel foundation.
- `nilinit` is the native PID 1 / service-supervision layer.
- `NilHAL` is the hardware abstraction boundary.
- System functionality should be implemented as small, separately supervised services.
- `runtime/nilrt` is part of the native application isolation/runtime boundary.
- NilLang + Alap + NilUI are the native developer platform.
- `.nilax` is the native application/package identity.
- `nilpkg` owns native package operations.
- `nilshell` is the native shell/UI shell boundary.
- SoftBus is the local-first distributed/device layer.
- Security and privacy are defaults, not optional conveniences.

## 3. Never do these without explicit maintainer approval

1. Migrate the project to AOSP.
2. Turn OnuronOS into a conventional Android ROM.
3. Replace NilLang or Alap with generic web/PWA or Android APIs as the native platform.
4. Introduce `systemd` or `OpenRC` as a hidden dependency.
5. Assume that a directory existing in the repository means its feature is production-ready.
6. Claim hardware support without reproducible evidence.
7. Disable or bypass sandboxing, permissions, SELinux, seccomp, or namespace isolation merely to make a feature work.
8. Rewrite an existing subsystem when a project abstraction already provides an appropriate boundary.
9. Couple Android compatibility code directly into the native Onuron application model.
10. Make a major architectural change without documenting its owning layer and validation path.

## 4. Reality / maturity rule

Every feature or subsystem must be classified honestly as one of:

- `IMPLEMENTED`
- `FUNCTIONAL PROTOTYPE`
- `EXPERIMENTAL`
- `STUB/SIMULATED`
- `PLANNED`

Code presence is not proof of implementation maturity.

When uncertain, inspect the code, tests, build path, and current documentation before describing a feature as implemented.

## 5. Change discipline

Before changing architecture:

- Identify the layer that owns the change.
- Reuse existing interfaces where practical.
- Keep public/internal contracts versionable.
- Avoid leaking hardware details upward into application code.
- Add tests, fixtures, or a realistic validation procedure for new behavior.
- Update the relevant documentation/ADR when an interface or architectural decision changes.
- Keep Android compatibility isolated from native Onuron APIs.

## 6. Native application direction

The intended native flow is:

Source
→ NilLang / Alap
→ build
→ package as `.nilax`
→ sign
→ install with `nilpkg`
→ launch through `nilrt`
→ permissions / sandbox
→ NilUI / native services

Native applications should not access hardware directly. They should use stable Onuron APIs and service boundaries.

## 7. Hardware direction

The project prioritizes:

1. Reproducible QEMU development.
2. A controlled hardware bring-up path.
3. One clearly supported reference device before broad device claims.

A feature should not be marked hardware-supported merely because a driver, HAL stub, or source file exists.

## 8. Technology choices

Currently fixed architectural choices:

- PID 1: `nilinit`
- Kernel: Linux LTS
- Userspace preference: Rust-first
- Hardware boundary: NilHAL
- Native language/framework: NilLang + Alap
- Native package identity: `.nilax`
- Package manager: `nilpkg`
- Native shell: `nilshell`
- Distributed layer: SoftBus
- Security foundation: SELinux + seccomp + namespaces
- Update direction: signed transactional / A-B
- Primary development target: QEMU
- A specific reference device for hardware bring-up

These are intended to be stable unless the maintainer explicitly changes the architecture.

## 9. Deliberately undecided choices

Do not silently hard-code these as architectural facts:

- final package compression algorithm
- final GPU backend
- final audio backend
- exact NilLang bytecode format
- complete SoftBus transport choice
- Android container vs VM final decision
- commercial app store strategy
- desktop/convergence UI strategy

Record decisions about these in `docs/adr/`.

## 10. Standard vs custom

OnuronOS does not need to reimplement every foundational technology.

Prefer mature/standard components where they provide compatibility, security, or hardware support, including areas such as:

- Linux kernel
- DRM/KMS
- evdev
- ALSA
- camera/media infrastructure
- cryptography / TLS
- filesystem encryption
- codecs and compression
- hardware drivers
- bootloader signing infrastructure

Keep Onuron identity in project-owned components such as:

- `nilinit`
- NilHAL interfaces
- Onuron service contracts
- NilLang
- Alap
- NilUI
- `.nilax`
- `nilpkg`
- SoftBus policy
- native permission model
- Onuron shell
- Onuron update orchestration

## 11. Current work priority

When selecting work, prefer the native platform spine before compatibility breadth:

1. Canonical architecture + status ledger
2. Reproducible QEMU path
3. `nilinit` hardening
4. Stable NilHAL boundary
5. Versioned IPC/contracts
6. Fake HAL / integration testing
7. Native `.nilax` signing and package flow
8. Per-app identity and sandboxing
9. One native app end-to-end
10. NilUI input/scene/render boundaries
11. Reference-hardware bring-up
12. DRM/KMS, touch, storage, power, networking, and other real mobile foundations
13. Android compatibility after the native platform is sufficiently stable

## 12. Final rule

When making a change, ask:

> Does this strengthen OnuronOS as an independent Linux-based mobile OS with a native NilLang/Alap application platform, or does it quietly turn OnuronOS into something else?

If the latter, stop and document the architectural conflict before proceeding.
