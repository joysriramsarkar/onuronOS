# Onuron OS — Deep Improvement Plan

> **Status:** Living document. Last updated after the recovery, package
> retrieval, IPC-authorization, sandbox/permission, update-signing,
> power/thermal, reproducibility, and app-lifecycle passes.
>
> **Honest summary:** Onuron OS is a functional prototype with real
> cryptographic package verification (now including HTTPS retrieval and safe
> extraction), a supervised init with a recovery path, a namespace sandbox with
> a permission model, peer-credential IPC authorization, fscrypt-based
> encryption at rest, signed A/B system updates, a deterministic initramfs
> build, and a working UI shell. Its `cargo test --workspace` suite is green on
> the Windows dev host, but **no Linux/QEMU run has been recorded** and no row
> of the hardware matrix is validated. It is **not** a complete operating
> system. This plan orders the remaining work by risk and dependency so that
> each phase produces a verifiable improvement rather than a claim.

---

## 1. How to read this plan

Every item below has four fields:

| Field | Meaning |
|---|---|
| **Goal** | The observable behaviour that must exist when the item is done |
| **Why now** | The risk or dependency that makes this the correct next step |
| **Done when** | Concrete, testable acceptance criteria |
| **Risks** | What could go wrong and how to detect it early |

Work proceeds in phases. A phase is only complete when its "Done when"
criteria pass on the target platform, not just on a Windows dev host.

---

## 2. Phase A — Make the current code trustworthy

The code that exists must be correct before more is built on top of it.

### A1. Linux-target CI must actually run

- **Goal:** Every push builds for `x86_64-unknown-linux-musl`, runs the full
  test suite, packages the initramfs, and boots it under QEMU.
- **Why now:** All verification so far has been on Windows. Linux-only code
  (`unshare`, `flock`, `chroot`, sysfs parsing, musl linking) has never
  executed. The new `.github/workflows/linux-qemu.yml` exists but has not
  been observed passing.
- **Done when:**
  - The workflow is green on a clean runner.
  - `cargo test --workspace --all-targets` passes under the musl target.
  - `python3 build/qemu-smoke.py` reports `nilinit` boot completion.
  - A deliberately failing test turns the workflow red (proves the gate works).
- **Risks:** musl linking failures, missing `qemu-system-x86_64` in the
  runner image, kernel download flakiness. Mitigate by pinning the kernel
  URL and caching it.

### A2. Service crash/restart tests ✅ DONE

- **Goal:** Prove that `nilinit` restarts a crashing service with backoff and
  that a healthy service's backoff resets.
- **Why now:** The restart logic (`nilinit/src/supervisor.rs`) is pure and
  unit-tested, but the *integration* — a real child process dying and being
  respawned by PID 1 — is untested.
- **Done when:**
  - A test spawns a service that exits immediately, asserts it is restarted
    at least N times, and asserts the delay between restarts grows.
  - A test spawns a service that stays up past the 60 s healthy threshold and
    asserts the backoff counter resets.
  - A malformed `exec` line in `services.toml` produces a logged warning and
    does not panic PID 1.
- **Risks:** Timing-sensitive tests. Use injected clocks or generous margins
  rather than exact sleeps.
- **Status:** Implemented. The supervision loop was extracted into a testable
  `Supervisor` struct (`nilinit/src/supervisor.rs`) and covered by integration
  tests that spawn real child processes: crash/restart with backoff, no-respawn
  for `restart = "never"`, malformed exec handling, shutdown, and healthy-uptime
  backoff reset. All 8 tests pass.

### A3. Persistent-data validation

- **Goal:** Boot with a pre-populated `/data` image and assert that OOBE state,
  PIN hash, contacts, and SMS threads survive a reboot.
- **Why now:** `nilinit` mounts `/data` and falls back to tmpfs when no disk
  is present. Nothing verifies that data written in one boot is readable in
  the next.
