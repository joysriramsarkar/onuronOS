#!/usr/bin/env python3
"""
build/qemu-recovery-test.py — Automated QEMU Recovery-Path Test.

Proves the recovery boot path end to end against a real boot image (same
kernel + initramfs as the smoke test) on a live QEMU machine:

  Phase "flag"    : `onuron.recovery=1` on the kernel command line must drop
                    PID 1 into `nilrecovery` (reason + menu banner), and the
                    'q' action must leave recovery and reboot the guest.
  Phase "counter" : a /data image seeded with two previous failed boots must
                    enter recovery automatically on the next boot.
  Phase "reset"   : from that recovery menu, factory reset confirmed with
                    'YES' must wipe /data in place (no mkfs tool ships in the
                    image) and clear the failed-boot counter.
  Phase "reboot"  : after the reset, the same disk must boot NORMALLY again
                    (persistent storage mounted, boot completed) — proving the
                    recovery loop is broken and the disk was not bricked.
  Phase "corrupt" : a garbage disk image must produce the loud
                    'CORRUPT DISK DETECTED' warning (never the silent
                    no-disk tmpfs fallback) while the system still reaches
                    'boot completed', and no legitimate IPC connection may be
                    denied by the nilsd policy on the way up.

The decision logic itself is pure and unit-tested in
`nilinit/src/recovery.rs`; this harness proves the plumbing on a live boot:
the counter file on a real ext4 disk, the kernel command line, the shipped
`nilrecovery` binary (mkinitramfs bins list), and the reboot path.

Usage:
    python build/qemu-recovery-test.py [--target TARGET] [--timeout-secs N] [--allow-skip]
"""
import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time

try:
    import target_registry
    import mkdisk
except ImportError:
    try:
        from . import target_registry, mkdisk
    except (ImportError, ValueError):
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import target_registry
        import mkdisk

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

# Console strings the evaluators look for. These must match the output of
# nilinit (`enter_recovery`, `mount_data_partition`, the boot completion log)
# and nilrecovery's menu exactly; see nilinit/src/main.rs and
# runtime/nilrt/src/bin/nilrecovery.rs.
RECOVERY_REQUESTED_MSG = "Entering recovery mode: recovery requested on the kernel command line"
RECOVERY_COUNTER_MSG = "Entering recovery mode: 3 consecutive failed boots (limit 3)"
RECOVERY_BANNER = "Onuron Recovery System"
RECOVERY_MISSING_MSG = "Could not start /usr/bin/nilrecovery"
FACTORY_RESET_DONE = "Factory reset complete."
FACTORY_RESET_CANCELLED = "Factory reset cancelled (no confirmation)"
FACTORY_RESET_FAILED = "Factory reset failed"
CORRUPT_DISK_MSG = "CORRUPT DISK DETECTED"
SILENT_NODISK_MSG = "No persistent disk found — /data is tmpfs"
STORAGE_MOUNTED_MSG = "Persistent storage mounted:"
BOOT_COMPLETED_MSG = "Onuron OS boot completed"
BOOT_FAILED_MSG = "Onuron OS boot failed"
BOOT_DEGRADED_MSG = "Onuron OS boot degraded"
IPC_DENIAL_MSG = "nilsd:auth] denied"

# stdin payloads driving the recovery menu where a phase reaches it.
STDIN_PAYLOADS = {
    "flag": b"q\n",        # leave recovery immediately
    "counter": b"q\n",     # same seeded disk is reused by the "reset" phase
    "reset": b"3\nYES\n",  # factory reset, explicitly confirmed
    "corrupt": None,
    "reboot": None,
}

# Which phases expect the guest to reboot (QEMU exits thanks to -no-reboot)
# instead of being killed by the harness after the verdict.
EXPECT_EXIT = {
    "flag": True,
    "counter": True,
    "reset": True,
    "corrupt": False,
    "reboot": False,
}


