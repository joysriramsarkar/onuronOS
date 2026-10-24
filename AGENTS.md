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

## 13. Build and test notes (verified in this workspace)

Environment: Debian 13 (trixie). Rust stable via `source $HOME/.cargo/env`.

- Full suite: `cargo test --workspace`. 0 failures on a clean tree.
- `runtime/nilrt` sandbox tests call `unshare(CLONE_NEW*)`, which returns EPERM in
  an unprivileged container. The project ships a documented dev fallback gated on
  `CI=true` or `NILRT_ALLOW_INSECURE_DEV=1`; without it the sandbox refuses to run
  unconfined (by design). Use `CI=true cargo test --workspace` on a dev host.
  On real hardware/root, no override is needed and isolation is enforced.
- Maturity ledger: `python3 build/gen-maturity.py` then
  `python3 build/gen-maturity.py --check`. The generator rejects any entry that
  declares `ui_screens` without `simulated = true`, and rejects registry keys in
  `shell/src/simulated.rs` that no entry claims. Tests: `python3 -m unittest build.test_gen_maturity`.
- `build/mkinitramfs.py` copies an explicit `bins` list into the rootfs. A new
  daemon or app binary must be added to that list or it will not ship.

### Kernel ABI facts worth not re-deriving

- `struct input_event` is 24 bytes on 64-bit Linux, 16 on 32-bit. Getting this
  wrong silently misparses every evdev record.
- ioctl encodings, cross-checked against `<linux/input.h>`:
  `EVIOCGBIT(0,8) == 0x80084520`, `EVIOCGABS(0) == 0x80184540`,
  `EVIOCGNAME(256) == 0x81004506`.
- `cd` must resolve `.`/`..` lexically. `std::fs::canonicalize` alone fails on a
  path that does not exist, so `/data/..` cannot be computed off-device.

### Test-harness gotcha: flock and forked test children

`pkg/nilpkg`'s lock tests were intermittently failing under parallel test
threads. `flock` locks live on the open file description, and a concurrent
test's `fork` transiently copies every descriptor in the parent, so a sibling
test's lock was momentarily held by the forked child between `fork` and `exec`
(FD_CLOEXEC does not help inside that window). The lock itself is correct; the
tests in `pkg/nilpkg/src/lockfile.rs` now take a module-local `Mutex` so the
process-spawning test cannot overlap the others. `lock_fd_is_close_on_exec`
pins the production guarantee that the lock fd does not survive `exec`.

### DRM/KMS: what is verified and what is not

`runtime/nilui-gpu` drives `/dev/dri/card*` through raw KMS ioctls. The parts
that are *verified*:

- ioctl request numbers in `src/ioctl.rs` are asserted against values compiled
  from `/usr/include/drm/drm.h` and `drm_mode.h` (see the tests in that file).
- UAPI struct sizes and field offsets in `src/drm_uapi.rs` are asserted at
  compile time (`const _: () = { ... }`), so a layout drift breaks the build.
- opening a non-DRM file as a card must fail with the ioctl's errno; the test
  `opening_a_non_drm_file_reports_enotty_not_fake_success` is what proves the
  ioctls are actually issued rather than faked.

The part that is *not* verified: no physical panel has run this. The dev
container has no `/dev/dri` node and cannot load `vkms`, so discovery,
`SETCRTC`, and the VBLANK page-flip wait have never executed against a real
driver. Do not describe the display path as hardware-validated until a device
boots and shows a frame. `DrmDevice` reports an offscreen backend (and
`page_flip_success = false`) when there is no panel, and the `present_demo`,
`bootsplash`, and `touch_compositor` binaries exit 2 in that case rather than
printing success.

### Service boundary: the shell must not read hardware directly

The status bar used to read `/sys/class/power_supply` and `/sys/class/net`
itself, which violates §6 of this file. That is fixed: the wire types
(`BatteryInfoPayload`, `NetworkStatePayload`, `ConnectionType`) now live in
`protocol/src/lib.rs`, `powerd`/`netd` re-export them, and the shell reads
battery and network state through `runtime/nilpower` and `runtime/nilnet`.

