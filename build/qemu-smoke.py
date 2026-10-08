#!/usr/bin/env python3
"""Headless QEMU boot smoke test; succeeds only after PID 1 reports readiness."""
import os
import shutil
import subprocess
import sys
import tempfile
import time

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(TOP, "out", "x86_64-generic")
QEMU = shutil.which("qemu-system-x86_64")
KERNEL = os.path.join(OUT, "vmlinuz-lts")
INITRD = os.path.join(OUT, "nilos-initramfs.cpio.gz")


def main():
    if not QEMU:
        raise RuntimeError("qemu-system-x86_64 not found")
    for path in (KERNEL, INITRD):
        if not os.path.isfile(path):
            raise RuntimeError(f"Required image missing: {path}")

    with tempfile.TemporaryFile(mode="w+b") as output:
        command = [QEMU, "-m", "1024", "-smp", "2", "-kernel", KERNEL,
                   "-initrd", INITRD,
                   "-append", "console=ttyS0 init=/init panic=-1",
                   "-display", "none", "-monitor", "none", "-serial", "stdio",
                   "-no-reboot", "-no-shutdown"]
        proc = subprocess.Popen(command, stdin=subprocess.DEVNULL,
                                stdout=output, stderr=subprocess.STDOUT)
        deadline = time.monotonic() + 90
        try:
            while time.monotonic() < deadline:
                if proc.poll() is not None:
                    break
                time.sleep(1)
                output.seek(0)
                log = output.read().decode("utf-8", errors="replace")
                if "Kernel panic" in log or "VFS: Unable to mount root" in log:
                    raise RuntimeError("Kernel failed to boot; inspect QEMU output")
                if "Onuron OS boot completed" in log:
                    print("QEMU boot smoke passed: nilinit readiness observed")
                    return
            output.seek(0)
            log = output.read().decode("utf-8", errors="replace")
            print(log[-12000:], file=sys.stderr)
            raise RuntimeError("Timed out or QEMU exited before nilinit readiness")
        finally:
            if proc.poll() is None:
                proc.terminate()
                try:
                    proc.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    proc.kill()


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[ERROR] {exc}", file=sys.stderr)
        sys.exit(1)