- **Done when:**
  - A QEMU run writes state, shuts down, reboots from the same disk, and the
    shell reports the same user name, PIN acceptance, and SMS threads.
  - Corrupting the disk image produces a logged warning, not a silent tmpfs
    fallback that looks like success.
- **Risks:** ext4 tooling in the runner. Use `mkdisk.py` to build the image
  and `debugfs` or a small Rust reader to inspect it.
- **Status:** Harness implemented in `build/persistent-data-test.py` and
  `build/mkdisk.py`: `mkdisk.py` writes a real, mountable ext2/3/4 image when
  `mke2fs` is available, and `persistent-data-test.py` materializes known state
  (`/data/config/oobe_done`, a PIN hash, a contacts file, an SMS thread) and
  verifies it with `debugfs`. It skips cleanly on hosts without e2fsprogs.
  **What is not yet proven:** a real QEMU write → reboot → read-back, and a
  corrupt-image path that logs instead of silently falling back to tmpfs. Those
  remain open; the harness is the substrate for them, not the proof.

### A4. Recovery boot path

- **Goal:** A `recovery` kernel command line (or failed-boot counter) drops
  into a minimal recovery shell instead of the normal UI.
- **Why now:** There is currently no way to recover a device whose `/data` is
  corrupt or whose boot count is exceeded. `nilrecovery` exists as a binary
  but is not wired into the boot flow.
- **Done when:**
  - Booting with `onuron.recovery=1` starts `nilrecovery` instead of the
    normal shell.
  - Three consecutive failed boots (tracked via a counter file on `/data`)
    automatically enter recovery.
  - Recovery can reformat `/data` only after an explicit confirmation.
- **Risks:** A recovery path that is too easy to trigger bricks the device;
  too hard and it is useless. Require both a counter and an explicit flag.
- **Status:** Implemented. `nilinit/src/recovery.rs` holds the pure decision
  logic (`plan_boot`, `recovery_requested`, `parse_boot_count`) with 6 unit
  tests; `nilinit/src/main.rs` increments `/data/system/boot_count` at boot
  start, enters recovery at `MAX_FAILED_BOOTS = 3`, and clears the counter only
  after `nilinit` reports boot completion. `runtime/nilrt/src/bin/nilrecovery.rs`
  now runs a real menu whose factory-reset path requires typing `YES`, then
  unmounts and reformats a real block device (falling back to removing `/data`
  contents for tmpfs); it has 3 unit tests. Not yet exercised under QEMU.

---

## 3. Phase B — Close the package-distribution gap

The local installer is solid. What remains is getting packages *onto* the
device safely.

### B1. Safe archive extraction ✅ DONE

- **Goal:** Accept a `.nilax` archive (tar or zip) and extract it into a
  staging directory without path traversal, symlink escapes, or zip bombs.
- **Why now:** `install <directory>` trusts a pre-extracted directory. Real
  distribution will use archives, and archive extraction is a historically
  bug-prone area (Zip Slip, symlink races).
- **Done when:**
  - `extract <archive> <dest>` rejects entries containing `..`, absolute
    paths, and symlinks pointing outside the destination.
  - A decompression bomb (highly compressible payload) is rejected via an
    uncompressed-size cap checked *during* extraction, not after.
  - Extraction is atomic: either the whole archive lands in staging or
    nothing does.
  - Tests include a malicious tar with `../../etc/passwd`, a symlink escape,
    and a zip bomb.
- **Risks:** Platform differences in tar/zip handling. Prefer a pure-Rust
  crate (`tar`, `zip`) over shelling out to system tools.
- **Status:** Implemented in `pkg/nilpkg/src/extract.rs`. Covers tar and zip,
  path traversal, absolute paths, symlinks, non-regular files, per-file and
  total size caps, and format auto-detection by magic bytes. 7 tests pass,
  including raw-byte malicious tar entries that bypass the tar crate's own
  validation.

