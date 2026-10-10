# Reference Board (D1)

> **Status:** factual as of this document's creation. Nothing in this file
> claims real-hardware validation. See the maturity rule in `AGENTS.md` §4:
> code presence (a trait, a driver file, a `.so`) is **not** evidence that a
> feature works on any device.

This document names **one** reference bring-up target and records exactly what
is and is not verified, so the project stops treating a collection of HAL
traits as if it were hardware support.

---

## 1. The reference target

**Primary reference target: QEMU, x86_64 (the `qemu-system-x86_64` default
machine).** All existing, runnable boot tooling and the CI boot smoke test
target x86_64. Automated boot smoke testing passes green in CI.

**Secondary reference target: QEMU `virt` on aarch64 (`qemu-system-aarch64 -M virt`).**
The bring-up tooling is implemented in `build/qemu-aarch64.sh` / `.ps1`, `build/mkinitramfs.py --arch aarch64`,
`build/mkdisk.py`, and `build/qemu-smoke.py --arch aarch64`. The CI job is wired in `.github/workflows/linux-qemu.yml`.
Physical ARM64 smartphone hardware is **NOT YET RUN**.

**Hosted runtime target: Samsung Galaxy S25 (`android-host-arm64`).**
Documented under `docs/s25-hosted-runtime.md` and ADR-0001. Runs as an Android APK hosting the
Onuron shell UI and JNI bridge. It is explicitly a hosted evaluation track, not bare-metal native OS.

**Physical native bring-up candidate (planned, not validated): OnePlus 6T (`fajita`, Snapdragon 845).**
Selected under ADR-0007 as the primary physical bring-up candidate due to unlockable bootloader,
EDL disaster recovery, and established community mainline Linux baseline. No physical device is
flashed yet; status remains **PLANNED / NOT RUN**.


> Invariant: QEMU is the CI reference target. A physical device is only claimed
> once the hardware test matrix below has been executed **on that device** and
> recorded as VALIDATED.

---

## 2. Exact build + QEMU invocation (as it exists in the repo)

Do not treat the snippets below as aspirational — they are the real arguments
used by the checked-in scripts.

### 2.1 Build / package

| Artifact / step | Script | What it actually does |
|---|---|---|
| Workspace build | `build/build.py` (default arg `aarch64-generic`) | Runs `cargo build --release --workspace`, creates a rootfs skeleton under `out/<target>/`, writes a 1 MiB `system_a.img` and a text `vbmeta_a.img`. It does **not** cross-compile to aarch64 and does not boot anything. |
| Full image build | `build/build.sh` (default `x86_64-generic`) | Builds the workspace and installs binaries into `out/<device>/usr/bin`; calls `build/mkimage-x86.sh` (GPT A/B + userdata disk `nilos-disk.raw`); signs `vbmeta_a.img` via `build/mkvbmeta.sh`. |
| Kernel | `build/mkinitramfs.py` | Downloads **Alpine v3.19 x86_64 `vmlinuz-lts`** from `dl-cdn.alpinelinux.org` into `out/x86_64-generic/vmlinuz-lts`. x86_64 only. |
| Initramfs | `build/mkinitramfs.py` | Packages `out/x86_64-generic/rootfs` into `out/x86_64-generic/nilos-initramfs.cpio.gz` (SVR4 cpio, gzip). Requires a built `nilinit`. The binary list already includes `powerd` and `thermald`.
| Data disk | `build/mkdisk.py` | Creates a 256 MB sparse ext2 image `out/x86_64-generic/nilos.img` (mounted at `/dev/vda` → `/data` on first boot).

### 2.2 QEMU invocations present in the repo

- **`build/qemu-boot.sh`** (Linux/macOS/WSL) and **`build/qemu-boot.ps1`**
  (Windows) — interactive boot:

  ```
  qemu-system-x86_64 -m 1024 -smp 2 \
    -kernel out/x86_64-generic/vmlinuz-lts \
    -initrd out/x86_64-generic/nilos-initramfs.cpio.gz \
    -append "console=ttyS0 console=tty0 init=/init panic=10 rw" \
    -drive file=out/x86_64-generic/nilos.img,format=raw,if=none,id=vda_disk,cache=writeback \
    -device virtio-blk-pci,drive=vda_disk,id=vda \
    -netdev user,id=net0 -device virtio-net-pci,netdev=net0 \
    -vga std -serial stdio        # or: -nographic -serial mon:stdio
  ```

