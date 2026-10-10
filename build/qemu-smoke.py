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

try:
    import target_registry
except ImportError:
    try:
        from . import target_registry
    except (ImportError, ValueError):
        sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
        import target_registry

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _resolve_smoke_defaults(target_name):
    canonical = target_registry.resolve_target(target_name, warn=False)
    out_dir = target_registry.get_target_output_dir(canonical)
    cfg = target_registry.load_target(canonical, warn=False)
    norm_arch = "aarch64" if canonical == "qemu-aarch64" else "x86_64"
    qemu_bin_name = "qemu-system-aarch64" if norm_arch == "aarch64" else "qemu-system-x86_64"
    qemu_bin = qemu_bin_name
    if sys.platform == "win32":
        default_win_qemu = os.path.join(os.environ.get("ProgramFiles", r"C:\Program Files"), "qemu", f"{qemu_bin_name}.exe")
        if os.path.exists(default_win_qemu) and not shutil.which(qemu_bin_name):
            qemu_bin = default_win_qemu

    initrd_name = "initramfs.cpio.gz" if norm_arch == "aarch64" else "nilos-initramfs.cpio.gz"
    kernel_name = cfg.get("kernel", {}).get("name", "vmlinuz-virt")
    disk_name = cfg.get("storage", {}).get("primary_disk", "data.img")

    c_kernel = os.path.join(out_dir, kernel_name)
    c_initrd = os.path.join(out_dir, initrd_name)
    c_disk = os.path.join(out_dir, disk_name)

    is_ci = os.environ.get("CI") == "true" or os.environ.get("GITHUB_ACTIONS") == "true"
    legacy_dir = os.path.join(TOP, "out", "aarch64-qemu" if norm_arch == "aarch64" else "x86_64-generic")

    if os.path.exists(c_kernel):
        kernel = c_kernel
    elif not is_ci and os.path.exists(os.path.join(legacy_dir, kernel_name)):
        print(f"[WARN] [qemu-smoke] Using legacy fallback kernel: {legacy_dir}/{kernel_name}", file=sys.stderr)
        kernel = os.path.join(legacy_dir, kernel_name)
    elif not is_ci and os.path.exists(os.path.join(legacy_dir, "vmlinuz-lts")):
        print(f"[WARN] [qemu-smoke] Using legacy fallback kernel: {legacy_dir}/vmlinuz-lts", file=sys.stderr)
        kernel = os.path.join(legacy_dir, "vmlinuz-lts")
    else:
        kernel = c_kernel

    if os.path.exists(c_initrd):
        initrd = c_initrd
    elif not is_ci and os.path.exists(os.path.join(legacy_dir, initrd_name)):
        print(f"[WARN] [qemu-smoke] Using legacy fallback initrd: {legacy_dir}/{initrd_name}", file=sys.stderr)
        initrd = os.path.join(legacy_dir, initrd_name)
    else:
        initrd = c_initrd

    if os.path.exists(c_disk):
        disk = c_disk
    elif not is_ci and os.path.exists(os.path.join(legacy_dir, disk_name)):
        print(f"[WARN] [qemu-smoke] Using legacy fallback disk: {legacy_dir}/{disk_name}", file=sys.stderr)
        disk = os.path.join(legacy_dir, disk_name)
    elif not is_ci and os.path.exists(os.path.join(legacy_dir, "nilos.img")):
        print(f"[WARN] [qemu-smoke] Using legacy fallback disk: {legacy_dir}/nilos.img", file=sys.stderr)
        disk = os.path.join(legacy_dir, "nilos.img")
    else:
        disk = c_disk

    return {
        "canonical_target": canonical,
        "out_dir": out_dir,
        "qemu_bin": qemu_bin,
        "kernel": kernel,
        "initrd": initrd,
        "disk": disk,
    }


ARCH_DEFAULTS = {
    "x86_64": _resolve_smoke_defaults("qemu-x86_64"),
    "aarch64": _resolve_smoke_defaults("qemu-aarch64"),
}

OUT = ARCH_DEFAULTS["x86_64"]["out_dir"]
DEFAULT_KERNEL = ARCH_DEFAULTS["x86_64"]["kernel"]
DEFAULT_INITRD = ARCH_DEFAULTS["x86_64"]["initrd"]


