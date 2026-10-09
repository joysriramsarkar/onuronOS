#!/usr/bin/env python3
"""
build/mkinitramfs.py — NilOS Standalone Initramfs & Image Generator
Creates a Linux-compatible SVR4 (070701) cpio.gz initramfs for QEMU x86_64 boot.
Runs completely cross-platform without requiring external 'cpio' or 'mknod'.
"""

import os
import sys
import gzip
import hashlib
import io
import shutil
import urllib.request
import argparse

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(TOP, "out", "x86_64-generic")
ROOTFS = os.path.join(OUT, "rootfs")
KERNEL_PATH = os.path.join(OUT, "vmlinuz-lts")
INITRD_PATH = os.path.join(OUT, "nilos-initramfs.cpio.gz")

KERNEL_URL = "https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/netboot/vmlinuz-lts"


def ensure_kernel(skip_download=False):
    os.makedirs(OUT, exist_ok=True)
    if os.path.exists(KERNEL_PATH) and os.path.getsize(KERNEL_PATH) > 1000000:
        print(f"[OK] Kernel present: {KERNEL_PATH} ({os.path.getsize(KERNEL_PATH)} bytes)")
        return
    if skip_download:
        raise RuntimeError(f"Kernel missing at {KERNEL_PATH}; provide a pre-fetched kernel")
    print(f"==> Downloading Linux LTS kernel for QEMU from:\n    {KERNEL_URL}")
    try:
        urllib.request.urlretrieve(KERNEL_URL, KERNEL_PATH)
        print(f"[OK] Downloaded kernel: {KERNEL_PATH} ({os.path.getsize(KERNEL_PATH)} bytes)")
    except Exception as e:
        print(f"[WARN] Failed to download kernel automatically: {e}")
        print("       Please download vmlinuz-lts into out/x86_64-generic/")


class CpioWriter:
    """Writes SVR4 portable cpio (070701) archives."""
    def __init__(self, f):
        self.f = f
        self.ino = 1

    def add_entry(self, name, mode, content=b"", rdevmajor=0, rdevminor=0):
        name_bytes = name.encode('utf-8') + b'\x00'
        namesize = len(name_bytes)
        filesize = len(content)

        header = (
            f"070701"
            f"{self.ino:08X}"
            f"{mode:08X}"
            f"{0:08X}"         # uid
            f"{0:08X}"         # gid
            f"{1:08X}"         # nlink
            f"{1700000000:08X}" # mtime
            f"{filesize:08X}"
            f"{0:08X}"         # devmajor
            f"{0:08X}"         # devminor
            f"{rdevmajor:08X}" # rdevmajor
            f"{rdevminor:08X}" # rdevminor
            f"{namesize:08X}"
            f"{0:08X}"         # check
        ).encode('ascii')

        self.ino += 1
        self.f.write(header)
        self.f.write(name_bytes)
        name_pad = (4 - ((110 + namesize) % 4)) % 4
        if name_pad > 0:
            self.f.write(b'\x00' * name_pad)

        if filesize > 0:
            self.f.write(content)
            data_pad = (4 - (filesize % 4)) % 4
            if data_pad > 0:
                self.f.write(b'\x00' * data_pad)

    def close(self):
        self.add_entry("TRAILER!!!", 0)
        curr_pos = self.f.tell()
        pad = (512 - (curr_pos % 512)) % 512
        if pad > 0:
            self.f.write(b'\x00' * pad)


def gzip_compress(data, compresslevel=6):
    """Deterministically gzip `data` (mtime pinned to 0, no embedded filename).

    ``gzip.compress`` defaults to stamping the current wall-clock time into the
    gzip header, which makes byte-for-byte reproducibility impossible. We write
    through a ``BytesIO`` (so no ``FNAME`` field is emitted) and force
    ``mtime=0``. Both are required for two runs on the same inputs to produce
    an identical archive.
    """
    buf = io.BytesIO()
    with gzip.GzipFile(fileobj=buf, mode="wb", compresslevel=compresslevel, mtime=0) as gz:
        gz.write(data)
    return buf.getvalue()