- **`build/qemu-smoke.py`** — the headless CI boot test:
  `qemu-system-x86_64 -m 1024 -smp 2 -kernel ... -initrd ...
  -append "console=ttyS0 init=/init panic=-1" -display none -monitor none
  -serial stdio -no-reboot -no-shutdown`.
  It succeeds only when the guest prints `Onuron OS boot completed`.
- **`build/qemu-run.sh`** — a development helper (not CI). It boots
  `out/x86_64-generic/nilos-disk.raw` with `-enable-kvm`, virtio-vga-gl, a
  virtio tablet/keyboard, and hostfwd `tcp::2222-:22`. It depends on KVM and on
  a disk image produced by `build/mkimage-x86.sh`.

### 2.3 CI

`.github/workflows/linux-qemu.yml` runs on `ubuntu-latest` and, in order:
`cargo test --workspace --all-targets`, the initramfs builder unit tests,
`cargo clippy`, a `x86_64-unknown-linux-musl` release build,
`python3 build/mkinitramfs.py`, image reproducibility check, persistent-data validation harness,
and finally `python3 build/qemu-smoke.py`.
It targets **x86_64 musl** and executes automated headless QEMU boot verification.

> Status: The automated x86_64 QEMU boot smoke test and persistent-data harness are **VALIDATED IN CI**.
> Physical mobile hardware (PinePhone / A64) remains unexecuted (**NOT RUN**).

---

## 3. Hardware test matrix

Legend:
- **VALIDATED (CI/QEMU)** — automated execution succeeds in CI, with verifiable logs.
- **NOT RUN** — script/support exists in repo but has not been executed on target hardware.
- **PLANNED** — architectural target, not yet wired.

| Capability | QEMU-ci (x86_64) | Physical (PinePhone candidate) | Notes |
|---|---|---|---|
| Boot to `nilinit` completion | VALIDATED (CI/QEMU) | NOT RUN | `build/qemu-smoke.py` asserts `Onuron OS boot completed` and verifies all core services are healthy. Passes green in CI. |
| Display on/off | NOT RUN | NOT RUN | `-vga std` exists; automated DRM/KMS presentation assertion pending. |
| Touch / input events | NOT RUN | NOT RUN | `services/inputd` reads `/dev/input/event*`; automated hardware evdev loop pending. |
| Battery read | NOT RUN | NOT RUN | QEMU has no battery; synthetic battery values provided. Real gauge requires physical PMIC bring-up. |
| Suspend / resume | PLANNED | PLANNED | Writing `/sys/power/state` on mobile hardware not implemented. |
| Storage (`/data`) | VALIDATED (CI/QEMU) | NOT RUN | `build/persistent-data-test.py` validates write → reboot → read persistence on virtio-blk/ext4 image. |
| Networking | NOT RUN | NOT RUN | `netd` reads interfaces; automated external internet routing assertion pending. |

---

## 4. What remains before the physical target can be claimed supported

At minimum, before "PinePhone (Allwinner A64) is supported" can be written
anywhere:

1. A bootable aarch64 kernel + device tree (DTB) for the A64 and a bootloader
   (e.g. U-Boot) path, none of which exist in this repo yet.
2. A real aarch64 QEMU `-M virt` build + boot path and a CI job that exercises
   it (the current tooling is x86_64-only).
3. Display: DRM/KMS bring-up on the actual panel, not a stub
   (`docs/drm-kms-compositor.md` describes intent, not a verified driver).
4. Input: evdev touch controller enumerated and delivering events to
   `inputd` from real hardware.
5. Power: a real battery gauge (`/sys/class/power_supply/*`) and a
   suspend/resume path that actually writes `/sys/power/state`.
6. Storage: persistent `/data` verified across a real reboot.
7. Networking: Wi-Fi/cellular, not just a virtio link.
8. Every row in the matrix above executed on the device and marked VALIDATED
   with recorded evidence (boot log, kernel version, dmesg excerpt).

Until then, physical phone targets remain **candidate / planned, not validated**, and
QEMU is the validated automated reference target in CI.