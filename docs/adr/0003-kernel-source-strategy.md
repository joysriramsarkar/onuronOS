# ADR-0003: Kernel Source Strategy

## Status
Accepted

## Context
Kernel requirements differ between virtual development machines and physical smartphone hardware. QEMU uses generic virtual devices (VirtIO, ttyAMA0), whereas smartphones require SoC-specific device tree blobs (DTB), non-free firmware, PMIC power management drivers, and display panels.

## Decision
1. **Virtual QEMU Targets (`qemu-x86_64`, `qemu-aarch64`)**:
   - Use pre-built, cryptographically pinned distribution Linux LTS kernels (Alpine netboot `vmlinuz-lts`).
   - Every kernel binary is validated fail-closed against an exact pinned SHA-256 hash. Download or cache mismatches immediately abort build.
2. **Physical Hardware Targets (`oneplus-fajita`)**:
   - Require an explicit kernel source repository, pinned commit hash, configuration file (`defconfig`), and device tree tree.
   - Initial reference bring-up builds on the community mainline Linux / postmarketOS baseline for Snapdragon 845 (`sdm845`).
   - Vendor kernel reuse without source verification is forbidden for security and reproducibility.

## Consequences
- QEMU target setup remains lightweight, network-verifiable, and reproducible without multi-gigabyte kernel compilation on every local build.
- Native phone porting has an unambiguous provenance baseline before hardware flashing.

## Validation
- `build/mkinitramfs.py` verifies kernel SHA-256 fail-closed.
- Unit tests (`build/test_mkinitramfs.py`) enforce rejection of mismatched kernel digests.