### B2. Remote retrieval over HTTPS ✅ DONE

- **Goal:** `nilpkg fetch <url>` downloads a package over TLS, verifies the
  publisher signature, and installs it.
- **Why now:** Without retrieval, every package must be side-loaded, which is
  not a distribution channel.
- **Done when:**
  - TLS certificate validation is on by default with no override flag.
  - The downloaded archive's SHA-256 is checked against the manifest before
  extraction.
  - A failed download leaves no partial package on disk.
  - A test serves a local HTTPS fixture and asserts a tampered payload is
  rejected.
- **Risks:** Certificate pinning vs. CA trust. Start with the system trust
  store; add pinning later for the update channel.
- **Status:** Implemented in `pkg/nilpkg/src/lib.rs` (`fetch`/`install` over
  HTTPS) and exercised by the HTTPS fixture tests. TLS validation is on by
  default with no override; the downloaded archive's SHA-256 is checked against
  the signed manifest before extraction; a failed download leaves no partial
  package; tampered payloads and untrusted publishers are rejected. The local
  test uses a self-signed fixture served over loopback; CA pinning for the real
  update channel is still future work.

### B3. Publisher key management ✅ DONE

- **Goal:** `nilpkg trust <keyfile>` and `nilpkg revoke <keyfile>` manage the
  trust store, and `keygen` encrypts the private key at rest.
- **Why now:** Trust is currently a manual file copy into
  `/etc/onuron/keys/trusted/`. Private keys are written in cleartext.
- **Done when:**
  - `trust` verifies the file is a valid 32-byte Ed25519 public key before
    adding it.
  - `revoke` moves a key to `revoked/` and is immediately effective.
  - `keygen` prompts for a passphrase and encrypts the private key (e.g.
    age, or NaCl secretbox with a scrypt-derived key).
  - A test asserts an encrypted key cannot be used without the passphrase.
- **Risks:** Losing the only trusted key bricks all future installs. Keep a
  recovery mechanism (a factory key in the image) and document it.
- **Status:** Implemented in `pkg/nilpkg/src/keymgmt.rs`. `keygen` now prompts
  for a passphrase and writes an encrypted private key (`enc1$salt$tag$ciphertext`
  with a SHA-256-derived key and authentication tag). `trust` validates and adds
  a public key to `trusted/`; `revoke` moves it to `revoked/`. 5 tests pass,
  including wrong-passphrase and tamper detection.

---

## 4. Phase C — Security hardening

### C1. IPC authorization audit

- **Goal:** Every Unix socket the system opens (`/run/nilos/*.sock`,
  `/run/onuron/*.sock`) has an explicit authorization check.
- **Why now:** `nilbus`, `notifyd`, `nilimed`, `nilttsd`, and `nilandroidd`
  all open sockets. None currently verify that the caller is allowed to
  connect. A local unprivileged process could otherwise send notifications,
  inject text, or talk to the Android bridge.
- **Done when:**
  - Each socket records its peer's UID/GID (via `SO_PEERCRED` on Unix).
  - A policy file maps UID → allowed services.
  - Tests connect as an unprivileged user and assert denial.
- **Risks:** Breaking legitimate services. Run the audit on a live QEMU boot
  and fix denials found in practice.
- **Status:** Implemented in `runtime/nilsd/src/auth.rs` with a policy file at
  `etc/nilos/ipc-policy.toml`. Daemon sockets (`notifyd`, `nilimed`,
  `nilandroidd`, `nilkeyd`, `softbus`, the Android agent) record the peer UID
  via `SO_PEERCRED` and check it against the UID→service policy before serving;
  unknown peers/services are denied. The decision logic is unit-tested.
  **Not yet done:** observing a live QEMU boot to confirm no legitimate
  service is denied — that is the remaining acceptance step.

### C2. Sandbox escape-resistance review

- **Goal:** An independent review of `runtime/nilrt/src/sandbox.rs` against
  known escape techniques.