Two things to keep in mind when touching this area:

- **Two accessors, on purpose.** `battery_info()`/`state()` are infallible and
  return a payload with `is_simulated = true` when the daemon is unreachable, so
  a UI always has something to draw. `try_battery_info()`/`try_state()` return
  `Err` instead, for callers where a placeholder is unacceptable. A UI that uses
  the infallible form must render the `[SIMULATED]` badge from the flag.
- **A failed IPC bind must not print success.** Both daemons bind on a worker
  thread, so the old code printed `[  OK  ] ... daemon active` even when the bind
  had failed and no client could connect. They now wait for the bind outcome and
  print `[ FAIL ]` without calling `notify_ready` when it fails.

What is verified: `runtime/nilpower/tests/powerd_ipc.rs` and
`runtime/nilnet/tests/netd_ipc.rs` spawn the real daemon binaries on a private
socket and read through the real clients; they assert the strict accessor errors
when no daemon runs. What is *not* verified: no physical battery or modem has
been read — `powerd` returns the `nilhal` QEMU path in the dev container.

### Wayland compositor: what is real and what is not

`runtime/nilui-gpu/src/wayland/` implements a Wayland server directly against
the wire protocol, with no `libwayland` dependency. The reason is the same as
for `drm.rs` hand-rolling its ioctls: the headers are absent, `deny.toml` gates
unmaintained crates as a release gate, and the musl release build static-links.
Do not add a Wayland crate here without the maintainer deciding to drop that gate.

Implemented and tested: framing, `wl_display`, `wl_registry`, `wl_compositor`,
`wl_surface`, `wl_shm`/`wl_shm_pool`/`wl_buffer` (ARGB8888, XRGB8888),
`wl_callback`, `wl_output`, `wl_seat`, `wl_touch`, SCM_RIGHTS passing, and
`wl_display.error` before close. `wayland/compose.rs` blends committed surfaces
on the CPU into the existing DRM present path. `wayland/client.rs` exposes
`SurfaceProducer`, which is what a surface producer (including a future Android
bridge) uses.

Not implemented, so do not claim it: `linux-dmabuf` (no GPU buffer import —
every frame must be copied to `wl_shm`), `xdg-shell`, `wl_region` clipping,
damage-limited redraw, `wl_pointer`/`wl_keyboard` events, subsurfaces. A stock
GTK/Qt application will not run against this. No physical panel has run it.

Two bugs worth remembering, because both were found by tests rather than by
reading the code:

- **The attached buffer must survive commits.** Wayland keeps the buffer set by
  `attach` current until another `attach` replaces it. The server originally took
  it with `mem::take`, so a client that attached once and committed many frames
  was drawn exactly once.
- **A protocol violation must not leak descriptors.** `break`ing out of the
  session loop skipped the descriptor cleanup, leaking one fd per malformed
  message until the fd table filled. Descriptors now live in `FdGuard`, which
  closes on drop, so no exit path can leak.

- **A blocking `accept` makes `shutdown()` a no-op.** The listener was blocking,
  so the accept loop parked forever: the `WouldBlock` arm was dead code, the
  compositor could not be stopped by SIGTERM, and it kept the socket bound.
  `bind` now sets the listener non-blocking. Accepted streams stay blocking.

The presenter blocks on an eventfd signalled per commit. Polling on a timer was
tried first and drops frames: a client that commits and then destroys its surface
inside one poll interval is never drawn.

The compositor binary drops the server before `process::exit`, so the socket is
unlinked. Exiting first skips the destructor and leaves a stale socket.

A stopped process can appear in `ps` as state `Z` (zombie) when the container's
PID 1 does not reap it. Check `/proc/<pid>/stat` state, not `ps` presence, before
concluding that a process ignored its signal.

### Test-harness note: never count process-wide descriptors

`rejected_messages_do_not_leak_descriptors` checks for a leak by inode, not by
counting `/proc/self/fd`. Counting is process-wide, and Rust runs test functions
in parallel threads, so other tests' sockets appear in the count and the result
is flaky. Check the specific inode instead.