def build_qemu_cmd(qemu_bin, kernel_path, initrd_path, disk_path, phase,
                   memory_mb=1024, smp=2, arch="x86_64"):
    """Builds the QEMU command for one recovery-test phase."""
    norm_arch = "aarch64" if arch in ("aarch64", "arm64") else "x86_64"
    console = "ttyAMA0" if norm_arch == "aarch64" else "ttyS0"
    # The recovery-entry phases stop before services start, so they need no
    # readiness budget; the two full-boot phases keep the same generous budget
    # the smoke and persistence harnesses use.
    if phase == "flag":
        append = f"console={console} init=/init panic=-1 onuron.recovery=1"
    elif phase in ("counter", "reset"):
        append = f"console={console} init=/init panic=-1"
    else:
        append = f"console={console} init=/init panic=-1 onuron.readiness_ms=45000"

    cmd = [
        qemu_bin,
        "-m", str(memory_mb),
        "-smp", str(smp),
        # Pin a modern virtual CPU (see build/qemu-smoke.py for the reasoning
        # behind not trusting the host QEMU's default model).
        "-cpu", "cortex-a57" if norm_arch == "aarch64" else "max",
    ]
    if norm_arch == "aarch64":
        cmd += ["-M", "virt"]
    cmd += [
        "-kernel", kernel_path,
        "-initrd", initrd_path,
        "-append", append,
        "-display", "none",
        "-monitor", "none",
        "-serial", "stdio",
        "-no-reboot",
    ]
    cmd += [
        "-drive", f"file={disk_path},format=raw,if=none,id=vda_disk,cache=writeback",
        "-device", "virtio-blk-pci,drive=vda_disk,id=vda",
    ]
    return cmd


def make_counter_seed_dir(seed_root, failed_boots=2):
    """Lay out the populate-dir that seeds /data with a failed-boot count.

    `mke2fs -d` copies this tree onto the filesystem, so the disk boots with
    /data/system/boot_count == failed_boots, exactly what nilinit's
    read_boot_count expects after two boots that never completed.
    """
    system_dir = os.path.join(seed_root, "system")
    os.makedirs(system_dir, exist_ok=True)
    with open(os.path.join(system_dir, "boot_count"), "w") as f:
        f.write(str(failed_boots))
    return seed_root


def build_corrupt_image(path, size_mb=64):
    """A block device image that exists but carries no valid filesystem.

    Random garbage covers the first 4 KiB (the ext superblock region), so
    every mount attempt fails and nilinit must take the loud CorruptDisk
    path instead of the silent 'no persistent disk found' one.
    """
    with open(path, "wb") as f:
        f.seek(size_mb * 1024 * 1024 - 1)
        f.write(b"\x00")
        f.seek(0)
        f.write(os.urandom(4096))
    return path


def _common_failures(log):
    if "Kernel panic" in log:
        return ("FAIL", "Kernel panic observed")
    if "VFS: Unable to mount root" in log:
        return ("FAIL", "Root filesystem mount failure observed")
    if RECOVERY_MISSING_MSG in log:
        return (
            "FAIL",
            "nilrecovery is not shipped in the image (check the mkinitramfs bins list)",
        )
    return None


def check_flag_log(log):
    """Phase 'flag': explicit onuron.recovery=1 must reach the menu."""
    common = _common_failures(log)
    if common:
        return common
    if RECOVERY_REQUESTED_MSG in log:
        if RECOVERY_BANNER in log and "Leaving recovery. The device will reboot." in log:
            return ("SUCCESS", "explicit recovery flag entered the menu; 'q' left it for reboot")
        return ("IN_PROGRESS", "")
    if BOOT_COMPLETED_MSG in log and RECOVERY_REQUESTED_MSG not in log:
        return ("FAIL", "boot ignored onuron.recovery=1 and continued normally")
    return ("IN_PROGRESS", "")


def check_counter_log(log):
    """Phase 'counter': two previous failed boots must trigger recovery."""
    common = _common_failures(log)
    if common:
        return common
    if RECOVERY_COUNTER_MSG in log:
        if RECOVERY_BANNER in log and "Leaving recovery. The device will reboot." in log:
            return ("SUCCESS", "failed-boot counter triggered recovery; 'q' left it for reboot")
        return ("IN_PROGRESS", "")
    if BOOT_COMPLETED_MSG in log and RECOVERY_COUNTER_MSG not in log:
        return (
            "FAIL",
            "seeded failed-boot counter did not trigger recovery "
            "(check the /data/system/boot_count seeding)",
        )
    return ("IN_PROGRESS", "")