- **Why now:** The sandbox has the right primitives (`unshare`, `chroot` +
  `chdir("/")`, `setgroups`, private mounts) but has never been attacked.
- **Done when:**
  - A review checks: CWD handling after `chroot`, `/proc` and `/sys` mounts
    inside the sandbox, `ptrace` access from outside, device-node access, and
  mount-propagation leaks.
  - Each finding has a test that fails before the fix and passes after.
- **Risks:** Some escapes require kernel features that cannot be disabled
  (e.g. `ptrace` of a sibling). Document these as accepted risks with
  mitigations rather than pretending they are fixed.
- **Status:** Reviewed and hardened. `sandbox.rs` now performs all privileged
  setup in the forked child's `pre_exec`: a fresh `procfs` is mounted after
  `CLONE_NEWPID` (so the app cannot read host `/proc`), `/tmp` becomes a private
  tmpfs, and `/sys` is replaced with an empty read-only tmpfs by default.
  CWD-after-chroot and supplementary-group leakage remain fixed. The mount plan
  is pure and unit-tested (`plan_sandbox_mounts`, 4 tests). **Accepted risks,
  documented in the module header:** same-UID `ptrace` without a user namespace
  or LSM (mitigated by per-app UIDs / SELinux), device-node access without a
  devices cgroup (mitigated by the E2 masking plan), and kernel namespace bugs.

### C3. Verified boot and signed updates

- **Goal:** The boot chain verifies each stage's signature, and system
  updates are signed and applied atomically.
- **Why now:** There is no integrity protection below the userspace. A
  modified kernel or initramfs is accepted silently.
- **Done when:**
  - The kernel command line or a TPM-backed measure records the initramfs
  hash.
  - `nilupd` verifies an update signature against a trusted key before
  applying it.
  - An update that fails verification leaves the previous system bootable.
- **Risks:** This is the largest remaining item. Scope it to initramfs
  signing first; full measured boot is a follow-up.
- **Status:** Update-signing side implemented in `services/nilupd` (lib + CLI):
  a signed update image is verified (streamed SHA-256 + size + Ed25519 over the
  canonical manifest + trusted/revoked publisher) before anything is written,
  then applied to the inactive A/B slot with a durable journal + atomic pointer
  flip, and recoverable after an interrupted apply. 7 tests cover valid apply,
  tampered payload, untrusted publisher, a running-image-hash replay guard,
  crash recovery, and reversible rollback. **Not implemented:** a hardware-rooted
  measured boot / TPM attestation of the kernel or initramfs. The "running image
  hash" is a locally maintained marker, not a bootloader measurement; this is an
  update trust anchor, not verified boot. Documented as such in code and docs.

### C4. Encryption at rest

- **Goal:** `/data` is encrypted with a key derived from the device PIN plus
  a hardware-bound secret.
- **Why now:** The PIN is now hashed properly, but `/data` contents
  (contacts, SMS, app data) are plaintext on disk.
- **Done when:**
  - `nilkeyd` derives a fscrypt v2 policy key from the PIN hash and a
  per-device secret stored in a TPM or a file with `0o400` permissions.
  - A wrong PIN cannot decrypt `/data`.
  - Changing the PIN rewraps the key without re-encrypting everything.
- **Risks:** Losing the device secret makes data unrecoverable. Document the
  recovery procedure explicitly.
- **Status:** Implemented in `services/nilkeyd/src/lib.rs` + `fscrypt.rs`
  (20 tests). It derives the fscrypt v2 policy key from the PIN hash and a
  per-device secret, builds the kernel `fscrypt` UAPI structures
  (`key_specifier`, `add_key_arg`, `policy_v2`) verified against the kernel
  headers, and drives `FS_IOC_ADD_ENCRYPTION_KEY` / `FS_IOC_SET_ENCRYPTION_POLICY`.
  A wrong PIN cannot produce the key; changing the PIN rewraps without
  re-encrypting. **Not yet done:** wiring the key handoff into `nilinit` and a
  real QEMU `/data` mount, plus documenting the recovery path end to end.

