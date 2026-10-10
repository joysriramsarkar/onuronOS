#!/usr/bin/env python3
"""F1 — Reproducible image gate.

Builds the artifacts that *can* be built deterministically twice from the same
inputs in-process and asserts the SHA-256 digests match:

  * the initramfs (gzip-compressed cpio) via ``mkinitramfs.serialize_initramfs``;
  * the data-partition disk image via ``mkdisk.build_synthetic_image``.

The initramfs check needs a rootfs directory (``--rootfs``, defaulting to the
generated ``out/x86_64-generic/rootfs``); it needs neither QEMU nor the network.

Deliberately NOT covered here (documented in docs/reproducibility.md):
  * the Linux kernel download — the URL is unpinned and fetched over the network;
  * a *real* ext4 ``nilos.img`` produced by ``mke2fs`` — mkfs embeds a random
    hash seed and current timestamps, so those images are not yet byte-stable.

Usage:
    python build/check-reproducible.py [--rootfs DIR] [--skip-disk]
                                       [--disk-size-mb N]
"""
import argparse
import hashlib
import importlib.util
import os
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, filename):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, filename))
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module
    spec.loader.exec_module(module)
    return module


mkinitramfs = _load("mkinitramfs", "mkinitramfs.py")
mkdisk = _load("mkdisk", "mkdisk.py")


def sha256_bytes(data):
    return hashlib.sha256(data).hexdigest()


def sha256_file(path):
    digest = hashlib.sha256()
    with open(path, "rb") as f:
        for chunk in iter(lambda: f.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def check_initramfs(root_dir):
    """Return (ok, first_hash, second_hash) for two in-memory builds."""
    return mkinitramfs.check_reproducible(root_dir)


def check_synthetic_disk(size_mb):
    """Return (ok, first_hash, second_hash) for two synthetic disk images."""
    with tempfile.TemporaryDirectory() as tmp:
        first = os.path.join(tmp, "first.img")
        second = os.path.join(tmp, "second.img")
        mkdisk.build_synthetic_image(first, size_mb=size_mb)
        mkdisk.build_synthetic_image(second, size_mb=size_mb)
        first_hash = sha256_file(first)
        second_hash = sha256_file(second)
    return (first_hash == second_hash, first_hash, second_hash)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--rootfs", default=None,
                        help="rootfs tree to package (default: out/qemu-x86_64/rootfs)")
    parser.add_argument("--skip-disk", action="store_true",
                        help="only check the initramfs, not the disk image")
    parser.add_argument("--disk-size-mb", type=int, default=mkdisk.DISK_SIZE_MB,
                        help="size of the synthetic disk image used for the check")
    args = parser.parse_args(argv)

    root_dir = args.rootfs or mkinitramfs.ROOTFS
    if not os.path.isdir(root_dir):
        print(f"[FAIL] rootfs directory not found: {root_dir}", file=sys.stderr)
        print("       Build it first (python build/mkinitramfs.py) or pass --rootfs.",
              file=sys.stderr)
        return 1

    failures = []

    print(f"==> Checking initramfs reproducibility from {root_dir}")
    ok, first, second = check_initramfs(root_dir)
    print(f"    build #1 sha256: {first}")
    print(f"    build #2 sha256: {second}")
    if ok:
        print("[OK] initramfs is byte-for-byte reproducible")
    else:
        print("[FAIL] initramfs differs between builds", file=sys.stderr)
        failures.append("initramfs")

    if not args.skip_disk:
        print(f"==> Checking synthetic disk image reproducibility ({args.disk_size_mb} MB)")
        ok, first, second = check_synthetic_disk(args.disk_size_mb)
        print(f"    build #1 sha256: {first}")
        print(f"    build #2 sha256: {second}")
        if ok:
            print("[OK] synthetic nilos.img is byte-for-byte reproducible")
        else:
            print("[FAIL] synthetic disk image differs between builds", file=sys.stderr)
            failures.append("disk image")

    if failures:
        print(f"[FAIL] not reproducible: {', '.join(failures)}", file=sys.stderr)
        return 1
    print("[OK] All checked artifacts are reproducible.")
    return 0


if __name__ == "__main__":
    sys.exit(main())