def check_reset_log(log):
    """Phase 'reset': factory reset confirmed with YES must complete."""
    common = _common_failures(log)
    if common:
        return common
    if FACTORY_RESET_FAILED in log:
        return ("FAIL", "factory reset reported failure")
    if FACTORY_RESET_CANCELLED in log:
        return ("FAIL", "factory reset was not confirmed — 'YES' did not reach the prompt")
    if RECOVERY_COUNTER_MSG in log and FACTORY_RESET_DONE in log:
        return ("SUCCESS", "factory reset completed from the recovery menu (in-place wipe)")
    if BOOT_COMPLETED_MSG in log and FACTORY_RESET_DONE not in log:
        return ("FAIL", "boot continued normally instead of entering recovery for the reset")
    return ("IN_PROGRESS", "")


def check_reboot_log(log):
    """Phase 'reboot': after the reset the same disk must boot normally."""
    common = _common_failures(log)
    if common:
        return common
    if "Entering recovery mode" in log:
        return ("FAIL", "device re-entered recovery after factory reset (counter not cleared)")
    if CORRUPT_DISK_MSG in log or SILENT_NODISK_MSG in log:
        return ("FAIL", "disk no longer mounts cleanly after the in-place wipe")
    if IPC_DENIAL_MSG in log:
        return ("FAIL", "a legitimate service was denied by the IPC policy on a live boot")
    if BOOT_FAILED_MSG in log:
        return ("FAIL", "boot failed after factory reset")
    if BOOT_DEGRADED_MSG in log:
        return ("FAIL", "boot degraded after factory reset")
    if BOOT_COMPLETED_MSG in log:
        if STORAGE_MOUNTED_MSG in log:
            return ("SUCCESS", "post-reset boot mounted persistent storage and completed normally")
        return ("FAIL", "boot completed but persistent storage was not mounted")
    return ("IN_PROGRESS", "")


def check_corrupt_log(log):
    """Phase 'corrupt': garbage disk must warn loudly and still boot fully."""
    common = _common_failures(log)
    if common:
        return common
    if SILENT_NODISK_MSG in log:
        return (
            "FAIL",
            "corrupt disk was masked as 'no persistent disk found' — silent tmpfs fallback",
        )
    if IPC_DENIAL_MSG in log:
        return ("FAIL", "a legitimate service was denied by the IPC policy on a live boot")
    if BOOT_COMPLETED_MSG in log and CORRUPT_DISK_MSG not in log:
        return ("FAIL", "boot completed without reporting the corrupt disk")
    if CORRUPT_DISK_MSG in log:
        if BOOT_FAILED_MSG in log:
            return ("FAIL", "boot failed entirely after the corrupt-disk warning")
        if BOOT_DEGRADED_MSG in log:
            return (
                "FAIL",
                "boot degraded after the corrupt-disk warning (emergency tmpfs must stay operational)",
            )
        if BOOT_COMPLETED_MSG in log:
            return (
                "SUCCESS",
                "corrupt disk loudly reported; system reached full boot on emergency tmpfs",
            )
    return ("IN_PROGRESS", "")


PHASE_EVALUATORS = {
    "flag": check_flag_log,
    "counter": check_counter_log,
    "reset": check_reset_log,
    "reboot": check_reboot_log,
    "corrupt": check_corrupt_log,
}