---

## 5. Phase D — Hardware enablement

### D1. Named target board

- **Goal:** Pick one supported device (PinePhone or a QEMU virt machine) and
  make it the reference target.
- **Why now:** "Hardware support" is currently a collection of HAL traits
  with no device where they all work together.
- **Done when:**
  - The reference target boots to the lock screen.
  - Display, touch, and input work through the real HAL backends.
  - A hardware test matrix (display on/off, touch events, battery read,
    suspend/resume) runs in CI on QEMU and is documented for real hardware.
- **Risks:** Real hardware is unavailable in CI. Keep QEMU as the CI target
  and document manual steps for physical devices.
- **Status:** A reference target is now named in `docs/reference-board.md`:
  QEMU x86_64 as the primary (and only wired) CI target, QEMU aarch64 `-M virt`
  as planned, and PinePhone (Allwinner A64) as an explicit "candidate, not
  validated". The document records the real build/QEMU invocations from the
  repo and a hardware test matrix in which **no row is VALIDATED** on QEMU or
  hardware. It states plainly that a trait or driver file existing is not
  evidence of hardware support.

### D2. Power and thermal management

- **Goal:** `powerd` and `thermald` act on real sysfs data and enforce
  safe behaviour on low battery and high temperature.
- **Why now:** Both daemons exist but have no tests against realistic data.
- **Done when:**
  - A battery below 5% triggers a warning and then a graceful shutdown.
  - A thermal zone above a threshold reduces performance and logs the event.
  - Tests feed synthetic sysfs trees and assert the daemon's response.
- **Risks:** Aggressive shutdown thresholds can brick a device that cannot
  charge. Make thresholds configurable and default conservatively.
- **Status:** Implemented as pure, unit-tested policy in `powerd` and
  `thermald` (14 tests total). `powerd` parses sysfs capacity/status, warns
  below 5%, and requests a graceful shutdown below a critical threshold while
  charging suppresses a power-off; `thermald` parses millidegree zones and
  requests throttling above a threshold, treating malformed/missing sensors as
  non-fatal. **Not yet done:** running against real hardware sysfs (QEMU has no
  battery) and the actual suspend/resume path (`/sys/power/state` is not yet
  written).

---

## 6. Phase E — Application platform

### E1. App lifecycle tests

- **Goal:** An app can be installed, launched, backgrounded, resumed, and
  uninstalled with its data surviving or being removed as documented.
- **Why now:** `nilrt-launch` and the sandbox exist, but no end-to-end test
  exercises the full lifecycle.
- **Done when:**
  - A test installs a package, launches it through the sandbox, writes data,
  kills the process, relaunches, and asserts the data persisted.
  - Uninstalling removes the app directory and its data.
  - A crash mid-launch leaves no staging residue.
- **Risks:** Lifecycle bugs that only appear under memory pressure. Add a
  test that launches with a reduced memory limit.
- **Status:** Implemented in `runtime/nilrt/src/lifecycle.rs` with an
  end-to-end integration test (`runtime/nilrt/tests/lifecycle.rs`): it installs
  a signed package via `nilpkg`, launches it through the sandbox, writes state,
  relaunches and observes persistence, then uninstalls and asserts the app dir
  and data are gone. A second test asserts an aborted launch and a failed
  install leave no staging residue. **Not yet done:** background/resume
  semantics and the reduced-memory-pressure case.

### E2. Permission enforcement

- **Goal:** The sandbox denies access to resources the app's manifest does
  not list.
- **Why now:** The manifest has a `permissions` field and an allowlist, but
  nothing enforces it at runtime.
- **Done when:**
  - An app without `camera` cannot open `/dev/video0`.
  - An app without `storage.write` cannot write outside its data directory.
  - A test asserts each denied access.
