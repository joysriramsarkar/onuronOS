# ADR-0002: Target Identity and Canonical Naming

## Status
Accepted

## Context
Multiple overlapping target names existed across scripts and build systems (`aarch64-generic`, `arm64-generic`, `aarch64-qemu`, `x86_64-generic`), causing confusion and risking unintentional cross-flashing or cross-linking.

## Decision
We define canonical internal target identifiers:
- `qemu-x86_64`: x86_64 QEMU emulation with Linux LTS kernel and x86_64 userspace.
- `qemu-aarch64`: ARM64 QEMU `virt` machine with AArch64 Linux LTS kernel and AArch64 musl userspace.
- `android-host-arm64`: Android hosted application with `arm64-v8a` JNI shared library (`libandroid_host.so`).
- `oneplus-fajita`: Initial native reference hardware profile for OnePlus 6T (Qualcomm Snapdragon 845).

Legacy aliases (`aarch64-generic`, `x86_64-generic`) are retained only as compatibility mappings in build scripts, pointing unambiguously to their canonical counterparts.

## Consequences
- Every output directory (`out/<target>`), build manifest (`manifest.json`), checksum file (`checksums.txt`), and CI job uses these canonical identifiers.
- Image builders and flash tools reject mismatched target profiles fail-closed.

## Validation
- `build/mkinitramfs.py`, `build/build.py`, `build/qemu-smoke.py`, and CI workflows enforce target triple and architecture consistency.
