#!/usr/bin/env python3
"""
build/mkinitramfs.py — NilOS Standalone Initramfs & Image Generator
Creates a Linux-compatible SVR4 (070701) cpio.gz initramfs for QEMU x86_64 boot.
Runs completely cross-platform without requiring external 'cpio' or 'mknod'.
"""

import os
import sys
import gzip
import shutil
import urllib.request

TOP = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
OUT = os.path.join(TOP, "out", "x86_64-generic")
ROOTFS = os.path.join(OUT, "rootfs")
KERNEL_PATH = os.path.join(OUT, "vmlinuz-lts")
INITRD_PATH = os.path.join(OUT, "nilos-initramfs.cpio.gz")

KERNEL_URL = "https://dl-cdn.alpinelinux.org/alpine/v3.19/releases/x86_64/netboot/vmlinuz-lts"


def ensure_kernel():
    os.makedirs(OUT, exist_ok=True)
    if os.path.exists(KERNEL_PATH) and os.path.getsize(KERNEL_PATH) > 1000000:
        print(f"[OK] Kernel present: {KERNEL_PATH} ({os.path.getsize(KERNEL_PATH)} bytes)")
        return
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


def create_initramfs(root_dir, output_gz):
    print(f"==> Packaging rootfs ({root_dir}) into initramfs ({output_gz})...")
    import io
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
            rel_file = (f if rel_root == "." else f"{rel_root}/{f}").replace("\\", "/")
            with open(file_path, "rb") as fp:
                data = fp.read()
            mode = 0o100755 if ("bin" in rel_file or rel_file == "init") else 0o100644
            entries.append((rel_file, mode, data))

    for rel_path, mode, content in entries:
        writer.add_entry(rel_path, mode, content)

    writer.close()
    raw_data = bio.getvalue()
    gz_data = gzip.compress(raw_data, compresslevel=6)
    with open(output_gz, "wb") as fp:
        fp.write(gz_data)
    print(f"[OK] Initramfs created: {output_gz} ({len(gz_data)} bytes, uncompressed {len(raw_data)} bytes)")


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
        "userd", "crashd", "nilandroidd", "nilinstall", "nilup", "nilperf"
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

    # If nilinit was installed, link or copy to /init and /sbin/init
    nilinit_bin = os.path.join(ROOTFS, "usr", "bin", "nilinit")
    if os.path.exists(nilinit_bin):
        shutil.copy2(nilinit_bin, os.path.join(ROOTFS, "init"))
        shutil.copy2(nilinit_bin, os.path.join(ROOTFS, "bin", "nilinit"))
        shutil.copy2(nilinit_bin, os.path.join(ROOTFS, "sbin", "init"))
        print("[+] /init and /sbin/init linked to nilinit")


def main():
    print("=========================================================")
    print("          NilOS Initramfs & Image Builder                ")
    print("=========================================================")
    ensure_kernel()
    prepare_rootfs()
    create_initramfs(ROOTFS, INITRD_PATH)


if __name__ == "__main__":
    main()
