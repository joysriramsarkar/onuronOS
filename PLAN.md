# onuronOS — প্ল্যান (মোবাইল OS)

> Rust userspace, Linux LTS 6.6, AGPL-3.0। Phase 1–2 COMPLETE, Phase 3 ACTIVE। নিয়ম: `AGENTS.md` — প্রমাণ ছাড়া maturity আপগ্রেড নয়; simulated হলে ব্যাজ।

## বর্তমান অবস্থা
- nilinit PID1 (mount/cgroup/SELinux/services.toml/supervision/recovery) + 30 সার্ভিস (policy-lib + `ONUR` IPC) + QEMU headless smoke (`boot completed` + healthy) + ext4 persistence (2-boot) — সব unit-tested, QEMU-স্তরে বিশ্বাসযোগ্য।
- HAL পাতলা (C-ABI) + `runtime/nilhal` + FakeHAL; shell dual (Rust OOBE/PIN + legacy C); `nilui-gpu` DRM dumb-buffer।
- ফাঁক: recovery/no-silent-tmpfs, live-boot IPC audit/sandbox-attack, nilkeyd→nilinit fscrypt wiring, ref-board NOT VALIDATED, nilupd HTTPS/power-cut, 24h soak, Android-compat placeholder।

## পরের 6 (docs/improvement-plan ক্রমে)
1. `linux-qemu.yml` clean-runner-এ সবুজ (musl + `cargo test` + `qemu-smoke.py`) + failing-test gate।
2. Recovery QEMU-তে: `onuron.recovery=1`, 3-fail counter, corrupt-disk warning (silent tmpfs নয়)।
3. Live-boot IPC audit + sandbox attack (CWD/proc/sys/ptrace/device leak বন্ধ, legitimate denial ফিক্স)।
4. nilkeyd→nilinit fscrypt (real `/data` + PIN-rewrap + recovery doc)।
5. `qemu-aarch64 -M virt` বুট + `docs/reference-board.md` HW matrix শুরু।
6. Release hardening: nilupd HTTPS + power-cut test, kernel digest-pin, 24h soak (RSS/FD/proc), `mke2fs` determinism।

## DoD
- `cargo test` + `qemu-smoke.py` + persistence + recovery সবুজ; `docs/maturity.toml` + `gen-maturity.py --check` পাস।

## এই সেশনে
- [ ] `cargo test` (দ্রুত) + ফলাফল রেকর্ড; QEMU শুধু পরিবেশ থাকলে।
- [x] 13 Oct: `nilsim/` আর্কাইভ প্রস্তাব **বাতিল (ADR)** — Rust desktop sim বনাম Go app-runtime emulator ভিন্ন স্তর; দেখুন `onuron_project/specs/simulation.md`।