def build_cpio(root_dir):
    """Return the raw (uncompressed) SVR4 newc cpio archive for `root_dir`.

    All metadata is pinned (uid/gid 0, fixed mtime) and traversal order is
    sorted, so the output depends only on the file tree contents.
    """
    bio = io.BytesIO()
    writer = CpioWriter(bio)

    # Add initial essential device nodes
    writer.add_entry("dev", 0o040755, b"")
    writer.add_entry("dev/console", 0o020600, b"", rdevmajor=5, rdevminor=1)
    writer.add_entry("dev/null", 0o020666, b"", rdevmajor=1, rdevminor=3)
    writer.add_entry("dev/ttyS0", 0o020660, b"", rdevmajor=4, rdevminor=64)
    writer.add_entry("dev/tty", 0o020666, b"", rdevmajor=5, rdevminor=0)
    writer.add_entry("dev/tty0", 0o020660, b"", rdevmajor=4, rdevminor=0)
    writer.add_entry("dev/tty1", 0o020660, b"", rdevmajor=4, rdevminor=1)
    writer.add_entry("dev/kmsg", 0o020666, b"", rdevmajor=1, rdevminor=11)
    writer.add_entry("dev/fb0", 0o020660, b"", rdevmajor=29, rdevminor=0)

    # Collect files in deterministic order
    entries = []
    for root, dirs, files in os.walk(root_dir):
        dirs.sort()
        files.sort()
        rel_root = os.path.relpath(root, root_dir).replace("\\", "/")
        if rel_root != "." and rel_root != "dev":
            entries.append((rel_root, 0o040755, b""))
        for f in files:
            file_path = os.path.join(root, f)
            if os.path.islink(file_path):
                raise RuntimeError(f"Symlinks are not supported in initramfs rootfs: {file_path}")
            rel_file = (f if rel_root == "." else f"{rel_root}/{f}").replace("\\", "/")
            with open(file_path, "rb") as fp:
                data = fp.read()
            mode = 0o100755 if (rel_file == "init" or rel_file.startswith(("bin/", "sbin/", "usr/bin/"))) else 0o100644
            entries.append((rel_file, mode, data))

    for rel_path, mode, content in entries:
        writer.add_entry(rel_path, mode, content)

    writer.close()
    return bio.getvalue()


def serialize_initramfs(root_dir):
    """Return the complete gzip-compressed initramfs bytes for `root_dir`.

    Pure function: no I/O outside reading `root_dir`, so callers (including the
    reproducibility check and its unit tests) can build the image in memory.
    """
    return gzip_compress(build_cpio(root_dir), compresslevel=6)


def create_initramfs(root_dir, output_gz):
    print(f"==> Packaging rootfs ({root_dir}) into initramfs ({output_gz})...")
    raw_data = build_cpio(root_dir)
    gz_data = gzip_compress(raw_data, compresslevel=6)
    os.makedirs(os.path.dirname(os.path.abspath(output_gz)), exist_ok=True)
    with open(output_gz, "wb") as fp:
        fp.write(gz_data)
    print(f"[OK] Initramfs created: {output_gz} ({len(gz_data)} bytes, uncompressed {len(raw_data)} bytes)")


def check_reproducible(root_dir):
    """Build the initramfs twice from `root_dir` and compare SHA-256 digests.

    Returns ``(ok, first_hash, second_hash)``. Only in-memory work is done, so
    this needs neither QEMU nor the network.
    """
    first = gzip_compress(build_cpio(root_dir), compresslevel=6)
    second = gzip_compress(build_cpio(root_dir), compresslevel=6)
    first_hash = hashlib.sha256(first).hexdigest()
    second_hash = hashlib.sha256(second).hexdigest()
    return (first_hash == second_hash, first_hash, second_hash)


