# OS completion checklist

This is a release-gate checklist, not a claim that the OS is complete. Host
compilation does not validate Linux-only code, boot behavior, or phone hardware.

## Package-manager hardening pass

Implemented and covered by unit tests:
- Reject package identifiers containing traversal, path separators, shell
  metacharacters, empty dot components, or Windows reserved device names.
- Reject non-ASCII hexadecimal input without slicing inside UTF-8 characters.
- Check declared payload size, SHA-256 integrity, and strict Ed25519 signatures.
- Reject signed-metadata tampering.
- Require an explicitly trusted publisher key for local package installation;
  an embedded self-signed key alone is rejected.
- Reject symlinked source/payload files, oversized files, unsafe executable
  paths, and replacement of an already-installed package.

Additional changes:
- CLI errors return failure exit status.
- Generated demo launchers receive executable permissions on Unix.
- `install <directory>` now accepts a directory containing `manifest.json`
  and the signed executable at `bin/<name>`; it verifies the payload before
  staging it under the app root. `demo <id>` is explicitly only a sample.
- `keygen` refuses to overwrite existing keys and creates the private key
  with mode 0600 on Unix.
- `verify <app_id>` rechecks the on-disk manifest, payload, signature and
  current publisher trust; verification failure returns a nonzero status.
- `upgrade <directory>` replaces an installed package with a newer signed
  version. It refuses same-version upgrades, downgrades, untrusted publishers,
  and upgrades of packages that are not installed; `--allow-downgrade` forces a
  version change but never skips signature verification.
- `rollback <app_id>` restores the version replaced by the last upgrade and can
  itself be undone.
- Interrupted upgrades are repaired by a journal plus two renames: if the
  process dies after the old version is moved aside, the next package operation
  restores it instead of leaving the app missing. Orphaned staging directories
  are cleaned up at the same time.

To trust a publisher, independently verify their Ed25519 public key and put
its 64-character hexadecimal encoding in a `.pub` file under
`/etc/onuron/keys/trusted/` (or `%USERPROFILE%\\.onuron\\keys\\trusted\\`
on Windows). Generating a key does **not** automatically trust it. Signed
manifests bind `sha256`, `size_bytes`, `exec`, permissions, and other metadata.

Validation commands:

```sh
cargo test -p nilpkg
cargo check --workspace
```

## Init, supervision and image-build fixes

- `nilinit` restarts a failed service with exponential backoff (250 ms
  doubling to a 30 s cap) instead of respawning a crashing daemon in a tight
  loop that starved PID 1.
- A service that stayed up at least 60 seconds counts as healthy, so its
  backoff counter resets instead of compounding across unrelated incidents.
- Service command lines are parsed with quote support, so `--label "Onuron
  Shell"` is no longer split into separate tokens on start and restart.
- Malformed exec lines are reported instead of being silently skipped.
- The initramfs builder fails loudly when `nilinit` is missing, rejects
  symlinks in the root filesystem, and grants the executable bit only to
  `/init`, `/bin`, `/sbin` and `/usr/bin` entries.
- `build/qemu-boot.sh` validates arguments and documents them via `--help`.
- The supervision loop is extracted into a testable `Supervisor` struct
  (`nilinit/src/supervisor.rs`) and covered by integration tests that spawn
  real child processes: crash/restart with backoff, no-respawn for
  `restart = "never"`, malformed exec handling, shutdown, and healthy-uptime
  backoff reset.

## Sandbox and permission-broker security fixes

- **Sandbox chroot escape fixed:** `chroot()` does not change the working
  directory, so an app could previously escape a chrooted sandbox with relative
  `..` traversal. The sandbox now `chdir("/")`s after both `chroot` and
  `pivot_root`.
- **Supplementary groups dropped:** `setresgid` does not remove inherited group
  memberships, so apps kept the parent's groups. `setgroups` now runs first.
- **Mount isolation:** the sandbox mount namespace is now made private
  (`MS_REC | MS_PRIVATE`) so host mounts cannot propagate in and sandbox mounts
  cannot leak out.
- **Launcher path traversal fixed:** `nilrt-launch` accepted an app ID like
  `../../bin/sh`, which escaped `/data/app` and could launch any binary. IDs are
  now validated with the same rules as `nilpkg`, and a non-zero child exit is
  propagated instead of being silently ignored.
- **Permission broker no longer panics:** a clock before the Unix epoch made
  `now()` unwrap and panic; a future `last_used_timestamp` made the age
  subtraction underflow. Both now saturate safely.
- **Permission database is no longer corrupted by crashes:** writes go through a
  temp file plus rename instead of truncating the live file in place.
