#!/usr/bin/env python3
"""F1 regression tests: deterministic initramfs and disk image builds.

These exercise the determinism helpers on small temporary trees only; they need
neither QEMU nor the network.
"""
import gzip
import hashlib
import importlib.util
import os
import struct
import tempfile
import unittest

HERE = os.path.dirname(os.path.abspath(__file__))


def _load(name, filename):
    spec = importlib.util.spec_from_file_location(name, os.path.join(HERE, filename))
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


mkinitramfs = _load("mkinitramfs", "mkinitramfs.py")
mkdisk = _load("mkdisk", "mkdisk.py")


def make_root(root):
    """Create a tiny representative rootfs tree."""
    os.makedirs(os.path.join(root, "usr", "bin"))
    os.makedirs(os.path.join(root, "etc", "nilos"))
    with open(os.path.join(root, "init"), "wb") as f:
        f.write(b"#!/bin/sh\necho booting\n")
    with open(os.path.join(root, "usr", "bin", "nilinit"), "wb") as f:
        f.write(b"\x7fELF fake binary")
    with open(os.path.join(root, "etc", "nilos", "services.toml"), "wb") as f:
        f.write(b"[[services]]\nname = 'x'\n")
    return root


class InitramfsReproducibilityTests(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.root = make_root(os.path.join(self._tmp.name, "root"))

    def tearDown(self):
        self._tmp.cleanup()

    def test_serialize_is_byte_identical_across_calls(self):
        first = mkinitramfs.serialize_initramfs(self.root)
        second = mkinitramfs.serialize_initramfs(self.root)
        self.assertEqual(first, second)
        self.assertEqual(
            hashlib.sha256(first).hexdigest(),
            hashlib.sha256(second).hexdigest(),
        )

    def test_check_reproducible_reports_ok(self):
        ok, first, second = mkinitramfs.check_reproducible(self.root)
        self.assertTrue(ok)
        self.assertEqual(first, second)

    def test_different_content_changes_digest(self):
        first = mkinitramfs.serialize_initramfs(self.root)
        with open(os.path.join(self.root, "usr", "bin", "nilinit"), "wb") as f:
            f.write(b"\x7fELF different")
        second = mkinitramfs.serialize_initramfs(self.root)
        self.assertNotEqual(
            hashlib.sha256(first).hexdigest(),
            hashlib.sha256(second).hexdigest(),
        )

    def test_gzip_header_has_zero_mtime_and_no_filename(self):
        blob = mkinitramfs.gzip_compress(b"payload")
        self.assertEqual(blob[0:2], b"\x1f\x8b")
        self.assertEqual(blob[2], 8, "deflate compression method expected")
        flags = blob[3]
        self.assertEqual(flags & 0x08, 0, "FNAME flag must not be set")
        mtime = struct.unpack("<I", blob[4:8])[0]
        self.assertEqual(mtime, 0, "gzip MTIME must be pinned to zero")
        # Round-trips.
        self.assertEqual(gzip.decompress(blob), b"payload")

    def test_create_initramfs_writes_identical_files_twice(self):
        out_a = os.path.join(self._tmp.name, "a", "initrd.gz")
        out_b = os.path.join(self._tmp.name, "b", "initrd.gz")
        mkinitramfs.create_initramfs(self.root, out_a)
        mkinitramfs.create_initramfs(self.root, out_b)
        with open(out_a, "rb") as f:
            a = f.read()
        with open(out_b, "rb") as f:
            b = f.read()
        self.assertEqual(a, b)


class DiskImageReproducibilityTests(unittest.TestCase):
    def test_synthetic_disk_image_is_deterministic(self):
        with tempfile.TemporaryDirectory() as tmp:
            first = os.path.join(tmp, "first.img")
            second = os.path.join(tmp, "second.img")
            # Small size keeps the test fast; determinism is size-independent.
            mkdisk.build_synthetic_image(first, size_mb=4)
            mkdisk.build_synthetic_image(second, size_mb=4)
            with open(first, "rb") as f:
                h1 = hashlib.sha256(f.read()).hexdigest()
            with open(second, "rb") as f:
                h2 = hashlib.sha256(f.read()).hexdigest()
            self.assertEqual(h1, h2)
            self.assertEqual(os.path.getsize(first), 4 * 1024 * 1024)

    def test_synthetic_superblock_magic_and_label(self):
        with tempfile.TemporaryDirectory() as tmp:
            image = os.path.join(tmp, "img")
            mkdisk.build_synthetic_image(image, size_mb=4)
            with open(image, "rb") as f:
                f.seek(1024)
                sb = f.read(2048)
            self.assertEqual(struct.unpack("<H", sb[56:58])[0], mkdisk.EXT2_MAGIC)
            self.assertEqual(sb[120:136].rstrip(b"\x00"), b"nilos-data")


if __name__ == "__main__":
    unittest.main()