def run_qemu_phase(cmd, evaluator, timeout_secs, phase_name, stdin_payload=None, expect_exit=True):
    """Runs a single QEMU phase, evaluating the console log until the
    evaluator reaches a verdict.

    Menu input is written to QEMU's stdin only once the recovery banner is
    visible on the console, and the pipe is kept open afterwards. Both
    details are load-bearing, established by direct experiment: bytes sent
    at process start are dropped by the guest before its 8250 driver is up,
    and closing the pipe right after writing makes QEMU tear the stdin watch
    down on EOF — the menu then blocks forever either way.

    `expect_exit` phases additionally require the guest to reboot on its own
    (QEMU exits thanks to -no-reboot) rather than being killed by the harness;
    a guest that stays parked after the recovery action is a failure.
    """
    with tempfile.TemporaryFile(mode="w+b") as output:
        stdin_arg = subprocess.PIPE if stdin_payload is not None else subprocess.DEVNULL
        proc = subprocess.Popen(cmd, stdin=stdin_arg, stdout=output, stderr=subprocess.STDOUT)
        payload_sent = False

        deadline = time.monotonic() + timeout_secs
        while time.monotonic() < deadline:
            output.seek(0)
            log = output.read().decode("utf-8", errors="replace")

            if (
                not payload_sent
                and stdin_payload is not None
                and RECOVERY_BANNER in log
            ):
                try:
                    proc.stdin.write(stdin_payload)
                    proc.stdin.flush()
                except (BrokenPipeError, OSError):
                    pass
                payload_sent = True

            status, details = evaluator(log)

            if status == "FAIL":
                _close_stdin(proc)
                try:
                    proc.kill()
                except Exception:
                    pass
                raise RuntimeError(f"[{phase_name}] Failure: {details}\n--- Log ---\n{log}")

            if status == "SUCCESS":
                return _finish_phase(proc, output, expect_exit, phase_name, details)

            if proc.poll() is not None:
                break
            time.sleep(1)

        output.seek(0)
        log = output.read().decode("utf-8", errors="replace")
        if not payload_sent and stdin_payload is not None and RECOVERY_BANNER in log:
            try:
                proc.stdin.write(stdin_payload)
                proc.stdin.flush()
            except (BrokenPipeError, OSError):
                pass
            payload_sent = True
        status, details = evaluator(log)
        if status == "SUCCESS":
            return _finish_phase(proc, output, expect_exit, phase_name, details)
        if status == "FAIL":
            _close_stdin(proc)
            try:
                proc.kill()
            except Exception:
                pass
            raise RuntimeError(f"[{phase_name}] Failure: {details}\n--- Log ---\n{log}")
        _close_stdin(proc)
        try:
            proc.kill()
        except Exception:
            pass
        raise TimeoutError(f"[{phase_name}] Timed out after {timeout_secs}s.\n--- Log ---\n{log}")


def _close_stdin(proc):
    """Close the harness side of QEMU's stdin pipe (signals EOF on teardown)."""
    if proc.stdin is not None and not proc.stdin.closed:
        try:
            proc.stdin.close()
        except (BrokenPipeError, OSError):
            pass


def _finish_phase(proc, output, expect_exit, phase_name, details):
    """Wait for the guest to finish after a SUCCESS verdict.

    Rebooting phases must reboot (QEMU exits with -no-reboot) within a grace
    window; full-boot phases without a shutdown hook are simply stopped, the
    same tolerance the persistence harness applies after its verdicts.
    """
    if expect_exit:
        try:
            proc.wait(timeout=30)
            return details
        except subprocess.TimeoutExpired:
            output.seek(0)
            log = output.read().decode("utf-8", errors="replace")
            try:
                proc.kill()
            except Exception:
                pass
            raise RuntimeError(
                f"[{phase_name}] Guest did not reboot after the recovery action: {details}\n"
                f"--- Log ---\n{log}"
            )
    try:
        proc.wait(timeout=10)
    except subprocess.TimeoutExpired:
        try:
            proc.kill()
        except Exception:
            pass
    return details