def build_qemu_cmd(qemu_bin, kernel_path, initrd_path, memory_mb=1024, smp=2, arch="x86_64", data_disk=None):
    """Builds the argument list for headless QEMU execution across architectures."""
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
            "-append", "console=ttyAMA0 earlycon root=/dev/ram0 rdinit=/init panic=-1 rw",
            "-display", "none",
            "-monitor", "none",
            "-serial", "stdio",
            "-no-reboot",
            "-no-shutdown",
        ]
    else:
        cmd = [
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
    if data_disk and os.path.exists(data_disk):
        cmd += [
            "-drive", f"file={data_disk},format=raw,if=none,id=vda_disk,cache=writeback",
            "-device", "virtio-blk-pci,drive=vda_disk,id=vda"
        ]
    return cmd


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
    if "Onuron OS boot failed" in log_text:
        return ("PANIC", "Onuron OS PID 1 boot failed: core services not operational")
    if "Onuron OS boot degraded" in log_text:
        return ("PANIC", "Onuron OS PID 1 boot degraded: core service readiness incomplete")
    if "Onuron OS boot completed" in log_text:
        if "core services verified healthy" in log_text:
            return ("SUCCESS", "Onuron OS PID 1 boot completed with all core services verified healthy")
        return ("PANIC", "Onuron OS boot completed without core services verified healthy")
    return ("IN_PROGRESS", "")


def run_smoke_test(kernel=None, initrd=None, timeout_secs=90, allow_skip=False, arch="x86_64", data_disk=None, target=None):
    target_name = target or arch
    canonical = target_registry.resolve_target(target_name, warn=False)
    norm_arch = "aarch64" if canonical == "qemu-aarch64" else "x86_64"
    defaults = ARCH_DEFAULTS.get(norm_arch, ARCH_DEFAULTS["x86_64"])
    k_path = kernel or defaults["kernel"]
    i_path = initrd or defaults["initrd"]
    d_path = data_disk or (defaults["disk"] if os.path.exists(defaults["disk"]) else None)

    qemu_bin_name = defaults["qemu_bin"]
    qemu_bin = shutil.which(qemu_bin_name)
    if not qemu_bin:
        msg = f"{qemu_bin_name} not found in PATH"
        if allow_skip:
            print(f"[SKIP] {msg}")
            return True
        raise RuntimeError(msg)

    for path in (k_path, i_path):
        if not os.path.isfile(path):
            msg = f"Required image missing: {path}"
            if allow_skip:
                print(f"[SKIP] {msg}")
                return True
            raise RuntimeError(msg)

    print(f"[qemu-smoke] Launching QEMU headless smoke test ({norm_arch}, timeout: {timeout_secs}s)...")
    print(f"             Kernel: {k_path}")
    print(f"             Initrd: {i_path}")
    if d_path:
        print(f"             Disk:   {d_path}")

    with tempfile.TemporaryFile(mode="w+b") as output:
        cmd = build_qemu_cmd(qemu_bin, k_path, i_path, arch=norm_arch, data_disk=d_path)
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
                    print(f"\x1b[1;32m[qemu-smoke] [ PASS ] ({norm_arch}) {details}\x1b[0m")
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
    parser.add_argument("--target", default=None, help="Target profile name or alias (e.g. qemu-x86_64, qemu-aarch64)")
    parser.add_argument("--arch", choices=["x86_64", "aarch64", "arm64"], default="x86_64",
                        help="Target architecture (x86_64 or aarch64, default: x86_64)")
    parser.add_argument("--kernel", default=None, help="Path to vmlinuz-lts")
    parser.add_argument("--initrd", default=None, help="Path to initramfs")
    parser.add_argument("--data-disk", default=None, help="Optional persistent data disk path")
    parser.add_argument("--timeout", type=int, default=90, help="Boot deadline in seconds")
    parser.add_argument("--allow-skip", action="store_true", help="Exit 0 if QEMU or images are missing")
    args = parser.parse_args()

    run_smoke_test(args.kernel, args.initrd, args.timeout, args.allow_skip, arch=args.arch, data_disk=args.data_disk, target=args.target)


if __name__ == "__main__":
    try:
        main()
    except Exception as exc:
        print(f"[ERROR] {exc}", file=sys.stderr)
        sys.exit(1)