- **Corrupt databases are quarantined:** a malformed file is moved to
  `.corrupt` for diagnosis instead of being silently overwritten (which erased
  every app's grants).

```sh
cargo test -p nilrt
```

## Lockscreen PIN is hashed, not plaintext

- The device PIN was written to `/data/nilos/pin_hash` in cleartext and
  compared with `==`. It is now stored as a `v1$iterations$salt$hash` record:
  16-byte random salt, SHA-256 stretched 100,000 rounds, constant-time
  comparison (`shell/src/pin.rs`).
- The file is written owner-only (`0o600`) on Unix.
- Legacy plaintext files still unlock once, then are transparently re-hashed so
  no device is bricked by the migration.
- OOBE now enforces 4–8 digits in code, not just in the prompt.

```sh
cargo test -p nilshell
```

## Package concurrency and durability fixes

- **Cross-process lock:** install, upgrade and rollback now take an exclusive
  lock on the app root. On Unix this is `flock(LOCK_EX|LOCK_NB)`, on Windows an
  exclusive open with `share_mode(0)`. The OS releases it on process exit, so a
  crash cannot leave a stale lock. Two concurrent `nilpkg` processes no longer
  interleave staging renames.
- **Crash- and power-loss-safe writes:** staged files are written then fsynced,
  the whole staged tree is fsynced before it becomes live, and the app-root
  directory is fsynced after every rename. The upgrade journal is written
  durably before the swap begins. Windows `FlushFileBuffers` needs a
  write-handle, so read-only handles are skipped instead of failing the install.
- Covered by unit tests including a true two-process lock probe.
- `upgrade <directory>` refuses incompatible architectures, newer required OS
  versions, and unknown permissions; a key placed under `revoked/` is rejected
  even when also trusted.

```sh
cargo test -p nilpkg
```

## Verification infrastructure

- CI clippy and test steps no longer swallow failures with `|| true`.
- A new Linux workflow builds musl binaries, packages the initramfs, and runs
  a headless QEMU boot smoke test that succeeds only after `nilinit` reports
  boot completion.
- `build/test_mkinitramfs.py` asserts newc archive layout, device-node modes,
  executable-bit placement, and symlink rejection.

```sh
cargo test -p nilinit
python3 -m unittest build/test_mkinitramfs.py
python3 build/qemu-smoke.py   # requires Linux + QEMU
```

## Recent hardening passes

These are implemented and covered by unit/integration tests on the host, with the
automated Linux/QEMU boot smoke test and persistent data harness now passing green in CI:

- **IPC authorization (C1):** daemon sockets check peer UID via `SO_PEERCRED`
  against `etc/nilos/ipc-policy.toml` (fallback embedded); unknown
  peers/services are denied (`runtime/nilsd/src/auth.rs`).
- **Sandbox review (C2):** privileged setup moved into the child `pre_exec`;
  fresh `procfs`, private tmpfs `/tmp`, hidden read-only `/sys`; CWD and
  supplementary groups fixed. Accepted risks documented in the module header.
- **Encryption at rest (C4):** `nilkeyd` derives an fscrypt v2 policy key from
  the PIN hash + device secret and drives the kernel UAPI ioctls (20 tests).
- **Verified updates (C3/F2):** `nilupd` verifies a signed image against a
  trusted key before applying it to the inactive A/B slot, with journaled
  crash recovery and reversible rollback (7 tests). Not a measured boot.
- **App lifecycle (E1):** install → sandboxed launch → persist → relaunch →
  uninstall, plus no-staging-residue on abort (`runtime/nilrt/tests/lifecycle.rs`).
- **Permission enforcement (E2):** granted (not merely requested) permissions
  gate device nodes and writes outside the data dir; wired into `nilrt-launch`
  and applied by the sandbox mount plan.
- **Power/thermal (D2):** pure policies for low battery and thermal zones with
  synthetic-input tests; no real hardware exercised.
- **Reproducible images (F1):** deterministic initramfs gzip/newc and synthetic
  disk image, gated by `build/check-reproducible.py`.
- **Maturity labelling (E3):** `docs/maturity.toml` → README via
  `build/gen-maturity.py --check`, wired into CI.
- **Soak (F3):** 10,000-iteration supervision bookkeeping test; not the
  24-hour QEMU run.

## Remaining release gates

1. **Linux CI + QEMU (A1, A3):** the workflows exist but no green run is
   recorded. Persistent-data validation needs a real QEMU write→reboot→read
   cycle; corrupt-image handling must log rather than silently fall back to
   tmpfs.
2. **Security, live:** run the IPC policy on a real boot and fix any denials;
   attack the sandbox; implement measured/verified boot (TPM or bootloader
   measurement) — the current update trust anchor is not hardware-rooted.
3. **Hardware (D1):** a named reference target (`docs/reference-board.md`),
   with display/touch/input/audio/network/power actually VALIDATED on it,
   suspend/resume, and a real battery. No row is validated today.
4. **Applications:** background/resume semantics, memory-pressure behaviour,
   and distinguishing simulated telephony/Android from real integration.
5. **Release engineering:** pin the kernel by digest and make real `mke2fs`
   images deterministic; download updates over HTTPS inside `nilupd`; a
   24-hour QEMU soak run; documented supported devices and accessibility
   checks.
6. **Key encryption:** `keygen` still derives the private-key wrapping key
   from a single SHA-256; replace with age/NaCl/scrypt for production.