- **Risks:** Over-blocking breaks legitimate apps. Ship with a permissive
  default and a strict mode, then tighten.
- **Status:** Implemented. `runtime/nilrt/src/permissions.rs` holds a pure
  policy (`check_app_permissions`, `AppPolicy::check_path`, `plan_restrictions`)
  with 8 tests proving a camera-less app is denied `/dev/video0`, an app without
  `storage.write` is denied writes outside its data dir, and traversal
  (`<data>/../secret`) is not treated as app-private. `nilrt-launch` now resolves
  *granted* (not merely requested) permissions from the broker and enables
  strict enforcement by default for installed apps; `sandbox.rs` applies the
  plan by masking device nodes with `/dev/null` and remounting the root
  read-only. Default is permissive for non-installed dev payloads; enforcement
  is a mount-level mechanism, and a race-free seccomp mediation is a documented
  follow-up.

### E3. Simulated vs. real hardware labelling ✅ DONE

- **Goal:** Every UI screen that shows simulated data is labelled as such,
  and the README maturity table matches reality.
- **Why now:** The phone app shows "(simulated)" but the SoftBus and Android
  screens present fabricated state without any label. This has already
  caused overclaiming.
- **Done when:**
  - Each simulated screen has a visible "SIMULATED" badge.
  - The README maturity table is regenerated from a script that checks for
    the badge, so it cannot drift.
- **Risks:** Labelling is easy to forget. Make it part of the UI review
  checklist.
- **Status:** Implemented. `shell/src/simulated.rs` is the single registry of
  simulated screens; `docs/maturity.toml` is the source of truth; and
  `build/gen-maturity.py` regenerates the README maturity table and fails
  (`--check`) if the README drifts or a simulated screen lacks its badge. 12
  Python tests cover the generator, including a check that the checked-in README
  is fresh. The drift check is wired into `.github/workflows/code-quality.yml`.

---

## 7. Phase F — Release engineering

### F1. Reproducible images

- **Goal:** Building the same commit twice produces byte-identical images.
- **Why now:** The initramfs builder embeds a fixed mtime, which is good, but
  the kernel is downloaded and the disk image is built without reproducibility
  checks.
- **Done when:**
  - Two builds of the same commit produce identical `nilos.img` and
    initramfs hashes.
  - A CI job builds twice and compares hashes.
- **Risks:** Toolchain nondeterminism. Pin the toolchain version and document
  any remaining sources of variation.
- **Status:** Partial, and documented honestly in `docs/reproducibility.md`.
  The initramfs (`build/mkinitramfs.py`) now uses deterministic newc metadata
  and gzip with `mtime=0`/no embedded filename, and the synthetic disk image is
  byte-stable. `build/check-reproducible.py` builds both twice and compares
  SHA-256; `build/test_reproducible.py` unit-tests the helpers. CI runs the gate
  after packaging. **Still nondeterministic:** the network-fetched kernel (URL
  not digest-pinned) and a real `mke2fs` image (random UUID/hash seed/time).

### F2. Signed updates with rollback

- **Goal:** `nilupd` downloads a signed update, verifies it, and applies it
  with the same crash-safety guarantees as `nilpkg upgrade`.
- **Why now:** `nilpkg` has a proven transaction model; `nilupd` is a stub.
  Reuse the journal + two-rename pattern.
- **Done when:**
  - An update is signed by a trusted key and rejected otherwise.
  - A power loss during update leaves the previous system bootable.
  - A test simulates interruption at each step and asserts recovery.
- **Risks:** Reusing the pattern without adapting it. The update payload is
  larger and may span multiple files; design for partial application.