def prepare_rootfs():
    os.makedirs(ROOTFS, exist_ok=True)
    dirs = [
        "bin", "sbin", "usr/bin", "usr/lib", "etc/nilos",
        "proc", "sys", "dev", "run/nilos", "tmp", "mnt", "data"
    ]
    for d in dirs:
        os.makedirs(os.path.join(ROOTFS, d), exist_ok=True)

    # Copy etc/nilos configs
    etc_src = os.path.join(TOP, "etc", "nilos")
    if os.path.exists(etc_src):
        shutil.copytree(etc_src, os.path.join(ROOTFS, "etc", "nilos"), dirs_exist_ok=True)

    # Check compiled release binaries
    release_dir = os.path.join(TOP, "target", "x86_64-unknown-linux-musl", "release")
    fallback_release = os.path.join(TOP, "target", "release")

    bins = [
        "nilinit", "nild", "nilkeyd", "nilbus", "nilshell",
        "inputd", "netd", "audiod", "powerd", "notifyd", "nilpkg",
        "settings", "oobe", "hello", "launcher", "nilimed", "nilttsd",
        "logd", "clipd", "btd", "vpnd", "thermald", "alarmd",
        "userd", "crashd", "nilandroidd", "nilinstall", "nilup", "nilperf",
        "nilc", "nilrt-launch", "nilrt"
    ]
    for b in bins:
        target_path = os.path.join(ROOTFS, "usr", "bin", b)
        src_musl = os.path.join(release_dir, b)
        src_fb = os.path.join(fallback_release, b)

        if os.path.exists(src_musl):
            shutil.copy2(src_musl, target_path)
            shutil.copy2(src_musl, os.path.join(ROOTFS, "bin", b))
            print(f"[+] Installed musl binary: {b}")
        elif os.path.exists(src_fb):
            shutil.copy2(src_fb, target_path)
            shutil.copy2(src_fb, os.path.join(ROOTFS, "bin", b))
            print(f"[+] Installed native binary: {b}")

    if not os.path.isfile(os.path.join(ROOTFS, "usr", "bin", "nilinit")):
        raise RuntimeError("nilinit binary is missing; build nilinit before creating a bootable initramfs")

    # If nilinit was installed, link or copy to /init and /sbin/init
    nilinit_bin = os.path.join(ROOTFS, "usr", "bin", "nilinit")
    if os.path.exists(nilinit_bin):
        shutil.copy2(nilinit_bin, os.path.join(ROOTFS, "init"))
        shutil.copy2(nilinit_bin, os.path.join(ROOTFS, "bin", "nilinit"))
        shutil.copy2(nilinit_bin, os.path.join(ROOTFS, "sbin", "init"))
        print("[+] /init and /sbin/init linked to nilinit")


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("--skip-kernel-download", action="store_true",
                        help="fail instead of downloading a missing kernel")
    parser.add_argument("--check-reproducible", action="store_true",
                        help="build the initramfs twice and verify identical SHA-256")
    parser.add_argument("--rootfs", default=None,
                        help="rootfs directory to package (default: the generated one); "
                             "with --check-reproducible this can be any pre-populated tree")
    args = parser.parse_args(argv)
    print("=========================================================")
    print("          NilOS Initramfs & Image Builder                ")
    print("=========================================================")

    if args.check_reproducible:
        root_dir = args.rootfs or ROOTFS
        if args.rootfs is None:
            ensure_kernel(skip_download=args.skip_kernel_download)
            prepare_rootfs()
        if not os.path.isdir(root_dir):
            raise RuntimeError(f"rootfs directory not found: {root_dir}")
        ok, first_hash, second_hash = check_reproducible(root_dir)
        print(f"    build #1 sha256: {first_hash}")
        print(f"    build #2 sha256: {second_hash}")
        if not ok:
            raise RuntimeError("initramfs is NOT reproducible (digests differ)")
        print("[OK] Initramfs is byte-for-byte reproducible.")
        return 0

    ensure_kernel(skip_download=args.skip_kernel_download)
    prepare_rootfs()
    create_initramfs(ROOTFS, INITRD_PATH)
    return 0


if __name__ == "__main__":
    try:
        sys.exit(main())
    except Exception as exc:
        print(f"[ERROR] Initramfs build failed: {exc}", file=sys.stderr)
        sys.exit(1)
