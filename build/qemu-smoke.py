#!/usr/bin/env python3
"""
build/qemu-smoke.py — Automated Headless QEMU Boot Smoke Test.
Verifies that the Linux kernel, initramfs, and nilinit (PID 1) boot successfully
and reach the operational supervisor state.
"""
import argparse
import os
import shutil
import subprocess
import sys
import tempfile
import time

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(TOP, "out", "x86_64-generic")
DEFAULT_KERNEL = os.path.join(OUT, "vmlinuz-lts")
DEFAULT_INITRD = os.path.join(OUT, "nilos-initramfs.cpio.gz")


def build_qemu_cmd(qemu_bin, kernel_path, initrd_path, memory_mb=1024, smp=2):
    """Builds the argument list for headless QEMU execution."""
    return [
        qemu_bin,
        "-m", str(memory_mb),
        "-smp", str(smp),
        "-kernel", kernel_path,
        "-initrd", initrd_path,
        "-append", "console=ttyS0 init=/init panic=-1",
        "-display", "none",
        "-monitor", "none",
        "-serial", "stdio",
        "-no-reboot",
        "-no-shutdown",
    ]


def check_boot_log(log_text):
    """
    Evaluates console output from QEMU.
    Returns:
      ('SUCCESS', message) when nilinit reports readiness
      ('PANIC', message) when kernel panics or fails to mount root
      ('IN_PROGRESS', '') when boot is still underway
    """
    if "Kernel panic" in log_text:
        return ("PANIC", "Kernel panic observed in console log")
    if "VFS: Unable to mount root" in log_text:
        return ("PANIC", "Root filesystem mount failure observed")
    if "Onuron OS boot completed" in log_text:
        return ("SUCCESS", "Onuron OS PID 1 boot completed successfully")
    return ("IN_PROGRESS", "")


def run_smoke_test(kernel, initrd, timeout_secs=90, allow_skip=False):
    qemu_bin = shutil.which("qemu-system-x86_64")
    if not qemu_bin:
        msg = "qemu-system-x86_64 not found in PATH"
        if allow_skip:
            print(f"[SKIP] {msg}")
            return True
        raise RuntimeError(msg)

    for path in (kernel, initrd):
        if not os.path.isfile(path):
            msg = f"Required image missing: {path}"
            if allow_skip:
                print(f"[SKIP] {msg}")
                return True
            raise RuntimeError(msg)

    print(f"[qemu-smoke] Launching QEMU headless smoke test (timeout: {timeout_secs}s)...")
    print(f"             Kernel: {kernel}")
    print(f"             Initrd: {initrd}")

    with tempfile.TemporaryFile(mode="w+b") as output:
        cmd = build_qemu_cmd(qemu_bin, kernel, initrd)
        proc = subprocess.Popen(cmd, stdin=subprocess.DEVNULL, stdout=output, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + timeout_secs

        try:
            while time.monotonic() < deadline:
                if proc.poll() is not None:
                    break
                time.sleep(1)
                output.seek(0)
                log = output.read().decode("utf-8", errors="replace")
                status, details = check_boot_log(log)
                if status == "PANIC":
                    raise RuntimeError(f"Kernel boot failure: {details}")
                if status == "SUCCESS":
                    print(f"\x1b[1;32m[qemu-smoke] [ PASS ] {details}\x1b[0m")
                    return True

            output.seek(0)
            log = output.read().decode("utf-8", errors="replace")
            print(log[-12000:], file=sys.stderr)
            raise RuntimeError(f"Timed out after {timeout_secs}s or QEMU exited before readiness")
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()


def main():
    parser = argparse.ArgumentParser(description="Headless QEMU boot smoke test harness")
    parser.add_argument("--kernel", default=DEFAULT_KERNEL, help="Path to vmlinuz-lts")
    parser.add_argument("--initrd", default=DEFAULT_INITRD, help="Path to nilos-initramfs.cpio.gz")
    parser.add_argument("--timeout", type=int, default=90, help="Boot deadline in seconds")
    parser.add_argument("--allow-skip", action="store_true", help="Exit 0 if QEMU or images are missing")
    args = parser.parse_args()

    run_smoke_test(args.kernel, args.initrd, args.timeout, args.allow_skip)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[ERROR] {exc}", file=sys.stderr)
        sys.exit(1)
