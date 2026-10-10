#!/usr/bin/env python3
"""
build/qemu-persistence-test.py — Automated End-to-End QEMU Reboot Persistence Test.

Verifies that:
1. /data is mounted from a real ext4 block device (not tmpfs).
2. A test payload is written to /data/persistence_test_marker.bin and synced to disk.
3. The virtual machine performs a clean shutdown/reboot.
4. On the subsequent boot, the marker file persists and matches byte-for-byte.
5. Fails immediately if /data is tmpfs, missing, or corrupted.

Usage:
    python build/qemu-persistence-test.py [--target TARGET] [--timeout-secs N] [--allow-skip]
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
except ImportError:
    try:
        from . import target_registry
    except (ImportError, ValueError):
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import target_registry

try:
    import mkdisk
except ImportError:
    try:
        from . import mkdisk
    except (ImportError, ValueError):
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import mkdisk

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def build_qemu_cmd(
    qemu_bin,
    kernel_path,
    initrd_path,
    disk_path,
    phase,
    memory_mb=1024,
    smp=2,
    arch="x86_64",
):
    """Builds QEMU command for persistence test execution."""
    norm_arch = "aarch64" if arch in ("aarch64", "arm64") else "x86_64"
    if norm_arch == "aarch64":
        cmd = [
            qemu_bin,
            "-M", "virt",
            "-cpu", "cortex-a57",
            "-m", str(memory_mb),
            "-smp", str(smp),
            "-kernel", kernel_path,
            "-initrd", initrd_path,
            "-append", f"console=ttyAMA0 earlycon root=/dev/ram0 rdinit=/init panic=-1 rw onuron.test_persistence={phase}",
            "-display", "none",
            "-monitor", "none",
            "-serial", "stdio",
            "-no-reboot",
        ]
    else:
        cmd = [
            qemu_bin,
            "-m", str(memory_mb),
            "-smp", str(smp),
            "-kernel", kernel_path,
            "-initrd", initrd_path,
            "-append", f"console=ttyS0 init=/init panic=-1 onuron.test_persistence={phase}",
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


def check_write_log(log_text):
    """
    Evaluates console output from Phase 1 (Write).
    Returns: ('SUCCESS', msg), ('FAIL', msg), or ('IN_PROGRESS', '').
    """
    if "Kernel panic" in log_text:
        return ("FAIL", "Kernel panic observed during write phase")
    if "VFS: Unable to mount root" in log_text:
        return ("FAIL", "Root filesystem mount failure observed")
    if "Persistence test failed: /data is not mounted" in log_text:
        return ("FAIL", "/data is not mounted from real ext block device (tmpfs fallback rejected)")
    if "Failed writing persistence marker" in log_text:
        return ("FAIL", "Write failure encountered while persisting marker to /data")
    if "Onuron OS boot failed" in log_text:
        return ("FAIL", "Onuron OS boot failed: core services not operational")
    if "Onuron OS boot degraded" in log_text:
        return ("FAIL", "Onuron OS boot degraded: core readiness incomplete")
    if "Persistence test WRITE phase completed successfully" in log_text:
        if "Persistence marker written and synced to /data" in log_text:
            return ("SUCCESS", "Persistence marker successfully written and synced to persistent disk")
        return ("FAIL", "Write phase completed without confirmed sync to disk")
    return ("IN_PROGRESS", "")


def check_verify_log(log_text):
    """
    Evaluates console output from Phase 2 (Verify after reboot).
    Returns: ('SUCCESS', msg), ('FAIL', msg), or ('IN_PROGRESS', '').
    """
    if "Kernel panic" in log_text:
        return ("FAIL", "Kernel panic observed during verify phase")
    if "VFS: Unable to mount root" in log_text:
        return ("FAIL", "Root filesystem mount failure observed")
    if "Persistence test verify failed: /data is tmpfs!" in log_text:
        return ("FAIL", "/data is tmpfs on reboot; persistence validation failed")
    if "Persistence marker corrupt" in log_text:
        return ("FAIL", "Persistence marker content corrupted after reboot")
    if "Persistence marker missing after reboot" in log_text:
        return ("FAIL", "Persistence marker missing after reboot (storage did not persist)")
    if "Onuron OS boot failed" in log_text:
        return ("FAIL", "Onuron OS boot failed: core services not operational")
    if "Onuron OS boot degraded" in log_text:
        return ("FAIL", "Onuron OS boot degraded: core readiness incomplete")
    if "Persistence test VERIFY phase completed successfully" in log_text:
        if "payload matches byte-for-byte" in log_text:
            return ("SUCCESS", "Persistence marker verified across reboot: payload matches byte-for-byte")
        return ("FAIL", "Verify phase completed without exact byte match")
    return ("IN_PROGRESS", "")


def run_qemu_phase(cmd, evaluator, timeout_secs, phase_name):
    """Runs a single QEMU phase, evaluating logs and verifying process exit."""
    with tempfile.TemporaryFile(mode="w+b") as output:
        proc = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + timeout_secs
        success_msg = None

        while time.monotonic() < deadline:
            output.seek(0)
            log = output.read().decode("utf-8", errors="replace")
            status, details = evaluator(log)

            if status == "FAIL":
                try:
                    proc.kill()
                except Exception:
                    pass
                raise RuntimeError(f"[{phase_name}] Failure: {details}\n--- Log ---\n{log}")

            if status == "SUCCESS":
                success_msg = details
                # Wait for QEMU to cleanly terminate after poweroff
                try:
                    proc.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    proc.kill()
                return success_msg

            if proc.poll() is not None:
                # Process exited before success
                break
            time.sleep(1)

        # Check final output if process exited or timed out
        output.seek(0)
        log = output.read().decode("utf-8", errors="replace")
        status, details = evaluator(log)
        if status == "SUCCESS":
            return details
        if status == "FAIL":
            raise RuntimeError(f"[{phase_name}] Failure: {details}\n--- Log ---\n{log}")
        try:
            proc.kill()
        except Exception:
            pass
        raise TimeoutError(f"[{phase_name}] Timed out after {timeout_secs}s without reaching persistence marker checkpoint.\n--- Log ---\n{log}")


def run_persistence_test(
    kernel=None,
    initrd=None,
    disk=None,
    timeout_secs=60,
    allow_skip=False,
    arch="x86_64",
    target=None,
):
    target_name = target or arch
    canonical = target_registry.resolve_target(target_name, warn=False)
    norm_arch = "aarch64" if canonical == "qemu-aarch64" else "x86_64"
    out_dir = target_registry.get_target_output_dir(canonical)
    cfg = target_registry.load_target(canonical, warn=False)
    kernel_name = cfg.get("kernel", {}).get("name", "vmlinuz-virt")

    qemu_bin_name = "qemu-system-aarch64" if norm_arch == "aarch64" else "qemu-system-x86_64"
    qemu_bin = shutil.which(qemu_bin_name)
    if not qemu_bin and sys.platform == "win32":
        default_win_qemu = os.path.join(os.environ.get("ProgramFiles", r"C:\Program Files"), "qemu", f"{qemu_bin_name}.exe")
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

    # Prepare persistent test disk
    temp_disk = None
    if disk and os.path.isfile(disk):
        d_path = disk
    else:
        # Create a fresh 64MB disk for the test run (real ext4 if mke2fs available)
        fd, temp_disk = tempfile.mkstemp(prefix="onuron_persist_", suffix=".img")
        os.close(fd)
        if mkdisk.mke2fs_path():
            mkdisk.build_real_image(temp_disk, size_mb=64)
        else:
            mkdisk.build_synthetic_image(temp_disk, size_mb=64)
        d_path = temp_disk

    try:
        print(f"[qemu-persistence] Phase 1: Booting QEMU to write marker ({norm_arch})...")
        cmd_write = build_qemu_cmd(qemu_bin, k_path, i_path, d_path, phase="write", arch=norm_arch)
        res_write = run_qemu_phase(cmd_write, check_write_log, timeout_secs, "Phase 1: Write")
        print(f"  [ PASS ] {res_write}")

        print(f"[qemu-persistence] Phase 2: Rebooting QEMU with same disk to verify marker ({norm_arch})...")
        cmd_verify = build_qemu_cmd(qemu_bin, k_path, i_path, d_path, phase="verify", arch=norm_arch)
        res_verify = run_qemu_phase(cmd_verify, check_verify_log, timeout_secs, "Phase 2: Verify")
        print(f"  [ PASS ] {res_verify}")

        print(f"\x1b[1;32m[qemu-persistence] [ SUCCESS ] Full reboot persistence cycle verified across reboots!\x1b[0m")
        return True
    finally:
        if temp_disk and os.path.exists(temp_disk):
            try:
                os.remove(temp_disk)
            except OSError:
                pass


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", default="qemu-x86_64", help="target name (default: qemu-x86_64)")
    parser.add_argument("--kernel", help="path to kernel binary")
    parser.add_argument("--initrd", help="path to initramfs")
    parser.add_argument("--disk", help="path to disk image")
    parser.add_argument("--timeout-secs", type=int, default=60, help="per-phase timeout in seconds")
    parser.add_argument("--allow-skip", action="store_true", help="exit cleanly if images/qemu missing")
    args = parser.parse_args(argv)

    try:
        success = run_persistence_test(
            kernel=args.kernel,
            initrd=args.initrd,
            disk=args.disk,
            timeout_secs=args.timeout_secs,
            allow_skip=args.allow_skip,
            target=args.target,
        )
        sys.exit(0 if success else 1)
    except Exception as exc:
        print(f"\x1b[1;31m[qemu-persistence] [ FAILED ] {exc}\x1b[0m", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