- **Status:** Implemented in `services/nilupd` (lib + thin CLI), reusing
  nilpkg's durable-write/journal machinery. An update is accepted only if the
  image SHA-256/size and the Ed25519 manifest signature verify and the publisher
  is trusted (and not revoked); the previous slot is never touched until the new
  image is fully staged and fsynced, and the active slot is flipped with an
  atomic rename. `recover_pending` repairs an interrupted apply, and `rollback`
  restores the prior slot and is itself reversible. 7 tests cover valid apply,
  tamper, untrusted publisher, replay guard, crash recovery, and rollback.
  **Not done:** downloading the update over HTTPS inside `nilupd` itself (the
  CLI takes a verified directory) and the multi-file payload case.

### F3. Endurance and soak testing

- **Goal:** The system runs for an extended period without leaking memory,
  file descriptors, or processes.
- **Why now:** Long-running bugs (leaks in the supervision loop, socket
  accumulation in daemons) are invisible to short tests.
- **Done when:**
  - A 24-hour QEMU soak run shows stable memory and process count.
  - A leak-detection test runs the supervision loop for 10,000 iterations.
- **Risks:** Soak tests are slow. Run them on a schedule, not on every push.
- **Status:** Partial. `nilinit/src/supervisor.rs` now has a bounded soak test
  that drives the supervision bookkeeping for 10,000 iterations and asserts no
  live children accumulate and the restart-attempt count stays bounded (runs in
  well under a second). **Not done:** the 24-hour QEMU soak run measuring RSS,
  FD, and process counts over time.

---

## 8. Sequencing and dependencies

```
A1 (CI) ──► A2, A3, A4 (boot confidence)
                 │
                 ▼
B1 (extraction) ──► B2 (retrieval) ──► B3 (key management)
                 │
                 ▼
C1 (IPC audit) ──► C2 (sandbox review) ──► C3 (verified boot) ──► C4 (encryption)
                 │
                 ▼
D1 (target board) ──► D2 (power/thermal)
                 │
                 ▼
E1 (lifecycle) ──► E2 (permissions) ──► E3 (labelling)
                 │
                 ▼
F1 (reproducible) ──► F2 (signed updates) ──► F3 (soak)
```

Phase A must land first: nothing else can be trusted until the Linux CI is
green. Phases B and C can proceed in parallel after A. D, E, and F depend on
A and partially on B and C.

---

## 9. Definition of done for "complete OS"

Onuron OS is complete when all of the following hold:

1. The Linux CI workflow is green on every push, including a QEMU boot.
2. Every daemon has unit tests and an integration test on the reference
   target.
3. The package manager can fetch, verify, install, upgrade, and roll back
   a package over HTTPS with no data loss on power failure.
4. The sandbox has passed an independent escape-resistance review.
5. `/data` is encrypted and recoverable with the device PIN.
6. A named hardware target boots to a working lock screen with display,
   touch, input, and power management.
7. Every simulated capability is labelled, and the README maturity table is
   generated from verifiable checks.
8. A signed update can be applied and rolled back, and a 24-hour soak run
   shows no leaks.

Until all eight hold, this project is a prototype, and its documentation
should say so.

---

## 10. Immediate next actions

The remaining work is dominated by things that cannot run on a Windows dev
host. In priority order:

1. Run `.github/workflows/linux-qemu.yml` on a clean runner and fix every
   failure until it is green (A1). Nothing else on this list is trustworthy
   until this is observed.
2. Validate the boot/recovery path on QEMU: write `/data` state, reboot from
   the same disk, and confirm persistence; confirm `onuron.recovery=1` and the
   failed-boot counter both enter `nilrecovery` (A3, A4).
3. Run the IPC policy (`etc/nilos/ipc-policy.toml`) on a live boot and fix any
   denials; then actively attack the sandbox and close real findings (C1, C2).
4. Exercise `nilupd` apply/rollback across a simulated power cut on Linux, and
   add the 24-hour QEMU soak run (F2, F3).
5. Build and boot the aarch64 `-M virt` reference target, then start the
   hardware matrix in `docs/reference-board.md` (D1).