def run_recovery_test(kernel=None, initrd=None, timeout_secs=120,
                      allow_skip=False, target=None):
    target_name = target or "qemu-x86_64"
    canonical = target_registry.resolve_target(target_name, warn=False)
    norm_arch = "aarch64" if canonical == "qemu-aarch64" else "x86_64"
    out_dir = target_registry.get_target_output_dir(canonical)
    cfg = target_registry.load_target(canonical, warn=False)
    kernel_name = cfg.get("kernel", {}).get("name", "vmlinuz-virt")

    qemu_bin_name = "qemu-system-aarch64" if norm_arch == "aarch64" else "qemu-system-x86_64"
    qemu_bin = shutil.which(qemu_bin_name)
    if not qemu_bin and sys.platform == "win32":
        default_win_qemu = os.path.join(
            os.environ.get("ProgramFiles", r"C:\Program Files"), "qemu",
            f"{qemu_bin_name}.exe",
        )
        if os.path.exists(default_win_qemu):
            qemu_bin = default_win_qemu

    if not qemu_bin:
        msg = f"{qemu_bin_name} not found in PATH"
        if allow_skip:
            print(f"[SKIP] {msg}")
            return True
        raise RuntimeError(msg)

    initrd_name = "initramfs.cpio.gz" if norm_arch == "aarch64" else "nilos-initramfs.cpio.gz"
    k_path = kernel or os.path.join(out_dir, kernel_name)
    i_path = initrd or os.path.join(out_dir, initrd_name)

    for path in (k_path, i_path):
        if not os.path.isfile(path):
            msg = f"Required image missing: {path}"
            if allow_skip:
                print(f"[SKIP] {msg}")
                return True
            raise RuntimeError(msg)

    if not mkdisk.mke2fs_path():
        msg = "mke2fs is required to build the fresh and seeded /data images"
        if allow_skip:
            print(f"[SKIP] {msg}")
            return True
        raise RuntimeError(msg)

    tempdir = tempfile.mkdtemp(prefix="onuron_recovery_")
    disk_fresh = os.path.join(tempdir, "fresh.img")
    disk_seeded = os.path.join(tempdir, "seeded.img")
    disk_corrupt = os.path.join(tempdir, "corrupt.img")
    try:
        # Three disks: healthy and unseeded, seeded with two failed boots
        # (reused across counter -> reset -> reboot), and pure garbage.
        mkdisk.build_real_image(disk_fresh, size_mb=64)
        seed_dir = make_counter_seed_dir(os.path.join(tempdir, "seed"))
        mkdisk.build_real_image(disk_seeded, size_mb=64, populate_dir=seed_dir)
        build_corrupt_image(disk_corrupt, size_mb=64)

        # Recovery-entry boots stop before services start (fast); the two
        # full boots carry the 45s readiness budget, so they get more room.
        budgets = {"flag": timeout_secs, "counter": timeout_secs, "reset": timeout_secs,
                   "reboot": timeout_secs * 2, "corrupt": timeout_secs * 2}
        disks = {"flag": disk_fresh, "counter": disk_seeded, "reset": disk_seeded,
                 "reboot": disk_seeded, "corrupt": disk_corrupt}

        for phase in ("flag", "counter", "reset", "reboot", "corrupt"):
            print(f"[qemu-recovery] Phase '{phase}': booting QEMU ({norm_arch})...")
            cmd = build_qemu_cmd(qemu_bin, k_path, i_path, disks[phase],
                                 phase=phase, arch=norm_arch)
            res = run_qemu_phase(
                cmd, PHASE_EVALUATORS[phase], budgets[phase], f"Phase '{phase}'",
                stdin_payload=STDIN_PAYLOADS.get(phase),
                expect_exit=EXPECT_EXIT[phase],
            )
            print(f"  [ PASS ] {res}")

        print(
            "\x1b[1;32m[qemu-recovery] [ SUCCESS ] Recovery path verified: explicit flag, "
            "failed-boot counter, in-place factory reset, post-reset normal boot, "
            "corrupt-disk warning.\x1b[0m"
        )
        return True
    finally:
        shutil.rmtree(tempdir, ignore_errors=True)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", default="qemu-x86_64", help="target name (default: qemu-x86_64)")
    parser.add_argument("--kernel", help="path to kernel binary")
    parser.add_argument("--initrd", help="path to initramfs")
    parser.add_argument("--timeout-secs", type=int, default=120,
                        help="per-phase timeout in seconds (full-boot phases get 2x)")
    parser.add_argument("--allow-skip", action="store_true",
                        help="exit cleanly if images/qemu/mke2fs are missing")
    args = parser.parse_args(argv)

    try:
        success = run_recovery_test(
            kernel=args.kernel,
            initrd=args.initrd,
            timeout_secs=args.timeout_secs,
            allow_skip=args.allow_skip,
            target=args.target,
        )
        sys.exit(0 if success else 1)
    except Exception as exc:
        print(f"\x1b[1;31m[qemu-recovery] [ FAILED ] {exc}\x1b[0m", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
