# ADR-0001: Native OS vs Android-Host Boundary

## Status
Accepted

## Context
OnuronOS aims to provide both a native independent Linux mobile operating system and an Android-hosted evaluation runtime (such as on the Samsung Galaxy S25). Running inside Android as an application Activity/Surface is fundamentally distinct from replacing Android as the bare-metal OS. Confusing these two targets misleads users and distorts architectural boundaries.

## Decision
We establish a strict dual-track platform definition:
1. **Track B (Hosted Runtime)**: An Android app (`android-host`) running on top of the host Android Linux kernel, Android framework, and Android security sandbox. It uses JNI (`NativeBridge`) to bridge hardware capabilities where Android permissions allow. It is explicitly labeled in UI and metadata as `Onuron Hosted Runtime`.
2. **Track C (Native Mobile OS)**: A standalone Linux-based mobile operating system where the Linux kernel boots into `nilinit` (PID 1), NilHAL abstracts the SoC/peripherals directly, and native `.nilax` applications run isolated under `nilrt` namespaces and SELinux.

Under no circumstances will the Android-hosted demo be marketed or architecturalized as native hardware porting of the device.

## Consequences
- APK builds include explicit `hosted` indicators in `About` and diagnostic screens.
- App permissions in hosted mode are bounded by Android host runtime permissions and user grants.
- Native OS bring-up requires verified bootloader, kernel DTB, recovery image, and direct hardware drivers.

## Validation
- APK build manifests contain `"target": "android-host-arm64"`.
- Native image manifests contain `"target": "qemu-aarch64"` or device profile `"oneplus-fajita"`.
